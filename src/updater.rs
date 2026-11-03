use std::{
	path::{Path, PathBuf},
	process::Command,
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
	pub latest: Option<ReleaseInfo>,
	pub downloaded: Option<PathBuf>,
	pub error: Option<String>,
	pub checked: bool,
	check_rx: Option<Receiver<Result<Option<ReleaseInfo>, String>>>,
	download_rx: Option<Receiver<Result<PathBuf, String>>>,
}

impl UpdateState {
	pub fn new(auto_check: bool) -> Self {
		Self {
			auto_check,
			checking: false,
			downloading: false,
			latest: None,
			downloaded: None,
			error: None,
			checked: false,
			check_rx: None,
			download_rx: None,
		}
	}

	pub fn start_check(&mut self) {
		if self.checking || self.downloading {
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
		if self.downloading {
			return;
		}
		self.downloading = true;
		self.error = None;
		let (tx, rx) = mpsc::channel();
		self.download_rx = Some(rx);
		thread::spawn(move || {
			let _ = tx.send(download_release(&release));
		});
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
		if let Some(rx) = &self.download_rx {
			match rx.try_recv() {
				Ok(Ok(path)) => {
					self.download_rx = None;
					self.downloading = false;
					self.downloaded = Some(path);
					return Some(UpdateEvent::Downloaded);
				}
				Ok(Err(error)) => {
					self.download_rx = None;
					self.downloading = false;
					self.error = Some(error);
					return Some(UpdateEvent::DownloadFailed);
				}
				Err(mpsc::TryRecvError::Disconnected) => {
					self.download_rx = None;
					self.downloading = false;
					self.error = Some("Update download stopped unexpectedly".into());
					return Some(UpdateEvent::DownloadFailed);
				}
				Err(mpsc::TryRecvError::Empty) => {}
			}
		}
		None
	}

	pub fn install_and_restart(&mut self) -> Result<(), String> {
		let archive = self
			.downloaded
			.take()
			.ok_or("The update has not finished downloading")?;
		install_and_restart(&archive)
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
	}))
}

fn download_release(release: &ReleaseInfo) -> Result<PathBuf, String> {
	let path = std::env::temp_dir().join(format!(
		"3printer-update-{}-{}",
		std::process::id(),
		release.asset_name
	));
	let output = Command::new("curl")
		.args(["-fL", "-A", "3printer-updater", "-o"])
		.arg(&path)
		.arg(&release.asset_url)
		.output()
		.map_err(|error| format!("Could not start update download: {error}"))?;
	if !output.status.success() {
		let _ = std::fs::remove_file(&path);
		return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
	}
	Ok(path)
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
		let status = Command::new("powershell")
			.args([
				"-NoProfile",
				"-Command",
				"Expand-Archive -Force -LiteralPath $args[0] -DestinationPath $args[1]",
			])
			.arg(archive)
			.arg(&staging)
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
	use super::compare_versions;

	#[test]
	fn versions_are_compared_numerically() {
		assert!(compare_versions("1.10.0", "1.9.0").is_gt());
		assert!(compare_versions("v1.2.0", "1.2.0").is_eq());
	}
}
