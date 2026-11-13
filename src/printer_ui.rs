use crate::printer::Printer;
use eframe::egui::{self, Color32, RichText};

fn number(
	ui: &mut egui::Ui,
	label: &str,
	description: &str,
	value: &mut f32,
	range: std::ops::RangeInclusive<f32>,
) {
	ui.label(label).on_hover_text(description);
	let speed = if *range.end() <= 2. { 0.01 } else { 0.1 };
	crate::input_field(ui, |ui| {
		ui.add(
			egui::DragValue::new(value)
				.range(range)
				.speed(speed)
				.suffix(" mm"),
		)
	});
}

fn script(ui: &mut egui::Ui, id: &str, title: &str, hint: &str, value: &mut String) {
	ui.strong(title);
	ui.label(
		RichText::new(hint)
			.size(12.)
			.color(Color32::from_rgb(156, 165, 178)),
	);
	egui::Frame::group(ui.style()).inner_margin(10.).show(ui, |ui| {
		egui::ScrollArea::both()
			.id_salt(id)
			.max_height(175.)
			.show(ui, |ui| {
				crate::input_field(ui, |ui| {
					ui.add(
						egui::TextEdit::multiline(value)
							.code_editor()
							.desired_rows(8)
							.desired_width(f32::INFINITY),
					)
				});
			});
	});
}

pub fn content(ui: &mut egui::Ui, p: &mut Printer, tab: usize) {
	ui.add_space(8.);
	match tab {
		0 => {
			ui.horizontal(|ui| {
				ui.label("Profile name")
					.on_hover_text("Name used to identify this printer profile.");
				crate::input_field(ui, |ui| {
					ui.add(egui::TextEdit::singleline(&mut p.name).desired_width(380.))
				});
			});
			ui.add_space(8.);
			ui.strong("Build volume");
			ui.label(
				RichText::new("Dimensions and origin are applied to the workspace and exported paths.")
					.size(12.)
					.color(Color32::from_rgb(156, 165, 178)),
			);
			egui::Frame::group(ui.style()).inner_margin(12.).show(ui, |ui| {
				ui.set_min_width(ui.available_width());
				egui::Grid::new("printer-dimensions")
					.num_columns(4)
					.spacing([18., 10.])
					.show(ui, |ui| {
						ui.label("Build plate shape")
							.on_hover_text("Shape used for placement and printer boundary checks.");
						crate::input_field(ui, |ui| {
							egui::ComboBox::from_id_salt("plate-shape")
								.width(140.)
								.selected_text(if p.circular_bed {
									"Elliptic / circular"
								} else {
									"Rectangular"
								})
								.show_ui(ui, |ui| {
									ui.selectable_value(&mut p.circular_bed, false, "Rectangular");
									ui.selectable_value(&mut p.circular_bed, true, "Elliptic / circular");
								})
						});
						ui.label("Maximum height")
							.on_hover_text("Maximum printable Z height.");
						crate::input_field(ui, |ui| {
							ui.add(
								egui::DragValue::new(&mut p.size[2])
									.range(20. ..=2000.)
									.suffix(" mm"),
							)
						});
						ui.end_row();
						number(
							ui,
							"Plate width",
							"Usable X dimension of the build plate.",
							&mut p.size[0],
							20. ..=2000.,
						);
						number(
							ui,
							"Plate depth",
							"Usable Y dimension of the build plate.",
							&mut p.size[1],
							20. ..=2000.,
						);
						ui.end_row();
						ui.label("Bed heating")
							.on_hover_text("Whether the printer has a heated build plate.");
						ui.checkbox(&mut p.heated_bed, "Heated build plate")
							.on_hover_text("Marks the bed as heated in the profile.");
						ui.label("Coordinate origin").on_hover_text(
							"Selects whether coordinates start at the plate corner or center.",
						);
						ui.checkbox(&mut p.center_origin, "Center of plate")
							.on_hover_text("Uses the plate center as the XY origin.");
						ui.end_row();
					});
			});
			ui.checkbox(
				&mut p.generate_preview,
				"Generate a thumbnail preview in exported G-code",
			)
			.on_hover_text("Adds a small preview image to exported G-code when supported.");
			ui.add_space(10.);
			ui.strong("Print head & firmware");
			ui.label(
				RichText::new("These values describe the hardware used by generated paths.")
					.size(12.)
					.color(Color32::from_rgb(156, 165, 178)),
			);
			egui::Frame::group(ui.style()).inner_margin(12.).show(ui, |ui| {
				ui.set_min_width(ui.available_width());
				egui::Grid::new("printer-head").num_columns(4).spacing([18., 10.]).show(ui, |ui| {
					ui.label("Extruder type").on_hover_text("Describes the filament drive position.");
					crate::input_field(ui, |ui| egui::ComboBox::from_id_salt("extruder-drive").width(140.).selected_text(if p.direct_drive { "Direct drive" } else { "Distal / Bowden" }).show_ui(ui, |ui| {
						ui.selectable_value(&mut p.direct_drive, false, "Distal / Bowden");
						ui.selectable_value(&mut p.direct_drive, true, "Direct drive");
					}));
					number(ui, "Nozzle diameter", "Diameter used for line width and extrusion calculations.", &mut p.nozzle, 0.1..=2.);
					ui.end_row();
					ui.label("G-code flavor").on_hover_text("Command dialect emitted by the slicer.");
					ui.add_enabled(false, egui::Button::new("Marlin")).on_hover_text("The current slicer exports Marlin G-code.");
					ui.label("Number of extruders").on_hover_text("Number of independently controlled nozzles.");
					crate::input_field(ui, |ui| ui.add_enabled(false, egui::DragValue::new(&mut 1)))
						.on_hover_text("The current slicer supports one extruder.");
					ui.end_row();
				});
				ui.add_space(8.);
				ui.label("Machine head & fan polygon (mm)").on_hover_text("Machine-head footprint used for future clearance checks.");
				crate::input_field(ui, |ui| ui.add(egui::TextEdit::singleline(&mut p.head_polygon).font(egui::TextStyle::Monospace).desired_width(f32::INFINITY)));
				ui.label(RichText::new("Head footprint and drive type are saved with the profile. Sequential-print clearance and drive-specific retraction are not applied by this slicer.").small().color(Color32::GRAY));
			});
		}
		1 => {
			script(
				ui,
				"machine-start",
				"Start G-code",
				"Runs after heating. Custom scripts use firmware coordinates.",
				&mut p.start,
			);
			ui.add_space(8.);
			script(
				ui,
				"machine-end",
				"End G-code",
				"Runs after Extruder 1 end code, with relative extrusion enabled.",
				&mut p.end,
			);
		}
		_ => {
			ui.label(
				RichText::new("Extruder 1 · start and end sequence")
					.strong()
					.size(17.),
			);
			ui.label("Use $$ for the selected nozzle temperature, for example M104 S$$.");
			ui.add_space(8.);
			script(ui, "extruder-start", "Extruder start G-code", "Runs after machine start. Absolute coordinates and extrusion are restored before model paths.", &mut p.extruder_start);
			ui.add_space(8.);
			script(
				ui,
				"extruder-end",
				"Extruder end G-code",
				"Runs before machine end, in absolute extrusion mode. Reset E before a short retract.",
				&mut p.extruder_end,
			);
		}
	}
	ui.add_space(8.);
}
