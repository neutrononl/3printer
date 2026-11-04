use sha2::{Digest, Sha256};
use std::io::Read;
use std::{
	path::{Path, PathBuf},
	process::{Child, Command},
	sync::mpsc::{self, Receiver},
	thread,
};

const REPOSITORY: &str = "rastislavcore/3printer";
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug)]
pub struct ReleaseInfo {
	pub version: String,
	pub asset_name: String,
	pub asset_url: String,
	pub sha256: String,
	pub size: u64,
}

pub enum UpdateEvent {
	Available,
	UpToDate,
	CheckFailed,
	Downloaded,
	DownloadFailed,
}

pub struct UpdateState {
	pub auto_check: bool,
	pub checking: bool,
	pub downloading: bool,
	pub paused: bool,
	pub downloaded_bytes: u64,
	pub total_bytes: u64,
	pub latest: Option<ReleaseInfo>,
	pub downloaded: Option<PathBuf>,
	pub error: Option<String>,
	pub checked: bool,
	check_rx: Option<Receiver<Result<Option<ReleaseInfo>, String>>>,
	download_child: Option<Child>,
	download_path: Option<PathBuf>,
}

impl UpdateState {
	pub fn new(auto_check: bool) -> Self {
		Self {
			auto_check,
			checking: false,
			downloading: false,
			paused: false,
			downloaded_bytes: 0,
			total_bytes: 0,
			latest: None,
			downloaded: None,
			error: None,
			checked: false,
			check_rx: None,
			download_child: None,
			download_path: None,
		}
	}

	pub fn start_check(&mut self) {
		if self.checking || self.downloading || self.paused || self.downloaded.is_some() {
			return;
		}
		self.checking = true;
		self.checked = false;
		self.error = None;
		let (tx, rx) = mpsc::channel();
		self.check_rx = Some(rx);
		thread::spawn(move || {
			let _ = tx.send(check_latest());
		});
	}

	pub fn start_download(&mut self) {
		let Some(release) = self.latest.clone() else {
			return;
		};
		if self.downloading || self.downloaded.is_some() {
			return;
		}
		self.error = None;
		let path = self.download_path.clone().unwrap_or_else(|| {
			std::env::temp_dir().join(format!(
				"3printer-update-{}-{}",
				std::process::id(),
				release.asset_name
			))
		});
		let existing = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
		if existing > release.size {
			let _ = std::fs::remove_file(&path);
		}
		let mut command = Command::new("curl");
		command
			.args([
				"-fL",
				"--silent",
				"--show-error",
				"--retry",
				"2",
				"-A",
				"3printer-updater",
				"-o",
			])
			.arg(&path);
		if existing > 0 && existing < release.size {
			command.args(["-C", "-"]);
		}
		command
			.arg(&release.asset_url)
			.stderr(std::process::Stdio::null());
		match command.spawn() {
			Ok(child) => {
				self.download_child = Some(child);
				self.download_path = Some(path);
				self.total_bytes = release.size;
				self.downloading = true;
				self.paused = false;
			}
			Err(error) => self.error = Some(format!("Could not start download: {error}")),
		}
	}

	pub fn pause_download(&mut self) {
		if let Some(mut child) = self.download_child.take() {
			let _ = child.kill();
			let _ = child.wait();
			self.downloading = false;
			self.paused = true;
			self.refresh_progress();
		}
	}

	fn refresh_progress(&mut self) {
		if let Some(path) = &self.download_path {
			self.downloaded_bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
		}
	}

	pub fn poll(&mut self) -> Option<UpdateEvent> {
		if let Some(rx) = &self.check_rx {
			match rx.try_recv() {
				Ok(Ok(Some(release))) => {
					self.check_rx = None;
					self.checking = false;
					self.checked = true;
					self.latest = Some(release);
					return Some(UpdateEvent::Available);
				}
				Ok(Ok(None)) => {
					self.check_rx = None;
					self.checking = false;
					self.checked = true;
					self.latest = None;
					return Some(UpdateEvent::UpToDate);
				}
				Ok(Err(error)) => {
					self.check_rx = None;
					self.checking = false;
					self.checked = true;
					self.error = Some(error);
					return Some(UpdateEvent::CheckFailed);
				}
				Err(mpsc::TryRecvError::Disconnected) => {
					self.check_rx = None;
					self.checking = false;
					self.checked = true;
					self.error = Some("Update check stopped unexpectedly".into());
					return Some(UpdateEvent::CheckFailed);
				}
				Err(mpsc::TryRecvError::Empty) => {}
			}
		}
		self.refresh_progress();
		if let Some(child) = &mut self.download_child {
			match child.try_wait() {
				Ok(Some(status)) => {
					self.download_child = None;
					self.downloading = false;
					let path = self.download_path.as_ref().unwrap();
					let result = if status.success() {
						verify_download(path, self.latest.as_ref().unwrap())
					} else {
						Err(format!(
							"Download failed (curl exit code {}). Try again.",
							status.code().unwrap_or(-1)
						))
					};
					return Some(match result {
						Ok(()) => {
							self.downloaded = Some(path.clone());
							UpdateEvent::Downloaded
						}
						Err(error) => {
							self.error = Some(error);
							UpdateEvent::DownloadFailed
						}
					});
				}
				Ok(None) => {}
				Err(error) => {
					self.download_child = None;
					self.downloading = false;
					self.error = Some(format!("Could not monitor download: {error}"));
					return Some(UpdateEvent::DownloadFailed);
				}
			}
		}
		None
	}

	pub fn install_and_restart(&mut self) -> Result<(), String> {
		let archive = self
			.downloaded
			.as_ref()
			.ok_or("The update has not finished downloading")?;
		install_and_restart(archive)
	}
}

pub fn load_auto_check() -> bool {
	std::fs::read(prefs_path())
		.ok()
		.and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
		.and_then(|value| value.get("auto_check").and_then(serde_json::Value::as_bool))
		.unwrap_or(true)
}

pub fn save_auto_check(value: bool) {
	let path = prefs_path();
	if let Some(parent) = path.parent() {
		let _ = std::fs::create_dir_all(parent);
	}
	let _ = std::fs::write(path, serde_json::json!({"auto_check": value}).to_string());
}

fn prefs_path() -> PathBuf {
	let base = std::env::var_os("HOME")
		.map(PathBuf::from)
		.or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
		.unwrap_or_else(std::env::temp_dir);
	base.join(".3printer/update.json")
}

fn check_latest() -> Result<Option<ReleaseInfo>, String> {
	let output = Command::new("curl")
		.args([
			"-fsSL",
			"--connect-timeout",
			"10",
			"--max-time",
			"30",
			"-A",
			"3printer-updater",
			&format!("https://api.github.com/repos/{REPOSITORY}/releases/latest"),
		])
		.output()
		.map_err(|error| format!("Could not start update check: {error}"))?;
	if !output.status.success() {
		if String::from_utf8_lossy(&output.stderr).contains("404") {
			return Err("No published release is available yet.".into());
		}
		return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
	}
	let value: serde_json::Value = serde_json::from_slice(&output.stdout)
		.map_err(|error| format!("Invalid GitHub release response: {error}"))?;
	let tag = value
		.get("tag_name")
		.and_then(serde_json::Value::as_str)
		.ok_or("GitHub release has no tag")?;
	let version = tag.trim_start_matches('v');
	if compare_versions(version, CURRENT_VERSION) != std::cmp::Ordering::Greater {
		return Ok(None);
	}
	let suffix = platform_asset_suffix();
	let assets = value
		.get("assets")
		.and_then(serde_json::Value::as_array)
		.ok_or("GitHub release has no assets")?;
	let asset = assets
		.iter()
		.find(|asset| {
			asset
				.get("name")
				.and_then(serde_json::Value::as_str)
				.is_some_and(|name| name.ends_with(suffix))
		})
		.ok_or_else(|| format!("No {suffix} download is available for this system"))?;
	Ok(Some(ReleaseInfo {
		version: version.into(),
		asset_name: asset
			.get("name")
			.and_then(serde_json::Value::as_str)
			.ok_or("GitHub asset has no name")?
			.into(),
		asset_url: asset
			.get("browser_download_url")
			.and_then(serde_json::Value::as_str)
			.ok_or("GitHub asset has no download URL")?
			.into(),
		sha256: parse_github_digest(asset.get("digest").and_then(serde_json::Value::as_str))?,
		size: asset
			.get("size")
			.and_then(serde_json::Value::as_u64)
			.ok_or("GitHub asset has no size")?,
	}))
}

fn parse_github_digest(digest: Option<&str>) -> Result<String, String> {
	let hex = digest
		.and_then(|value| value.strip_prefix("sha256:"))
		.ok_or("GitHub release asset has no SHA-256 digest")?;
	if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
		return Err("GitHub release asset has an invalid SHA-256 digest".into());
	}
	Ok(hex.to_ascii_lowercase())
}

fn verify_download(path: &Path, release: &ReleaseInfo) -> Result<(), String> {
	let mut file = std::fs::File::open(path).map_err(|error| error.to_string())?;
	let size = file.metadata().map_err(|error| error.to_string())?.len();
	if size != release.size {
		return Err(format!("Incomplete download: {size} of {} bytes", release.size));
	}
	let mut hasher = Sha256::new();
	let mut buffer = [0u8; 64 * 1024];
	loop {
		let count = file.read(&mut buffer).map_err(|error| error.to_string())?;
		if count == 0 {
			break;
		}
		hasher.update(&buffer[..count]);
	}
	let actual = format!("{:x}", hasher.finalize());
	if actual != release.sha256 {
		let _ = std::fs::remove_file(path);
		return Err("SHA-256 verification failed. The downloaded file was discarded.".into());
	}
	Ok(())
}

fn platform_asset_suffix() -> &'static str {
	if cfg!(target_os = "macos") {
		"-macos.zip"
	} else if cfg!(target_os = "windows") {
		"-windows.zip"
	} else {
		"-linux.tar.gz"
	}
}

fn compare_versions(left: &str, right: &str) -> std::cmp::Ordering {
	let parse = |value: &str| {
		value
			.trim_start_matches('v')
			.split(|c: char| c == '.' || c == '-' || c == '+')
			.take(3)
			.map(|part| part.parse::<u64>().unwrap_or(0))
			.chain(std::iter::repeat(0))
			.take(3)
			.collect::<Vec<_>>()
	};
	parse(left).cmp(&parse(right))
}

fn install_and_restart(archive: &Path) -> Result<(), String> {
	let staging = std::env::temp_dir().join(format!("3printer-install-{}", std::process::id()));
	std::fs::create_dir_all(&staging).map_err(|error| error.to_string())?;
	if cfg!(target_os = "windows") {
		let quote = |path: &Path| format!("'{}'", path.to_string_lossy().replace('\'', "''"));
		let command = format!(
			"Expand-Archive -Force -LiteralPath {} -DestinationPath {}",
			quote(archive),
			quote(&staging)
		);
		let status = Command::new("powershell")
			.args(["-NoProfile", "-Command", &command])
			.status()
			.map_err(|error| format!("Could not unpack update: {error}"))?;
		if !status.success() {
			return Err("Could not unpack the Windows update".into());
		}
	} else if archive.extension().and_then(|ext| ext.to_str()) == Some("zip") {
		let status = Command::new("unzip")
			.args(["-q"])
			.arg(archive)
			.arg("-d")
			.arg(&staging)
			.status()
			.map_err(|error| format!("Could not unpack update: {error}"))?;
		if !status.success() {
			return Err("Could not unpack the macOS update".into());
		}
	} else {
		let status = Command::new("tar")
			.args(["-xzf"])
			.arg(archive)
			.arg("-C")
			.arg(&staging)
			.status()
			.map_err(|error| format!("Could not unpack update: {error}"))?;
		if !status.success() {
			return Err("Could not unpack the Linux update".into());
		}
	}
	let current = std::env::current_exe().map_err(|error| error.to_string())?;
	let replacement = find_replacement(&staging)?;
	let current = if cfg!(target_os = "macos") {
		current
			.ancestors()
			.find(|path| path.extension().is_some_and(|ext| ext == "app"))
			.ok_or("Run the packaged 3printer.app to install updates")?
			.to_path_buf()
	} else {
		current
	};
	if !replacement.exists() {
		return Err("Update archive does not contain the application".into());
	}
	spawn_restart_helper(&current, &replacement)?;
	Ok(())
}

fn find_replacement(staging: &Path) -> Result<PathBuf, String> {
	if cfg!(target_os = "macos") {
		return Ok(staging.join("3printer.app"));
	}
	let directory = std::fs::read_dir(staging)
		.map_err(|error| error.to_string())?
		.filter_map(Result::ok)
		.find(|entry| entry.path().is_dir())
		.map(|entry| entry.path())
		.ok_or("Update archive has no application directory")?;
	Ok(directory.join(if cfg!(target_os = "windows") {
		"3printer.exe"
	} else {
		"3printer"
	}))
}

fn spawn_restart_helper(current: &Path, replacement: &Path) -> Result<(), String> {
	if cfg!(target_os = "windows") {
		let quote = |path: &Path| format!("'{}'", path.to_string_lossy().replace('\'', "''"));
		let command = format!(
			"Start-Sleep -Seconds 1; Move-Item -Force -LiteralPath {} -Destination {}; Start-Process -FilePath {}",
			quote(replacement),
			quote(current),
			quote(current)
		);
		Command::new("powershell")
			.args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &command])
			.spawn()
			.map_err(|error| error.to_string())?;
	} else if cfg!(target_os = "macos") {
		let command =
			"sleep 1; mv \"$1\" \"$1.old\" && mv \"$2\" \"$1\" && open \"$1\"; sleep 2; rm -rf \"$1.old\"";
		Command::new("sh")
			.args(["-c", command, "3printer"])
			.arg(current)
			.arg(replacement)
			.spawn()
			.map_err(|error| error.to_string())?;
	} else {
		let command = "sleep 1; mv \"$1\" \"$1.old\" && mv \"$2\" \"$1\" && \"$1\"; rm -f \"$1.old\"";
		Command::new("sh")
			.args(["-c", command, "3printer"])
			.arg(current)
			.arg(replacement)
			.spawn()
			.map_err(|error| error.to_string())?;
	}
	Ok(())
}

pub fn platform_name() -> &'static str {
	if cfg!(target_os = "macos") {
		"macOS"
	} else if cfg!(target_os = "windows") {
		"Windows"
	} else if cfg!(target_os = "linux") {
		"Linux"
	} else {
		std::env::consts::OS
	}
}

#[cfg(test)]
mod tests {
	use super::{
		compare_versions, parse_github_digest, verify_download, ReleaseInfo, UpdateEvent, UpdateState,
	};
	use sha2::{Digest, Sha256};
	use std::{
		fs,
		time::{Duration, Instant},
	};

	#[test]
	fn versions_are_compared_numerically() {
		assert!(compare_versions("1.10.0", "1.9.0").is_gt());
		assert!(compare_versions("v1.2.0", "1.2.0").is_eq());
	}

	#[test]
	fn digest_is_required_and_download_is_verified() {
		let path = std::env::temp_dir().join(format!("3printer-hash-test-{}", std::process::id()));
		fs::write(&path, b"update bytes").unwrap();
		let digest = format!("{:x}", Sha256::digest(b"update bytes"));
		let release = ReleaseInfo {
			version: "2.0.0".into(),
			asset_name: "test.zip".into(),
			asset_url: String::new(),
			sha256: digest.clone(),
			size: 12,
		};
		assert_eq!(
			parse_github_digest(Some(&format!("sha256:{digest}"))).unwrap(),
			digest
		);
		assert!(parse_github_digest(None).is_err());
		assert!(verify_download(&path, &release).is_ok());
		let mut bad = release;
		bad.sha256 = "0".repeat(64);
		assert!(verify_download(&path, &bad).unwrap_err().contains("SHA-256"));
		assert!(!path.exists());
	}

	#[test]
	fn download_can_pause_resume_and_verify() {
		let source = std::env::temp_dir().join(format!("3printer-source-{}", std::process::id()));
		let contents = vec![42u8; 4 * 1024 * 1024];
		fs::write(&source, &contents).unwrap();
		let mut state = UpdateState::new(false);
		state.latest = Some(ReleaseInfo {
			version: "2.0.0".into(),
			asset_name: format!("simulation-{}.zip", std::process::id()),
			asset_url: format!("file://{}", source.display()),
			sha256: format!("{:x}", Sha256::digest(&contents)),
			size: contents.len() as u64,
		});
		state.start_download();
		assert!(state.downloading);
		state.pause_download();
		assert!(state.paused);
		state.start_download();
		let deadline = Instant::now() + Duration::from_secs(10);
		loop {
			if let Some(event) = state.poll() {
				assert!(
					matches!(event, UpdateEvent::Downloaded),
					"{}",
					state.error.unwrap_or_default()
				);
				break;
			}
			assert!(Instant::now() < deadline, "download simulation timed out");
			std::thread::sleep(Duration::from_millis(10));
		}
		assert_eq!(state.downloaded_bytes, contents.len() as u64);
		fs::remove_file(state.downloaded.unwrap()).unwrap();
		fs::remove_file(source).unwrap();
	}
}
