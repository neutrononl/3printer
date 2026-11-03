#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Printer {
	pub name: String,
	pub circular_bed: bool,
	pub generate_preview: bool,
	pub center_origin: bool,
	pub head_polygon: String,
	pub direct_drive: bool,
	pub extruder_start: String,
	pub extruder_end: String,
	pub size: [f32; 3],
	pub nozzle: f32,
	pub heated_bed: bool,
	pub start: String,
	pub end: String,
}
impl Default for Printer {
	fn default() -> Self {
		Self {
			name: "Creality Ender 5".into(),
			circular_bed: false,
			generate_preview: false,
			center_origin: false,
			head_polygon: "[[-20,10],[10,10],[10,-10],[-20,-10]]".into(),
			direct_drive: false,
			extruder_start: "G92 E0 ;Reset extruder distance\nM104 S$$".into(),
			extruder_end: "G92 E0 ;Reset extruder distance\nG1 F800 E-5 ;Short retract\nG92 E0 ;Reset extruder distance\nG90".into(),
			size: [220., 220., 300.],
			nozzle: 0.4,
			heated_bed: true,
			start: r#"M201 X500.00 Y500.00 Z100.00 E5000.00 ;Setup machine max acceleration
M203 X500.00 Y500.00 Z10.00 E50.00 ;Setup machine max feedrate
M204 P500.00 R1000.00 T500.00 ;Setup Print/Retract/Travel acceleration
M205 X8.00 Y8.00 Z0.40 E5.00 ;Setup Jerk
M220 S100 ;Reset Feedrate
M221 S100 ;Reset Flowrate

G28 ;Home

G92 E0 ;Reset Extruder
G1 Z2.0 F3000 ;Move Z Axis up
G1 X10.1 Y20 Z0.28 F5000.0 ;Move to start position
G1 X10.1 Y145.0 Z0.28 F1500.0 E15 ;Draw the first line
G1 X10.4 Y145.0 Z0.28 F5000.0 ;Move to side a little
G1 X10.4 Y20 Z0.28 F1500.0 E30 ;Draw the second line
G92 E0  ;Reset Extruder
G1 E-1.0000 F1800 ;Retract a bit
G1 Z2.0 F3000 ;Move Z Axis up
G1 E0.0000 F1800"#
				.into(),
			end: r#"G91 ;Relative positionning
G1 E-2 F2700 ;Retract a bit
G1 E-2 Z0.2 F2400 ;Retract and raise Z
G1 X5 Y5 F3000 ;Wipe out
G1 Z10 ;Raise Z more
G90 ;Absolute positionning

G1 X0 Y0 ;Present print
M106 S0 ;Turn-off fan
M104 S0 ;Turn-off hotend
M140 S0 ;Turn-off bed

M84 X Y E ;Disable all steppers but Z"#
				.into(),
		}
	}
}
impl Printer {
	pub fn from_json(bytes: &[u8]) -> Result<Self, String> {
		let value: serde_json::Value =
			serde_json::from_slice(bytes).map_err(|e| format!("Invalid JSON: {e}"))?;
		let data = if value.get("format").is_some() {
			if value.get("format").and_then(|v| v.as_str()) != Some("3printer-printer")
				|| value.get("version").and_then(|v| v.as_u64()) != Some(1)
			{
				return Err("Unsupported printer profile format or version".into());
			}
			value.get("printer").ok_or("Missing printer profile")?
		} else {
			&value
		};
		if !data.is_object() || data.get("name").is_none() || data.get("size").is_none() {
			return Err("Not a printer profile: expected printer name and build dimensions".into());
		}
		let printer: Self = serde_json::from_value(data.clone()).map_err(|e| e.to_string())?;
		printer.validate()?;
		Ok(printer)
	}
	pub fn to_json(&self) -> Result<Vec<u8>, String> {
		self.validate()?;
		serde_json::to_vec_pretty(&serde_json::json!({
			"format": "3printer-printer", "version": 1, "printer": self
		}))
		.map_err(|e| e.to_string())
	}
	pub fn contains(&self, p: [f32; 3]) -> bool {
		if (0..3).any(|a| p[a] < -0.001 || p[a] > self.size[a] + 0.001) {
			return false;
		}
		!self.circular_bed
			|| ((p[0] - self.size[0] / 2.) / (self.size[0] / 2.)).powi(2)
				+ ((p[1] - self.size[1] / 2.) / (self.size[1] / 2.)).powi(2)
				<= 1.00001
	}
	pub fn output_xy(&self, p: [f32; 2]) -> [f32; 2] {
		if self.center_origin {
			[p[0] - self.size[0] / 2., p[1] - self.size[1] / 2.]
		} else {
			p
		}
	}

	pub fn validate(&self) -> Result<(), String> {
		if self.name.trim().is_empty() {
			return Err("Enter a printer name".into());
		}
		if self
			.size
			.iter()
			.any(|v| !v.is_finite() || !(20. ..=2000.).contains(v))
		{
			return Err("Build dimensions must be between 20 and 2000 mm".into());
		}
		if !self.nozzle.is_finite() || !(0.1..=2.).contains(&self.nozzle) {
			return Err("Nozzle diameter must be between 0.1 and 2 mm".into());
		}
		let polygon: Vec<[f32; 2]> = serde_json::from_str(&self.head_polygon)
			.map_err(|_| "Head polygon must be an array of [X, Y] coordinates")?;
		if polygon.len() < 3 || polygon.iter().flatten().any(|n| !n.is_finite()) {
			return Err("Head polygon needs at least three finite points".into());
		}
		Ok(())
	}
}

pub fn thumbnail(layers: &[crate::geometry::Layer], printer: &Printer) -> String {
	let mut image = image::RgbImage::from_pixel(128, 128, image::Rgb([35, 39, 46]));
	for layer in layers {
		for [a, b] in layer.walls.iter().chain(&layer.infill) {
			let a = [a[0] / printer.size[0] * 127., a[1] / printer.size[1] * 127.];
			let b = [b[0] / printer.size[0] * 127., b[1] / printer.size[1] * 127.];
			let steps = ((b[0] - a[0]).abs().max((b[1] - a[1]).abs()).ceil() as usize).clamp(1, 256);
			for i in 0..=steps {
				let t = i as f32 / steps as f32;
				let x = (a[0] + (b[0] - a[0]) * t).round().clamp(0., 127.) as u32;
				let y = (a[1] + (b[1] - a[1]) * t).round().clamp(0., 127.) as u32;
				image.put_pixel(x, 127 - y, image::Rgb([30, 159, 215]));
			}
		}
	}
	let mut png = std::io::Cursor::new(Vec::new());
	if image::DynamicImage::ImageRgb8(image)
		.write_to(&mut png, image::ImageFormat::Png)
		.is_err()
	{
		return String::new();
	}
	const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
	let mut encoded = String::new();
	for chunk in png.get_ref().chunks(3) {
		let n = ((chunk[0] as u32) << 16)
			| ((chunk.get(1).copied().unwrap_or(0) as u32) << 8)
			| chunk.get(2).copied().unwrap_or(0) as u32;
		encoded.push(ALPHABET[((n >> 18) & 63) as usize] as char);
		encoded.push(ALPHABET[((n >> 12) & 63) as usize] as char);
		encoded.push(if chunk.len() > 1 {
			ALPHABET[((n >> 6) & 63) as usize] as char
		} else {
			'='
		});
		encoded.push(if chunk.len() > 2 {
			ALPHABET[(n & 63) as usize] as char
		} else {
			'='
		});
	}
	let mut output = format!("; thumbnail begin 128x128 {}\n", encoded.len());
	for chunk in encoded.as_bytes().chunks(78) {
		output += "; ";
		output += std::str::from_utf8(chunk).unwrap();
		output += "\n";
	}
	output + "; thumbnail end\n"
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn portable_profile_roundtrip_and_rejection() {
		let mut p = Printer::default();
		p.name = "Custom printer".into();
		p.size = [250., 260., 400.];
		p.center_origin = true;
		p.generate_preview = true;
		p.direct_drive = true;
		p.start = "G28\n; custom machine start".into();
		p.end = "M84 X Y E".into();
		p.extruder_start = "M104 S$$\nG92 E0".into();
		p.extruder_end = "G92 E0\nG1 E-3".into();
		assert_eq!(Printer::from_json(&p.to_json().unwrap()).unwrap(), p);
		assert_eq!(Printer::from_json(&serde_json::to_vec(&p).unwrap()).unwrap(), p);
		for invalid in [
			b"{}".as_slice(),
			b"not json",
			b"[]",
			br#"{"height":0.2}"#,
			br#"{"name":"Bad","size":[0,220,300]}"#,
			br#"{"format":"3printer-printer","version":99,"printer":{}}"#,
		] {
			assert!(Printer::from_json(invalid).is_err());
		}
		p.nozzle = 0.;
		assert!(p.to_json().is_err());
	}
	#[test]
	fn legacy_profiles_and_origin_and_ellipse() {
		let mut p: Printer =
			serde_json::from_str(r#"{"name":"Older profile","size":[220,220,300],"nozzle":0.4}"#).unwrap();
		assert!(p.validate().is_ok());
		assert!(!p.extruder_start.is_empty());
		p.center_origin = true;
		assert_eq!(p.output_xy([110., 110.]), [0., 0.]);
		assert_eq!(p.output_xy([0., 220.]), [-110., 110.]);
		p.circular_bed = true;
		assert!(p.contains([110., 110., 0.]));
		assert!(!p.contains([5., 5., 0.]));
		let mut m = crate::geometry::Model::cube();
		assert!(m.find_free_position_for(&[], &p));
		let mut settings = crate::geometry::Settings::default();
		settings.printer = p.clone();
		m.position = [15., 15., 0.];
		assert!(crate::geometry::slice(&[m], &settings).is_err());
		p.head_polygon = "invalid".into();
		assert!(p.validate().is_err());
	}
	#[test]
	fn scripts_and_preview_reach_export() {
		let mut settings = crate::geometry::Settings::default();
		settings.printer.center_origin = true;
		settings.printer.generate_preview = true;
		settings.printer.extruder_start = "M104 S$$ ;extruder start".into();
		settings.printer.extruder_end = "G92 E0 ;extruder end".into();
		let layers = vec![crate::geometry::Layer {
			z: 0.2,
			walls: vec![[[110., 110.], [120., 110.]]],
			infill: vec![],
		}];
		let code = crate::geometry::gcode(&layers, &settings);
		assert!(code.contains("; thumbnail begin 128x128 "));
		assert!(code.contains("; iVBOR"));
		assert!(code.contains("M104 S200 ;extruder start"));
		assert!(code.contains("G0 X0.000 Y0.000"));
		assert!(code.contains("G1 X10.000 Y0.000"));
		assert!(code.find(";extruder end").unwrap() < code.find(";Relative positionning").unwrap());
		assert!(code.find(";extruder start").unwrap() < code.find(";LAYER:0").unwrap());
		let data = serde_json::to_vec(&settings).unwrap();
		assert_eq!(crate::geometry::Settings::from_json(&data).unwrap(), settings);
	}
}
