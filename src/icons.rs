//! Lucide icon geometry (ISC license, Lucide contributors).
//! https://lucide.dev/license — 24px viewBox, 2px rounded strokes.
use eframe::egui::{self, Color32, Pos2, Stroke, Vec2};
fn paint(p: &egui::Painter, origin: Pos2, name: &str, color: Color32) {
	let paths: &[&[[f32; 2]]] = match name {
		"scan-box" => &[
			&[[3., 7.], [3., 3.], [7., 3.]],
			&[[17., 3.], [21., 3.], [21., 7.]],
			&[[21., 17.], [21., 21.], [17., 21.]],
			&[[7., 21.], [3., 21.], [3., 17.]],
			&[
				[12., 7.],
				[17., 10.],
				[17., 15.],
				[12., 18.],
				[7., 15.],
				[7., 10.],
				[12., 7.],
			],
			&[[7., 10.], [12., 13.], [17., 10.]],
			&[[12., 13.], [12., 18.]],
		],
		"bottom" => &[
			&[[3., 3.], [21., 3.]],
			&[[12., 7.], [5., 19.], [19., 19.], [12., 7.]],
		],
		"select" => &[&[[3., 3.], [7., 21.], [10., 14.], [17., 11.], [3., 3.]]],
		"move" => &[
			&[[5., 9.], [2., 12.], [5., 15.]],
			&[[9., 5.], [12., 2.], [15., 5.]],
			&[[19., 9.], [22., 12.], [19., 15.]],
			&[[9., 19.], [12., 22.], [15., 19.]],
			&[[2., 12.], [22., 12.]],
			&[[12., 2.], [12., 22.]],
		],
		"scale" => &[
			&[[15., 3.], [21., 3.], [21., 9.]],
			&[[21., 3.], [14., 10.]],
			&[[9., 21.], [3., 21.], [3., 15.]],
			&[[3., 21.], [10., 14.]],
		],
		"rotate" => &[
			&[
				[3., 11.],
				[5., 6.],
				[10., 3.],
				[16., 4.],
				[20., 8.],
				[21., 12.],
				[19., 18.],
				[14., 21.],
				[8., 20.],
				[4., 16.],
			],
			&[[3., 3.], [3., 11.], [11., 11.]],
		],
		"rotate-cw-clock" => &[
			&[
				[19., 8.],
				[19., 3.],
				[14., 3.],
				[18., 7.],
				[20., 12.],
				[18., 17.],
				[13., 20.],
				[8., 18.],
				[5., 13.],
				[6., 8.],
				[10., 5.],
			],
			&[[12., 8.], [12., 13.], [15., 16.]],
			&[[12., 3.], [12., 5.]],
		],
		"home" => &[
			&[[3., 10.], [12., 3.], [21., 10.], [21., 21.], [3., 21.], [3., 10.]],
			&[[9., 21.], [9., 12.], [15., 12.], [15., 21.]],
		],
		"import" => &[
			&[
				[21., 15.],
				[21., 19.],
				[19., 21.],
				[5., 21.],
				[3., 19.],
				[3., 15.],
			],
			&[[7., 10.], [12., 15.], [17., 10.]],
			&[[12., 15.], [12., 3.]],
		],
		"folder" => &[&[
			[3., 20.],
			[3., 4.],
			[9., 4.],
			[12., 7.],
			[21., 7.],
			[21., 20.],
			[3., 20.],
		]],
		"save" => &[
			&[
				[4., 3.],
				[18., 3.],
				[21., 6.],
				[21., 21.],
				[3., 21.],
				[3., 3.],
				[4., 3.],
			],
			&[[7., 3.], [7., 9.], [17., 9.], [17., 3.]],
			&[[7., 21.], [7., 14.], [17., 14.], [17., 21.]],
		],
		"trash" => &[
			&[[3., 6.], [21., 6.]],
			&[[9., 6.], [9., 3.], [15., 3.], [15., 6.]],
			&[[5., 6.], [6., 21.], [18., 21.], [19., 6.]],
			&[[10., 10.], [10., 17.]],
			&[[14., 10.], [14., 17.]],
		],
		"top" => &[
			&[[3., 7.], [12., 2.], [21., 7.], [12., 12.], [3., 7.]],
			&[[3., 7.], [3., 18.], [12., 23.], [21., 18.], [21., 7.]],
		],
		"front" => &[&[[4., 4.], [20., 4.], [20., 20.], [4., 20.], [4., 4.]]],
		"help" => &[
			&[
				[8., 7.],
				[9., 4.],
				[15., 4.],
				[18., 7.],
				[17., 10.],
				[12., 13.],
				[12., 16.],
			],
			&[[12., 20.], [12., 21.]],
		],
		"license" => &[
			&[[6., 3.], [17., 3.], [21., 7.], [21., 21.], [6., 21.], [6., 3.]],
			&[[17., 3.], [17., 7.], [21., 7.]],
			&[[10., 12.], [12., 14.], [16., 10.]],
		],
		"check" => &[&[[20., 6.], [9., 17.], [4., 12.]]],
		"settings" => &[
			&[[4., 21.], [4., 14.]],
			&[[4., 10.], [4., 3.]],
			&[[12., 21.], [12., 12.]],
			&[[12., 8.], [12., 3.]],
			&[[20., 21.], [20., 16.]],
			&[[20., 12.], [20., 3.]],
			&[[1., 14.], [7., 14.]],
			&[[9., 8.], [15., 8.]],
			&[[17., 16.], [23., 16.]],
		],
		_ => &[
			&[
				[12., 3.],
				[21., 8.],
				[21., 18.],
				[12., 23.],
				[3., 18.],
				[3., 8.],
				[12., 3.],
			],
			&[[3., 8.], [12., 13.], [21., 8.]],
			&[[12., 13.], [12., 23.]],
		],
	};
	for path in paths {
		p.add(egui::Shape::line(
			path.iter()
				.map(|v| origin + Vec2::new(v[0], v[1]) * 0.8)
				.collect(),
			Stroke::new(1.6, color),
		));
	}
}
pub fn icon(ui: &mut egui::Ui, name: &str, color: Color32) {
	let (r, _) = ui.allocate_exact_size(Vec2::splat(22.), egui::Sense::hover());
	paint(ui.painter(), r.min, name, color);
}
pub fn action(ui: &mut egui::Ui, name: &str, label: &str, shortcut: &str, selected: bool) -> egui::Response {
	let (r, response) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 40.), egui::Sense::click());
	let color = if selected {
		Color32::from_rgb(111, 198, 255)
	} else {
		Color32::from_gray(205)
	};
	if selected || response.hovered() {
		ui.painter().rect_filled(
			r,
			5.,
			if selected {
				Color32::from_rgb(42, 67, 88)
			} else {
				Color32::from_gray(48)
			},
		);
	}
	paint(ui.painter(), r.min + Vec2::new(7., 10.), name, color);
	ui.painter().text(
		r.min + Vec2::new(35., 20.),
		egui::Align2::LEFT_CENTER,
		label,
		egui::FontId::proportional(12.),
		color,
	);
	ui.painter().text(
		r.right_center() - Vec2::new(6., 0.),
		egui::Align2::RIGHT_CENTER,
		shortcut,
		egui::FontId::proportional(10.),
		Color32::GRAY,
	);
	response
		.on_hover_cursor(egui::CursorIcon::PointingHand)
		.on_hover_text(label)
}

pub fn brand(ui: &mut egui::Ui) {
	let (r, _) = ui.allocate_exact_size(Vec2::splat(32.), egui::Sense::hover());
	ui.painter().rect_filled(r, 8., Color32::from_rgb(65, 70, 78));
	paint(
		ui.painter(),
		r.min + Vec2::splat(6.4),
		"scan-box",
		Color32::from_gray(238),
	);
}
pub fn app_icon() -> egui::IconData {
	let image = image::load_from_memory(include_bytes!("../assets/app-icon.png"))
		.expect("app icon")
		.into_rgba8();
	egui::IconData {
		width: image.width(),
		height: image.height(),
		rgba: image.into_raw(),
	}
}

#[cfg(target_os = "macos")]
pub fn set_dock_icon() {
	use std::ffi::{c_char, c_void, CString};
	#[link(name = "objc")]
	extern "C" {
		fn objc_getClass(name: *const c_char) -> *mut c_void;
		fn sel_registerName(name: *const c_char) -> *mut c_void;
		fn objc_msgSend();
	}
	unsafe {
		let class = |name: &str| objc_getClass(CString::new(name).unwrap().as_ptr());
		let sel = |name: &str| sel_registerName(CString::new(name).unwrap().as_ptr());
		let send: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
			std::mem::transmute(objc_msgSend as *const ());
		let send_arg: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> *mut c_void =
			std::mem::transmute(objc_msgSend as *const ());
		let bundle_icon = std::env::current_exe()
			.ok()
			.and_then(|exe| {
				exe.parent()
					.and_then(|dir| dir.parent())
					.map(std::path::Path::to_path_buf)
			})
			.map(|contents| contents.join("Resources/AppIcon.icns"))
			.filter(|path| path.exists());
		let path = bundle_icon.unwrap_or_else(|| {
			std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/app-icon.png"))
		});
		let path = CString::new(path.to_string_lossy().as_bytes()).unwrap();
		let string = send_arg(
			class("NSString"),
			sel("stringWithUTF8String:"),
			path.as_ptr() as *mut c_void,
		);
		let image = send_arg(
			send(class("NSImage"), sel("alloc")),
			sel("initWithContentsOfFile:"),
			string,
		);
		if !image.is_null() {
			send_arg(
				send(class("NSApplication"), sel("sharedApplication")),
				sel("setApplicationIconImage:"),
				image,
			);
			send(image, sel("release"));
		}
	}
}

/// Full-width menu row with a leading vector icon and a right-aligned shortcut.
pub fn menu_item(ui: &mut egui::Ui, name: &str, label: &str, shortcut: &str) -> egui::Response {
	let font = egui::TextStyle::Button.resolve(ui.style());
	let color = ui.visuals().text_color();
	let label = ui.painter().layout_no_wrap(label.into(), font.clone(), color);
	let shortcut = ui.painter().layout_no_wrap(shortcut.into(), font, color);
	// Reserve actual columns: whitespace in Button text is trimmed by egui.
	let width = (36. + label.size().x + 24. + shortcut.size().x + 10.).max(260.);
	let response = ui.add(egui::Button::new("").min_size(Vec2::new(width, 32.)));
	let color = if ui.is_enabled() {
		Color32::WHITE
	} else {
		Color32::GRAY
	};
	if !ui.is_enabled() && response.contains_pointer() {
		ui.ctx().set_cursor_icon(egui::CursorIcon::NotAllowed);
	}
	paint(
		ui.painter(),
		response.rect.left_center() + Vec2::new(7., -10.),
		name,
		color,
	);
	ui.painter().galley_with_override_text_color(
		response.rect.left_center() + Vec2::new(36., -label.size().y / 2.),
		label,
		color,
	);
	ui.painter().galley_with_override_text_color(
		response.rect.right_center() - Vec2::new(10. + shortcut.size().x, shortcut.size().y / 2.),
		shortcut,
		color,
	);
	response
}
