use std::{fs, path::Path};
pub type V3 = [f32; 3];
pub type Triangle = [V3; 3];
#[derive(Clone, PartialEq)]
pub struct Model {
	pub name: String,
	pub triangles: Vec<Triangle>,
	pub position: V3,
	pub rotation: V3,
	pub scale: f32,
}
impl Model {
	pub fn cube() -> Self {
		let v = [
			[-10., -10., 0.],
			[10., -10., 0.],
			[10., 10., 0.],
			[-10., 10., 0.],
			[-10., -10., 20.],
			[10., -10., 20.],
			[10., 10., 20.],
			[-10., 10., 20.],
		];
		let faces = [
			[0, 2, 1],
			[0, 3, 2],
			[4, 5, 6],
			[4, 6, 7],
			[0, 1, 5],
			[0, 5, 4],
			[1, 2, 6],
			[1, 6, 5],
			[2, 3, 7],
			[2, 7, 6],
			[3, 0, 4],
			[3, 4, 7],
		];
		Self {
			name: "Calibration cube".into(),
			triangles: faces.map(|f| f.map(|i| v[i])).to_vec(),
			position: [100., 100., 0.],
			rotation: [0.; 3],
			scale: 1.,
		}
	}
	pub fn load(path: &Path) -> Result<Self, String> {
		let bytes = fs::read(path).map_err(|e| e.to_string())?;
		let mut tris = Vec::new();
		let ext = path
			.extension()
			.and_then(|s| s.to_str())
			.unwrap_or("")
			.to_ascii_lowercase();
		if ["png", "jpg", "jpeg", "bmp"].contains(&ext.as_str()) {
			let image = image::load_from_memory(&bytes)
				.map_err(|e| e.to_string())?
				.resize(100, 100, image::imageops::FilterType::Triangle)
				.to_luma8();
			let (w, h) = image.dimensions();
			if w < 2 || h < 2 {
				return Err("Image is too small".into());
			}
			let point = |x: u32, y: u32| {
				[
					x as f32,
					y as f32,
					1. + image.get_pixel(x, y).0[0] as f32 / 255. * 4.,
				]
			};
			for y in 0..h - 1 {
				for x in 0..w - 1 {
					let a = point(x, y);
					let b = point(x + 1, y);
					let c = point(x + 1, y + 1);
					let d = point(x, y + 1);
					tris.extend([
						[a, b, c],
						[a, c, d],
						[[a[0], a[1], 0.], [c[0], c[1], 0.], [b[0], b[1], 0.]],
						[[a[0], a[1], 0.], [d[0], d[1], 0.], [c[0], c[1], 0.]],
					]);
				}
			}
			for edge in 0..4 {
				let count = if edge % 2 == 0 { w } else { h };
				for i in 0..count - 1 {
					let (a, b) = match edge {
						0 => (point(i, 0), point(i + 1, 0)),
						1 => (point(w - 1, i), point(w - 1, i + 1)),
						2 => (point(i + 1, h - 1), point(i, h - 1)),
						_ => (point(0, i + 1), point(0, i)),
					};
					let aa = [a[0], a[1], 0.];
					let bb = [b[0], b[1], 0.];
					tris.extend([[a, aa, bb], [a, bb, b]]);
				}
			}
		} else if ext == "off" {
			let text = String::from_utf8(bytes.clone()).map_err(|_| "Invalid OFF")?;
			let clean = text
				.lines()
				.map(|l| l.split('#').next().unwrap_or(""))
				.collect::<Vec<_>>()
				.join(" ");
			let mut tokens = clean.split_whitespace();
			if tokens.next() != Some("OFF") {
				return Err("Expected OFF header".into());
			}
			let values: Vec<&str> = tokens.collect();
			let mut cursor = 0;
			let mut number = || -> Result<f32, String> {
				let value = values
					.get(cursor)
					.ok_or("Incomplete OFF")?
					.parse::<f32>()
					.map_err(|_| "Invalid OFF number")?;
				cursor += 1;
				Ok(value)
			};
			let nv = number()? as usize;
			let nf = number()? as usize;
			number()?;
			if nv > 1_000_000 || nf > 1_000_000 {
				return Err("OFF too large".into());
			}
			let mut vertices = Vec::new();
			for _ in 0..nv {
				vertices.push([number()?, number()?, number()?]);
			}
			for _ in 0..nf {
				let n = number()? as usize;
				let mut ids = Vec::new();
				for _ in 0..n {
					let i = number()? as usize;
					if i >= nv {
						return Err("OFF index out of bounds".into());
					}
					ids.push(i);
				}
				for i in 1..n.saturating_sub(1) {
					tris.push([vertices[ids[0]], vertices[ids[i]], vertices[ids[i + 1]]]);
				}
			}
		} else if ["dae", "3ds", "ply", "scn"].contains(&ext.as_str()) {
			let script = std::env::temp_dir().join("3printer-import-scene.swift");
			fs::write(&script, include_str!("import_scene.swift")).map_err(|e| e.to_string())?;
			let output = std::process::Command::new("/usr/bin/swift")
				.arg(script)
				.arg(path)
				.env(
					"CLANG_MODULE_CACHE_PATH",
					std::env::temp_dir().join("3printer-swift-cache"),
				)
				.output()
				.map_err(|e| e.to_string())?;
			if !output.status.success() {
				return Err(String::from_utf8_lossy(&output.stderr).to_string());
			}
			tris = serde_json::from_slice(&output.stdout).map_err(|e| format!("Invalid mesh: {e}"))?;
		} else if !["stl", "obj", "3mf"].contains(&ext.as_str()) {
			return Err(format!(
				".{ext} requires a format-specific converter. Export as STL, OBJ, DAE, PLY or 3MF to import."
			));
		}
		if !tris.is_empty() {
			// Geometry already decoded by the native scene importer.
		} else if path
			.extension()
			.and_then(|s| s.to_str())
			.unwrap_or("")
			.eq_ignore_ascii_case("3mf")
		{
			let output = std::process::Command::new("/usr/bin/python3")
				.args(["-I", "-c", include_str!("import_3mf.py")])
				.arg(path)
				.output()
				.map_err(|e| format!("Cannot start 3MF importer: {e}"))?;
			if !output.status.success() {
				return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
			}
			tris =
				serde_json::from_slice(&output.stdout).map_err(|e| format!("Invalid 3MF geometry: {e}"))?;
		} else if path
			.extension()
			.and_then(|s| s.to_str())
			.unwrap_or("")
			.eq_ignore_ascii_case("obj")
		{
			let text = String::from_utf8(bytes).map_err(|_| "Invalid OBJ text")?;
			let mut verts = Vec::new();
			for line in text.lines() {
				let mut w = line.split_whitespace();
				match w.next() {
					Some("v") => {
						let n: Vec<f32> = w
							.take(3)
							.map(str::parse)
							.collect::<Result<_, _>>()
							.map_err(|_| "Invalid OBJ vertex")?;
						if n.len() != 3 {
							return Err("Incomplete OBJ vertex".into());
						}
						verts.push([n[0], n[1], n[2]]);
					}
					Some("f") => {
						let ids: Vec<usize> = w
							.map(|x| {
								let n = x
									.split('/')
									.next()
									.unwrap_or("")
									.parse::<isize>()
									.map_err(|_| "Invalid OBJ face")?;
								let i = if n < 0 { verts.len() as isize + n } else { n - 1 };
								if i < 0 || i as usize >= verts.len() {
									Err("OBJ index out of range")
								} else {
									Ok(i as usize)
								}
							})
							.collect::<Result<_, _>>()?;
						for i in 1..ids.len().saturating_sub(1) {
							tris.push([verts[ids[0]], verts[ids[i]], verts[ids[i + 1]]]);
						}
					}
					_ => {}
				}
			}
		} else {
			let count = if bytes.len() >= 84 {
				u32::from_le_bytes(bytes[80..84].try_into().unwrap()) as usize
			} else {
				0
			};
			if count > 0 && count.checked_mul(50).and_then(|n| n.checked_add(84)) == Some(bytes.len()) {
				for chunk in bytes[84..].chunks_exact(50) {
					let mut tri = [[0.; 3]; 3];
					for (i, v) in tri.iter_mut().enumerate() {
						for (j, c) in v.iter_mut().enumerate() {
							*c = f32::from_le_bytes(
								chunk[12 + i * 12 + j * 4..16 + i * 12 + j * 4]
									.try_into()
									.unwrap(),
							);
						}
					}
					tris.push(tri);
				}
			} else {
				let text = String::from_utf8(bytes).map_err(|_| "Unrecognized STL encoding")?;
				let mut vertices = Vec::new();
				for line in text.lines() {
					let mut w = line.split_whitespace();
					if w.next() == Some("vertex") {
						let n: Vec<f32> = w
							.take(3)
							.map(str::parse)
							.collect::<Result<_, _>>()
							.map_err(|_| "Invalid STL vertex")?;
						if n.len() != 3 {
							return Err("Incomplete STL vertex".into());
						}
						vertices.push([n[0], n[1], n[2]]);
					}
				}
				if vertices.len() % 3 != 0 {
					return Err("Incomplete STL triangle".into());
				}
				tris = vertices.chunks_exact(3).map(|v| [v[0], v[1], v[2]]).collect();
			}
		}
		if tris.is_empty() || tris.iter().flatten().flatten().any(|n| !n.is_finite()) {
			return Err("Model has no valid finite geometry".into());
		}
		if tris.len() > 1_000_000 {
			return Err("Model exceeds one million triangles".into());
		}
		let mut model = Self {
			name: path.file_name().unwrap_or_default().to_string_lossy().into(),
			triangles: tris,
			position: [0.; 3],
			rotation: [0.; 3],
			scale: 1.,
		};
		let (lo, hi) = model.bounds();
		for tri in &mut model.triangles {
			for v in tri {
				v[0] -= (lo[0] + hi[0]) / 2.;
				v[1] -= (lo[1] + hi[1]) / 2.;
				v[2] -= lo[2];
			}
		}
		model.position = [100., 100., 0.];
		Ok(model)
	}
	#[cfg(test)]
	pub fn vertex(&self, v: V3) -> V3 {
		let rotation = self.rotation.map(|angle| angle.to_radians().sin_cos());
		self.vertex_with_rotation(v, &rotation)
	}
	fn vertex_with_rotation(&self, v: V3, rotation: &[(f32, f32); 3]) -> V3 {
		let mut v = v.map(|x| x * self.scale);
		for axis in 0..3 {
			let (s, c) = rotation[axis];
			let a = (axis + 1) % 3;
			let b = (axis + 2) % 3;
			let x = v[a] * c - v[b] * s;
			v[b] = v[a] * s + v[b] * c;
			v[a] = x;
		}
		for (x, p) in v.iter_mut().zip(self.position) {
			*x += p;
		}
		v
	}
	pub fn transformed(&self) -> Vec<Triangle> {
		let rotation = self.rotation.map(|angle| angle.to_radians().sin_cos());
		self.triangles
			.iter()
			.map(|t| t.map(|v| self.vertex_with_rotation(v, &rotation)))
			.collect()
	}
	pub fn bounds(&self) -> (V3, V3) {
		let rotation = self.rotation.map(|angle| angle.to_radians().sin_cos());
		let mut lo = [f32::INFINITY; 3];
		let mut hi = [f32::NEG_INFINITY; 3];
		for tri in &self.triangles {
			for v in tri {
				let v = self.vertex_with_rotation(*v, &rotation);
				for a in 0..3 {
					lo[a] = lo[a].min(v[a]);
					hi[a] = hi[a].max(v[a]);
				}
			}
		}
		(lo, hi)
	}
	#[cfg(test)]
	pub fn find_free_position(&mut self, others: &[Model]) -> bool {
		self.find_free_position_for(others, &crate::printer::Printer::default())
	}
	pub fn find_free_position_for(&mut self, others: &[Model], printer: &crate::printer::Printer) -> bool {
		let size = printer.size;
		self.ground();
		let original = self.position;
		let (base_lo, base_hi) = self.bounds();
		let occupied: Vec<_> = others.iter().map(Model::bounds).collect();
		let mut candidates = vec![[size[0] / 2., size[1] / 2.]];
		for y in (5..size[1] as i32).step_by(3) {
			for x in (5..size[0] as i32).step_by(3) {
				candidates.push([x as f32, y as f32]);
			}
		}
		candidates.sort_by(|a, b| {
			((a[0] - size[0] / 2.).powi(2) + (a[1] - size[1] / 2.).powi(2))
				.total_cmp(&((b[0] - size[0] / 2.).powi(2) + (b[1] - size[1] / 2.).powi(2)))
		});
		for p in candidates {
			self.position[0] = p[0];
			self.position[1] = p[1];
			let lo = [
				base_lo[0] + p[0] - original[0],
				base_lo[1] + p[1] - original[1],
				base_lo[2],
			];
			let hi = [
				base_hi[0] + p[0] - original[0],
				base_hi[1] + p[1] - original[1],
				base_hi[2],
			];
			if lo[0] < 1. || lo[1] < 1. || hi[0] > size[0] - 1. || hi[1] > size[1] - 1. || hi[2] > size[2] {
				continue;
			}
			if printer.circular_bed
				&& [lo, [hi[0], lo[1], lo[2]], [lo[0], hi[1], lo[2]], hi]
					.iter()
					.any(|p| !printer.contains(*p))
			{
				continue;
			}
			if occupied.iter().all(|(a, b)| {
				hi[0] + 2. <= a[0] || lo[0] - 2. >= b[0] || hi[1] + 2. <= a[1] || lo[1] - 2. >= b[1]
			}) {
				return true;
			}
		}
		self.position = original;
		false
	}
	pub fn center(&self) -> V3 {
		let (lo, hi) = self.bounds();
		[(lo[0] + hi[0]) / 2., (lo[1] + hi[1]) / 2., (lo[2] + hi[2]) / 2.]
	}
	pub fn keep_center(&mut self, center: V3) {
		let after = self.center();
		for a in 0..3 {
			self.position[a] += center[a] - after[a];
		}
	}
	pub fn resize_axis(&mut self, axis: usize, factor: f32, uniform: bool) {
		if uniform {
			self.scale *= factor;
		} else {
			// Bake the current orientation, then resize around the object's center.
			let (lo, hi) = self.bounds();
			let center = [(lo[0] + hi[0]) / 2., (lo[1] + hi[1]) / 2., (lo[2] + hi[2]) / 2.];
			self.triangles = self
				.transformed()
				.into_iter()
				.map(|t| {
					t.map(|mut v| {
						for a in 0..3 {
							v[a] -= center[a];
						}
						v[axis] *= factor;
						v
					})
				})
				.collect();
			self.position = center;
			self.rotation = [0.; 3];
			self.scale = 1.;
		}
	}
	pub fn pillar(p: [f32; 2], height: f32, width: f32) -> Model {
		let mut m = Model::cube();
		m.name = "Support pillar".into();
		for t in &mut m.triangles {
			for v in t {
				v[0] *= width / 20.;
				v[1] *= width / 20.;
				v[2] *= height / 20.;
			}
		}
		m.position = [p[0], p[1], 0.];
		m
	}
	pub fn supports(&self, grid: f32, angle: f32) -> Vec<Model> {
		let (lo, hi) = self.bounds();
		let tris = self.transformed();
		let mut out = Vec::new();
		let mut x = lo[0] + grid / 2.;
		while x < hi[0] {
			let mut y = lo[1] + grid / 2.;
			while y < hi[1] {
				let mut hits = Vec::new();
				for t in &tris {
					let [a, b, c] = *t;
					let den = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1]);
					if den.abs() < 0.00001 {
						continue;
					}
					let u = ((b[1] - c[1]) * (x - c[0]) + (c[0] - b[0]) * (y - c[1])) / den;
					let v = ((c[1] - a[1]) * (x - c[0]) + (a[0] - c[0]) * (y - c[1])) / den;
					if u >= 0. && v >= 0. && u + v <= 1. {
						let z = u * a[2] + v * b[2] + (1. - u - v) * c[2];
						let nx = (b[1] - a[1]) * (c[2] - a[2]) - (b[2] - a[2]) * (c[1] - a[1]);
						let ny = (b[2] - a[2]) * (c[0] - a[0]) - (b[0] - a[0]) * (c[2] - a[2]);
						let nz = den;
						hits.push((z, nz.abs() / (nx * nx + ny * ny + nz * nz).sqrt()));
					}
				}
				hits.sort_by(|a, b| a.0.total_cmp(&b.0));
				if let Some((z, n)) = hits.first() {
					if *z > 0.4 && *n >= angle.to_radians().cos() {
						out.push(Model::pillar([x, y], z - 0.15, 1.2));
					}
				}
				y += grid;
			}
			x += grid;
		}
		out
	}
	pub fn ground(&mut self) {
		let z = self.bounds().0[2];
		self.position[2] -= z;
	}
}
#[derive(Clone, Default)]
pub struct Layer {
	pub z: f32,
	pub walls: Vec<[[f32; 2]; 2]>,
	pub infill: Vec<[[f32; 2]; 2]>,
}
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Settings {
	pub printer: crate::printer::Printer,
	pub infill_pattern: InfillPattern,
	pub wall_speed: f32,
	pub infill_speed: f32,
	pub wall_flow: f32,
	pub infill_flow: f32,
	pub small_length: f32,
	pub small_speed: f32,
	pub density: f32,
	pub height: f32,
	pub infill: f32,
	pub speed: f32,
	pub nozzle: f32,
	pub temperature: f32,
	pub bed: f32,
	pub diameter: f32,
	pub solid_layers: usize,
	pub travel_speed: f32,
	pub flow: f32,
	pub fan: f32,
	pub material: String,
}
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InfillPattern {
	Lines,
	ZigZag,
	Grid,
	Triangles,
	Gyroid,
	Cubic,
	Concentric,
	Lightning,
}
impl InfillPattern {
	pub const ALL: [Self; 8] = [
		Self::Lines,
		Self::ZigZag,
		Self::Grid,
		Self::Triangles,
		Self::Gyroid,
		Self::Cubic,
		Self::Concentric,
		Self::Lightning,
	];
	pub fn label(self) -> &'static str {
		match self {
			Self::Lines => "Lines",
			Self::ZigZag => "Zig Zag",
			Self::Grid => "Grid",
			Self::Triangles => "Triangles",
			Self::Gyroid => "Gyroid",
			Self::Cubic => "Cubic",
			Self::Concentric => "Concentric",
			Self::Lightning => "Lightning",
		}
	}
}
impl Default for InfillPattern {
	fn default() -> Self {
		Self::ZigZag
	}
}
impl Default for Settings {
	fn default() -> Self {
		Self {
			printer: crate::printer::Printer::default(),
			infill_pattern: InfillPattern::default(),
			wall_speed: 25.,
			infill_speed: 50.,
			wall_flow: 100.,
			infill_flow: 100.,
			small_length: 9.42,
			small_speed: 35.,
			density: 1.24,
			height: 0.15,
			infill: 20.,
			speed: 50.,
			nozzle: 0.4,
			temperature: 200.,
			bed: 50.,
			diameter: 1.75,
			solid_layers: 4,
			travel_speed: 100.,
			flow: 100.,
			fan: 100.,
			material: "PLA".into(),
		}
	}
}
impl Settings {
	pub fn validate(&self) -> Result<(), String> {
		self.printer.validate()?;
		for (name, value, min, max) in [
			("Wall speed", self.wall_speed, 5., 300.),
			("Infill speed", self.infill_speed, 5., 300.),
			("Wall flow", self.wall_flow, 0., 150.),
			("Infill flow", self.infill_flow, 0., 150.),
			("Small feature length", self.small_length, 0., 100.),
			("Small feature speed", self.small_speed, 1., 100.),
			("Material density", self.density, 0.1, 10.),
			("Layer height", self.height, 0.04, 0.8),
			("Infill", self.infill, 0., 100.),
			("Print speed", self.speed, 5., 300.),
			("Line width / nozzle", self.nozzle, 0.1, 2.),
			("Nozzle temperature", self.temperature, 150., 300.),
			("Bed temperature", self.bed, 0., 120.),
			("Filament diameter", self.diameter, 0.5, 4.),
			("Travel speed", self.travel_speed, 5., 300.),
			("Flow", self.flow, 50., 150.),
			("Fan", self.fan, 0., 100.),
		] {
			if !value.is_finite() || value < min || value > max {
				return Err(format!("{name} must be between {min} and {max}"));
			}
		}
		if self.height > self.nozzle {
			return Err("Layer height must not exceed nozzle width".into());
		}
		if self.solid_layers > 20 {
			return Err("Solid layers must be between 0 and 20".into());
		}
		Ok(())
	}
	pub fn from_json(bytes: &[u8]) -> Result<Self, String> {
		let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
		let data = value.get("settings").unwrap_or(&value);
		if !data.is_object() || data.get("height").is_none() {
			return Err("Not a 3printer settings file".into());
		}
		let settings: Self = serde_json::from_value(data.clone()).map_err(|e| e.to_string())?;
		settings.validate()?;
		Ok(settings)
	}
}
fn add_infill_direction(layer: &mut Layer, walls: &[[[f32; 2]; 2]], spacing: f32, angle: f32, nozzle: f32) {
	let (sin, cos) = angle.sin_cos();
	let transform = |point: [f32; 2]| [point[0] * cos + point[1] * sin, -point[0] * sin + point[1] * cos];
	let inverse = |point: [f32; 2]| [point[0] * cos - point[1] * sin, point[0] * sin + point[1] * cos];
	let transformed: Vec<_> = walls
		.iter()
		.flat_map(|wall| wall.iter().copied().map(transform))
		.collect();
	let Some(min_scan) = transformed.iter().map(|p| p[0]).min_by(f32::total_cmp) else {
		return;
	};
	let Some(max_scan) = transformed.iter().map(|p| p[0]).max_by(f32::total_cmp) else {
		return;
	};
	let mut scan = min_scan + spacing / 2.;
	while scan < max_scan {
		let mut positions = Vec::new();
		for wall in walls {
			let a = transform(wall[0]);
			let b = transform(wall[1]);
			if (a[0] <= scan && b[0] > scan) || (b[0] <= scan && a[0] > scan) {
				positions.push(a[1] + (scan - a[0]) / (b[0] - a[0]) * (b[1] - a[1]));
			}
		}
		positions.sort_by(f32::total_cmp);
		for pair in positions.chunks_exact(2) {
			let start = pair[0] + nozzle;
			let end = pair[1] - nozzle;
			if end > start {
				layer.infill.push([inverse([scan, start]), inverse([scan, end])]);
			}
		}
		scan += spacing;
	}
}
pub fn slice(models: &[Model], s: &Settings) -> Result<Vec<Layer>, String> {
	s.validate()?;
	if models.is_empty() {
		return Err("Import a model first".into());
	}
	if !(0.04..=0.8).contains(&s.height) {
		return Err("Choose a layer height between 0.04 and 0.8 mm".into());
	}
	for m in models {
		if m.transformed().iter().flatten().flatten().any(|v| !v.is_finite()) {
			return Err("Model contains invalid coordinates".into());
		}
		let (lo, hi) = m.bounds();
		if (0..3).any(|a| lo[a] < -0.001 || hi[a] > s.printer.size[a] + 0.001)
			|| m.transformed().iter().flatten().any(|p| !s.printer.contains(*p))
		{
			return Err(format!("{} is outside the selected printer build volume", m.name));
		}
	}
	let tris: Vec<_> = models.iter().flat_map(Model::transformed).collect();
	let max_z = tris.iter().flatten().map(|v| v[2]).fold(0., f32::max);
	let count = (max_z / s.height).ceil() as usize;
	if count == 0 {
		return Err("Model has no printable height".into());
	}
	if count > 5000 {
		return Err("This model needs more than 5,000 layers".into());
	}
	let mut layers = Vec::new();
	for n in 0..count {
		// Sample inside the actual final layer, including models thinner than one layer.
		let bottom = n as f32 * s.height;
		let top = ((n + 1) as f32 * s.height).min(max_z);
		let z = (bottom + top) * 0.5;
		let mut l = Layer {
			z: top,
			..Default::default()
		};
		for tri in &tris {
			let mut hits = Vec::new();
			for i in 0..3 {
				let a = tri[i];
				let b = tri[(i + 1) % 3];
				if (a[2] <= z && b[2] > z) || (b[2] <= z && a[2] > z) {
					let t = (z - a[2]) / (b[2] - a[2]);
					hits.push([a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])]);
				}
			}
			if hits.len() == 2 {
				l.walls.push([hits[0], hits[1]]);
			}
		}
		let density = if n < s.solid_layers || n + s.solid_layers >= count {
			100.
		} else {
			s.infill
		};
		if density > 0. {
			let (angles, density_scale) = match s.infill_pattern {
				InfillPattern::Lines if n % 2 == 0 => (vec![0.], 1.),
				InfillPattern::Lines => (vec![std::f32::consts::FRAC_PI_2], 1.),
				InfillPattern::ZigZag if n % 2 == 0 => (vec![0.], 1.),
				InfillPattern::ZigZag => (vec![std::f32::consts::FRAC_PI_2], 1.),
				InfillPattern::Grid => (vec![0., std::f32::consts::FRAC_PI_2], 2.),
				InfillPattern::Triangles => (
					vec![0., std::f32::consts::FRAC_PI_3, std::f32::consts::FRAC_PI_3 * 2.],
					3.,
				),
				InfillPattern::Gyroid => (
					vec![std::f32::consts::FRAC_PI_4, -std::f32::consts::FRAC_PI_4],
					2.,
				),
				InfillPattern::Cubic => (
					vec![0., std::f32::consts::FRAC_PI_3, std::f32::consts::FRAC_PI_3 * 2.],
					3.,
				),
				InfillPattern::Concentric => (vec![0., std::f32::consts::FRAC_PI_2], 2.),
				InfillPattern::Lightning => (
					vec![if n % 2 == 0 {
						0.
					} else {
						std::f32::consts::FRAC_PI_2
					}],
					1.,
				),
			};
			let effective_density = if s.infill_pattern == InfillPattern::Lightning {
				density.min(8.)
			} else {
				density
			};
			let spacing = s.nozzle * 100. * density_scale / effective_density;
			let walls = l.walls.clone();
			for angle in angles {
				add_infill_direction(&mut l, &walls, spacing, angle, s.nozzle);
			}
		}
		layers.push(l);
	}
	Ok(layers)
}
pub fn read_gcode(text: &str) -> Result<Vec<Layer>, String> {
	let mut layers: Vec<Layer> = Vec::new();
	let mut xyz = [0.; 3];
	let mut e = 0.;
	let mut absolute = true;
	let mut absolute_e = true;
	for line in text.lines() {
		let mut words = line.split(';').next().unwrap_or("").split_whitespace();
		let command = words.next().unwrap_or("");
		match command {
			"G90" => {
				absolute = true;
				continue;
			}
			"G91" => {
				absolute = false;
				continue;
			}
			"M82" => {
				absolute_e = true;
				continue;
			}
			"M83" => {
				absolute_e = false;
				continue;
			}
			_ => {}
		}
		if !["G0", "G00", "G1", "G01", "G92"].contains(&command) {
			continue;
		}
		let mut next = xyz;
		let mut next_e = e;
		for word in words {
			if word.len() < 2 {
				continue;
			}
			let Ok(n) = word[1..].parse::<f32>() else {
				continue;
			};
			if !n.is_finite() {
				return Err("Non-finite G-code coordinate".into());
			}
			match &word[..1] {
				"X" => next[0] = n + if absolute || command == "G92" { 0. } else { xyz[0] },
				"Y" => next[1] = n + if absolute || command == "G92" { 0. } else { xyz[1] },
				"Z" => next[2] = n + if absolute || command == "G92" { 0. } else { xyz[2] },
				"E" => next_e = n + if absolute_e || command == "G92" { 0. } else { e },
				_ => {}
			}
		}
		if command != "G92" && next_e > e && (next[0] != xyz[0] || next[1] != xyz[1]) {
			if layers.last().map_or(true, |l| (l.z - next[2]).abs() > 0.001) {
				layers.push(Layer {
					z: next[2],
					..Layer::default()
				});
			}
			layers
				.last_mut()
				.unwrap()
				.walls
				.push([[xyz[0], xyz[1]], [next[0], next[1]]]);
		}
		xyz = next;
		e = next_e;
	}
	if layers.is_empty() {
		return Err("No linear extrusion paths found in G-code".into());
	}
	Ok(layers)
}
// Shares extrusion and speed calculations with the G-code writer.
fn path_parameters(distance: f32, wall: bool, s: &Settings) -> (f32, f32) {
	let speed = s.speed.min(if wall { s.wall_speed } else { s.infill_speed });
	let speed = if distance <= s.small_length {
		speed * s.small_speed / 100.
	} else {
		speed
	};
	let volume = distance * s.nozzle * s.height * s.flow / 100.
		* if wall { s.wall_flow } else { s.infill_flow }
		/ 100.;
	(speed, volume)
}
#[derive(Clone, Copy, Default)]
pub struct PrintReport {
	pub seconds: f32,
	pub meters: f32,
	pub grams: f32,
}
pub fn print_report(layers: &[Layer], s: &Settings) -> PrintReport {
	let mut report = PrintReport::default();
	let mut position = [0., 0.];
	let mut z = 0.;
	let mut volume = 0.;
	for layer in layers {
		report.seconds += (layer.z - z).abs() / 10.;
		z = layer.z;
		for (wall, paths) in [(true, &layer.walls), (false, &layer.infill)] {
			for [a, b] in paths {
				let distance = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
				let (speed, amount) = path_parameters(distance, wall, s);
				report.seconds += ((a[0] - position[0]).powi(2) + (a[1] - position[1]).powi(2)).sqrt()
					/ s.travel_speed
					+ distance / speed;
				volume += amount;
				position = *b;
			}
		}
	}
	report.meters = volume / (std::f32::consts::PI * (s.diameter / 2.).powi(2)) / 1000.;
	report.grams = volume / 1000. * s.density;
	report
}
pub fn gcode(layers: &[Layer], s: &Settings) -> String {
	let mut code = String::from("; 3printer - Marlin G-code\n");
	if s.printer.generate_preview {
		code += &crate::printer::thumbnail(layers, &s.printer);
	}
	code += "G21\nG90\nM82\n";
	if s.printer.heated_bed {
		code += &format!("M140 S{:.0}\nM190 S{:.0}\n", s.bed, s.bed);
	}
	code += &format!("M104 S{:.0}\nM109 S{:.0}\n", s.temperature, s.temperature);
	code += s.printer.start.trim();
	code += "\nG21\nG90\nM82\n";
	code += &s
		.printer
		.extruder_start
		.replace("$$", &format!("{:.0}", s.temperature));
	// Restore the slicer's units and absolute coordinate/extrusion modes after custom code.
	code += "\nG21\nG90\nM82\nG92 E0\n";
	code += &format!("M106 S{:.0}\n", s.fan * 255. / 100.);
	let mut e = 0.;
	for (i, l) in layers.iter().enumerate() {
		code += &format!(";LAYER:{}\nG0 Z{:.3} F600\n", i, l.z);
		for (name, paths) in [("WALL-OUTER", &l.walls), ("FILL", &l.infill)] {
			code += &format!(";TYPE:{name}\n");
			for [a, b] in paths {
				let distance = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
				let (speed, volume) = path_parameters(distance, name == "WALL-OUTER", s);
				let a = s.printer.output_xy(*a);
				let b = s.printer.output_xy(*b);
				e += volume / (std::f32::consts::PI * (s.diameter / 2.).powi(2));
				code += &format!(
					"G0 X{:.3} Y{:.3} F{:.0}\nG1 X{:.3} Y{:.3} E{:.5} F{:.0}\n",
					a[0],
					a[1],
					s.travel_speed * 60.,
					b[0],
					b[1],
					e,
					speed * 60.
				);
			}
		}
	}
	code + "G21\nG90\nM82\n"
		+ &s.printer
			.extruder_end
			.trim()
			.replace("$$", &format!("{:.0}", s.temperature))
		+ "\nM83\n"
		+ s.printer.end.trim()
		+ "\n"
}
#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn printer_scripts_modes_and_dimensions() {
		let mut settings = Settings::default();
		let mut cube = Model::cube();
		cube.position = [205., 205., 230.];
		assert!(slice(&[cube.clone()], &settings).is_ok());
		settings.printer.size = [200.; 3];
		assert!(slice(&[cube], &settings).is_err());
		settings.printer = crate::printer::Printer::default();
		let code = gcode(&[], &settings);
		assert_eq!(code.matches("G28").count(), 1);
		assert!(code.find("M109 S200").unwrap() < code.find("G1 X10.1").unwrap());
		assert!(code.contains(&(settings.printer.start.clone() + "\nG21\nG90\nM82\n")));
		assert!(!code.contains("$$"));
		assert!(code.ends_with(&("M83\n".to_owned() + &settings.printer.end + "\n")));
		assert!(!code.ends_with("M84\n"));
		settings.printer.heated_bed = false;
		settings.printer.start = "; custom start".into();
		settings.printer.end = "; custom end".into();
		let code = gcode(&[], &settings);
		assert!(!code.contains("M190") && !code.contains("M140"));
		assert!(code.contains("; custom start\n") && code.ends_with("; custom end\n"));
		let bytes = serde_json::to_vec(&settings).unwrap();
		assert_eq!(Settings::from_json(&bytes).unwrap(), settings);
	}
	#[test]
	fn report_matches_extrusion_and_speed_changes() {
		let layers = vec![Layer {
			z: 0.15,
			walls: vec![[[0., 0.], [100., 0.]]],
			infill: vec![],
		}];
		let mut settings = Settings::default();
		let report = print_report(&layers, &settings);
		assert!((report.seconds - 4.015).abs() < 0.001);
		assert!((report.grams - 0.00744).abs() < 0.00001);
		let code = gcode(&layers, &settings);
		let extrusion: f32 = code
			.lines()
			.skip_while(|line| !line.starts_with(";LAYER:"))
			.find(|line| line.starts_with("G1 "))
			.unwrap()
			.split_whitespace()
			.find_map(|word| word.strip_prefix('E'))
			.unwrap()
			.parse()
			.unwrap();
		assert!((extrusion / 1000. - report.meters).abs() < 0.000001);
		settings.wall_flow = 50.;
		settings.wall_speed = 10.;
		let changed = print_report(&layers, &settings);
		assert!((changed.grams - report.grams / 2.).abs() < 0.00001);
		assert!(changed.seconds > report.seconds);
		assert!(print_report(&[], &settings).grams == 0.);
	}
	#[test]
	fn placement_avoids_transformed_objects_and_full_plate() {
		let a = Model::cube();
		let mut b = a.clone();
		b.rotation[2] = 45.;
		assert!(b.find_free_position(&[a.clone()]));
		let (lo, hi) = b.bounds();
		let (x, y) = a.bounds();
		assert!(hi[0] + 1.9 <= x[0] || lo[0] - 1.9 >= y[0] || hi[1] + 1.9 <= x[1] || lo[1] - 1.9 >= y[1]);
		let mut full = a.clone();
		full.scale = 10.;
		assert!(!b.find_free_position(&[full]));
	}
	#[test]
	fn support_pillars_reach_slicer_and_center_is_preserved() {
		let mut m = Model::cube();
		m.position[2] = 10.;
		let supports = m.supports(5., 60.);
		assert!(!supports.is_empty());
		let layers = slice(&supports, &Settings::default()).unwrap();
		assert!(!layers[0].walls.is_empty());
		let center = m.center();
		m.rotation[0] = 45.;
		m.scale = 2.;
		m.keep_center(center);
		for (a, b) in m.center().into_iter().zip(center) {
			assert!((a - b).abs() < 0.001);
		}
		m.ground();
		assert!(m.bounds().0[2].abs() < 0.001);
	}
	#[test]
	fn generated_code_can_be_simulated() {
		let settings = Settings::default();
		let layers = slice(&[Model::cube()], &settings).unwrap();
		let code = gcode(&layers, &settings);
		let parsed = read_gcode(code.split_once(";LAYER:0").unwrap().1).unwrap();
		assert_eq!(
			parsed.len(),
			layers
				.iter()
				.filter(|l| !l.walls.is_empty() || !l.infill.is_empty())
				.count()
		);
		assert_eq!(
			parsed[0].walls.len(),
			layers[0].walls.len() + layers[0].infill.len()
		);
	}
	#[test]
	fn settings_roundtrip_and_invalid_input() {
		let s = Settings {
			flow: 110.,
			travel_speed: 80.,
			fan: 25.,
			..Settings::default()
		};
		let bytes = serde_json::to_vec(&s).unwrap();
		assert_eq!(Settings::from_json(&bytes).unwrap(), s);
		assert!(Settings::from_json(br#"{"height":0.2,"nozzle":0}"#).is_err());
		let ls = slice(&[Model::cube()], &s).unwrap();
		let code = gcode(&ls, &s);
		assert!(code.contains("F4800"));
		assert!(code.contains("M106 S64"));
		assert!(!code.contains("NaN"));
	}
	#[test]
	fn cube_slice() {
		let s = Settings::default();
		let ls = slice(&[Model::cube()], &s).unwrap();
		assert_eq!(ls.len(), 134);
		assert_eq!(ls[0].walls.len(), 8);
		assert!(ls[0].infill.len() > ls[20].infill.len());
		assert!(gcode(&ls, &s).contains("M109 S200"));
	}
	#[test]
	fn thin_models_and_partial_top_layers_have_paths() {
		for height in [0.03, 0.21] {
			let mut model = Model::cube();
			model.resize_axis(2, height / 20., false);
			model.ground();
			let layers = slice(&[model], &Settings::default()).unwrap();
			assert!(layers.iter().all(|l| !l.walls.is_empty()));
			assert!((layers.last().unwrap().z - height).abs() < 0.0001);
		}
	}
	#[test]
	fn rejects_outside() {
		let mut m = Model::cube();
		m.position[0] = 250.;
		assert!(slice(&[m], &Settings::default()).is_err());
	}
	#[test]
	fn rotations_ground() {
		let mut m = Model::cube();
		m.rotation[0] = 45.;
		m.ground();
		assert!(m.bounds().0[2].abs() < 0.001);
	}
	#[test]
	fn obj_negative_indices() {
		let path = std::env::temp_dir().join(format!("3printer-geometry-{}.obj", std::process::id()));
		fs::write(&path, "v 0 0 0\nv 1 0 0\nv 0 1 0\nf -3 -2 -1\n").unwrap();
		assert_eq!(Model::load(&path).unwrap().triangles.len(), 1);
		fs::remove_file(path).unwrap();
	}
}
