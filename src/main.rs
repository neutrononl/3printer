#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod geometry;
mod icons;
mod menus;
mod printer;
mod printer_ui;
use eframe::egui::{self, Color32, Pos2, RichText, Stroke, Vec2};
use geometry::*;
const BLUE: Color32 = Color32::from_rgb(30, 159, 215);
const BUTTON_BLUE: Color32 = Color32::from_rgb(18, 94, 145);
const MUTED: Color32 = Color32::from_rgb(156, 165, 178);
const TITLE: Color32 = Color32::from_rgb(225, 228, 235);
fn kebab_name(name: &str, fallback: &str) -> String {
	let name = name
		.split(|c: char| !c.is_alphanumeric())
		.filter(|part| !part.is_empty())
		.map(str::to_lowercase)
		.collect::<Vec<_>>()
		.join("-");
	if name.is_empty() {
		fallback.into()
	} else {
		name
	}
}
#[derive(PartialEq, Clone, Copy)]
enum Tool {
	Select,
	Move,
	Scale,
	Rotate,
	PlaceBottom,
}
struct Studio {
	printers: Vec<printer::Printer>,
	show_printers: bool,
	printer_tab: usize,
	printer_message: String,
	printer_draft: printer::Printer,
	printer_index: Option<usize>,
	saved_models: Vec<Model>,
	saved_settings: Settings,
	allow_close: bool,
	slicing: Option<std::sync::mpsc::Receiver<Result<Vec<Layer>, String>>>,
	models: Vec<Model>,
	selected: Option<usize>,
	selection: Vec<usize>,
	clone_count: usize,
	scale_original: Vec<Option<Model>>,
	history: Vec<Vec<Model>>,
	tool: Tool,
	rotation_axis: usize,
	uniform: bool,
	playing: bool,
	play_speed: f32,
	play_step: usize,
	play_clock: f64,
	support_grid: f32,
	support_angle: f32,
	drag_model: Option<usize>,
	drag_axis: Option<usize>,
	settings: Settings,
	profile: usize,
	layers: Vec<Layer>,
	layer: usize,
	preview: bool,
	show_settings: bool,
	show_help: bool,
	show_license: bool,
	status: String,
	yaw: f32,
	pitch: f32,
	zoom: f32,
	code_view: bool,
	code: String,
	frame: u32,
	screenshot: Option<String>,
	requested: bool,
}
impl Studio {
	fn new(cc: &eframe::CreationContext<'_>) -> Self {
		#[cfg(target_os = "macos")]
		{
			icons::set_dock_icon();
			menus::native::install(cc.egui_ctx.clone());
		}
		let mut style = (*cc.egui_ctx.style()).clone();
		style.visuals = egui::Visuals::dark();
		style.visuals.interact_cursor = Some(egui::CursorIcon::PointingHand);
		style.visuals.panel_fill = Color32::from_rgb(40, 43, 49);
		style.visuals.window_fill = Color32::from_rgb(47, 51, 58);
		style.visuals.selection.bg_fill = BUTTON_BLUE;
		style.visuals.selection.stroke = Stroke::new(1., Color32::WHITE);
		// Selected buttons inherit the current interaction state's text color.
		// Keep idle, hovered and pressed labels legible on the blue fill.
		style.visuals.widgets.inactive.fg_stroke.color = Color32::from_gray(240);
		style.visuals.widgets.hovered.fg_stroke.color = Color32::WHITE;
		style.visuals.widgets.active.fg_stroke.color = Color32::WHITE;
		style.spacing.item_spacing = Vec2::new(10., 10.);
		style.spacing.button_padding = Vec2::new(12., 8.);
		cc.egui_ctx.set_style(style);
		let printers: Vec<printer::Printer> = std::fs::read(Self::printers_path())
			.ok()
			.and_then(|b| serde_json::from_slice::<Vec<printer::Printer>>(&b).ok())
			.filter(|p| !p.is_empty() && p.iter().all(|p| p.validate().is_ok()))
			.unwrap_or_else(|| vec![printer::Printer::default()]);
		let mut settings = Settings::default();
		settings.printer = printers[0].clone();
		settings.nozzle = settings.printer.nozzle;
		Self {
			printers,
			printer_message: String::new(),
			show_printers: std::env::args().any(|a| a == "--printer-settings"),
			printer_tab: std::env::args()
				.skip_while(|a| a != "--printer-tab")
				.nth(1)
				.and_then(|a| a.parse().ok())
				.unwrap_or(0),
			printer_draft: settings.printer.clone(),
			printer_index: Some(0),
			saved_models: vec![],
			saved_settings: settings.clone(),
			allow_close: false,
			slicing: None,
			models: vec![],
			selected: None,
			selection: vec![],
			clone_count: 1,
			scale_original: vec![],
			history: vec![],
			tool: Tool::Select,
			rotation_axis: 2,
			uniform: true,
			playing: false,
			play_speed: 100.,
			play_step: 0,
			play_clock: 0.,
			support_grid: 5.,
			support_angle: 60.,
			drag_model: None,
			drag_axis: None,
			settings,
			profile: 1,
			layers: vec![],
			layer: 1,
			preview: false,
			show_settings: false,
			show_help: false,
			show_license: false,
			status: "Ready · Import a model to begin".into(),
			yaw: -0.65,
			pitch: 0.8,
			zoom: 1.,
			code_view: false,
			code: String::new(),
			frame: 0,
			screenshot: std::env::args().skip_while(|a| a != "--screenshot").nth(1),
			requested: false,
		}
	}
	fn printers_path() -> std::path::PathBuf {
		let base = std::env::var_os("HOME")
			.map(std::path::PathBuf::from)
			.unwrap_or_else(std::env::temp_dir);
		base.join(".3printer/printers.json")
	}
	fn save_printers(&mut self) {
		let path = Self::printers_path();
		let result = std::fs::create_dir_all(path.parent().unwrap())
			.and_then(|_| std::fs::write(path, serde_json::to_vec_pretty(&self.printers).unwrap()));
		if let Err(e) = result {
			self.status = format!("Could not save printers: {e}");
		}
	}
	fn import_printer(&mut self) {
		let Some(path) = rfd::FileDialog::new()
			.set_title("Import printer profile")
			.add_filter("Printer profile", &["json"])
			.pick_file()
		else {
			return;
		};
		match std::fs::read(path)
			.map_err(|e| e.to_string())
			.and_then(|bytes| printer::Printer::from_json(&bytes))
		{
			Ok(printer) => {
				self.printer_draft = printer;
				self.printer_index = None;
				self.printer_message =
					"Profile imported. Choose Save and select to add it to your printers.".into();
			}
			Err(e) => self.printer_message = format!("Import failed: {e}"),
		}
	}
	fn export_printer(&mut self) {
		let data = match self.printer_draft.to_json() {
			Ok(data) => data,
			Err(e) => {
				self.printer_message = format!("Export failed: {e}");
				return;
			}
		};
		let name = kebab_name(&self.printer_draft.name, "printer");
		let Some(path) = rfd::FileDialog::new()
			.set_title("Export printer profile")
			.add_filter("Printer profile", &["json"])
			.set_file_name(format!("{name}.printer.json"))
			.save_file()
		else {
			return;
		};
		self.printer_message = match std::fs::write(path, data) {
			Ok(_) => "Printer profile exported, including current edits and all G-code scripts.".into(),
			Err(e) => format!("Export failed: {e}"),
		};
	}
	fn printer_window(&mut self, ctx: &egui::Context) {
		let mut open = self.show_printers;
		let mut close = false;
		egui::Window::new("Manage printers")
			.open(&mut open)
			.default_width(720.)
			.min_width(660.)
			.collapsible(false)
			.show(ctx, |ui| {
				ui.horizontal(|ui| {
					egui::ComboBox::from_id_salt("manage-printer")
						.selected_text(&self.printer_draft.name)
						.show_ui(ui, |ui| {
							for i in 0..self.printers.len() {
								if ui
									.selectable_label(self.printer_index == Some(i), &self.printers[i].name)
									.clicked()
								{
									self.printer_index = Some(i);
									self.printer_draft = self.printers[i].clone();
								}
							}
						});
					if ui.button("Add printer").clicked() {
						self.printer_index = None;
						self.printer_draft = printer::Printer::default();
						self.printer_draft.name = "New printer".into();
					}
					if ui
						.button("Import…")
						.on_hover_text("Load a printer profile from JSON")
						.clicked()
					{
						self.import_printer();
					}
					if ui
						.add_enabled(
							self.printer_draft.validate().is_ok(),
							egui::Button::new("Export…"),
						)
						.on_hover_text("Export the current profile and edits as JSON")
						.clicked()
					{
						self.export_printer();
					}
				});
				if !self.printer_message.is_empty() {
					ui.label(RichText::new(&self.printer_message).small());
				}
				ui.add_space(8.);
				ui.horizontal(|ui| {
					for (i, label) in ["Basic parameters", "Machine G-code", "Extruder 1"]
						.iter()
						.enumerate()
					{
						ui.selectable_value(&mut self.printer_tab, i, *label);
					}
				});
				ui.separator();
				egui::ScrollArea::vertical()
					.id_salt("printer-content")
					.max_height(470.)
					.show(ui, |ui| {
						ui.set_min_height(350.);
						printer_ui::content(ui, &mut self.printer_draft, self.printer_tab);
					});
				ui.separator();
				let valid = self.printer_draft.validate();
				if let Err(e) = &valid {
					ui.colored_label(Color32::LIGHT_RED, e);
				}
				ui.horizontal(|ui| {
					if ui
						.add_enabled(
							valid.is_ok(),
							egui::Button::new(RichText::new("Save and select").color(Color32::WHITE))
								.fill(BUTTON_BLUE),
						)
						.clicked()
					{
						if let Some(i) = self.printer_index {
							self.printers[i] = self.printer_draft.clone();
						} else {
							self.printers.push(self.printer_draft.clone());
							self.printer_index = Some(self.printers.len() - 1);
						}
						self.settings.printer = self.printer_draft.clone();
						self.settings.nozzle = self.printer_draft.nozzle;
						self.invalidate();
						self.save_printers();
						close = true;
					}
					if ui
						.add_enabled(
							self.printer_index.is_some() && self.printers.len() > 1,
							egui::Button::new("Remove printer"),
						)
						.clicked()
					{
						let removed = self.printers.remove(self.printer_index.unwrap());
						if self.settings.printer == removed {
							self.settings.printer = self.printers[0].clone();
							self.settings.nozzle = self.settings.printer.nozzle;
							self.invalidate();
						}
						self.printer_index = Some(0);
						self.printer_draft = self.printers[0].clone();
						self.save_printers();
					}
					if ui
						.button("Reset")
						.on_hover_text("Restore Ender 5 defaults in this draft")
						.clicked()
					{
						self.printer_draft = printer::Printer::default();
					}
					if ui.button("Cancel").clicked() {
						close = true;
					}
				});
			});
		self.show_printers = open && !close;
	}
	fn import_settings(&mut self) {
		if let Some(path) = rfd::FileDialog::new()
			.add_filter("3printer settings", &["json"])
			.pick_file()
		{
			match std::fs::read(path)
				.map_err(|e| e.to_string())
				.and_then(|b| Settings::from_json(&b))
			{
				Ok(settings) => {
					self.settings = settings;
					self.profile = 3;
					self.invalidate();
					self.status = "Settings imported".into();
				}
				Err(error) => self.status = format!("Settings import failed: {error}"),
			}
		}
	}
	fn export_settings(&mut self) {
		if let Some(path) = rfd::FileDialog::new()
			.add_filter("3printer settings", &["json"])
			.set_file_name("print-settings.json")
			.save_file()
		{
			let data = serde_json::json!({"version": 2, "settings": self.settings});
			self.status = match std::fs::write(path, serde_json::to_vec_pretty(&data).unwrap()) {
				Ok(_) => "Settings exported".into(),
				Err(e) => e.to_string(),
			};
		}
	}
	fn actions(&mut self, ctx: &egui::Context) {
		egui::SidePanel::left("actions")
			.exact_width(154.)
			.resizable(false)
			.frame(
				egui::Frame::new()
					.fill(Color32::from_rgb(30, 32, 37))
					.inner_margin(12.),
			)
			.show(ctx, |ui| {
				ui.add_space(8.);
				ui.label(RichText::new("TOOLS").small().color(MUTED));
				ui.separator();
				for (tool, icon, label, shortcut) in [
					(Tool::Select, "select", "Select", "V"),
					(Tool::Move, "move", "Move", "G"),
					(Tool::Scale, "scale", "Scale", "S"),
					(Tool::Rotate, "rotate", "Rotate", "R"),
					(Tool::PlaceBottom, "bottom", "Place bottom", "P"),
				] {
					if icons::action(ui, icon, label, shortcut, self.tool == tool).clicked() {
						self.tool = tool;
						if tool == Tool::PlaceBottom {
							self.place_bottom();
						}
					}
				}
				ui.add_space(14.);
				ui.label(RichText::new("WORKSPACE").small().color(MUTED));
				ui.separator();
				if icons::action(ui, "import", "Import model", "", false).clicked() {
					self.import();
				}
				if icons::action(ui, "box", "Add cube", "", false).clicked() {
					self.checkpoint();
					self.add_model(Model::cube());
				}
				if icons::action(ui, "home", "Reset view", "", false).clicked() {
					self.yaw = -0.65;
					self.pitch = 0.8;
					self.zoom = 1.;
				}
				if icons::action(ui, "settings", "Configure", "", self.show_settings).clicked() {
					self.show_settings = !self.show_settings;
				}
				if !ctx.wants_keyboard_input() {
					for (key, tool) in [
						(egui::Key::V, Tool::Select),
						(egui::Key::G, Tool::Move),
						(egui::Key::S, Tool::Scale),
						(egui::Key::R, Tool::Rotate),
						(egui::Key::P, Tool::PlaceBottom),
					] {
						if ctx.input(|i| i.modifiers.is_none() && i.key_pressed(key)) {
							self.tool = tool;
							if tool == Tool::PlaceBottom {
								self.place_bottom();
							}
						}
					}
				}
			});
	}
	fn checkpoint(&mut self) {
		self.history.push(self.models.clone());
		if self.history.len() > 25 {
			self.history.remove(0);
		}
		self.invalidate();
	}
	fn is_selected(&self, i: usize) -> bool {
		self.selected == Some(i) || (self.selected.is_some() && self.selection.contains(&i))
	}
	fn select_model(&mut self, hit: Option<usize>, extend: bool) {
		if !extend {
			self.selection.clear();
		}
		if let Some(i) = hit {
			if extend && self.is_selected(i) {
				self.selection.retain(|&n| n != i);
				if self.selected == Some(i) {
					self.selected = self.selection.last().copied();
				}
				return;
			}
			if let Some(old) = self.selected {
				if extend && !self.selection.contains(&old) {
					self.selection.push(old);
				}
			}
			if !self.selection.contains(&i) {
				self.selection.push(i);
			}
		}
		self.selected = hit;
	}
	fn capture_scale(&mut self) {
		self.scale_original.resize(self.models.len(), None);
		for n in 0..self.models.len() {
			if self.is_selected(n) && self.scale_original[n].is_none() {
				self.scale_original[n] = Some(self.models[n].clone());
			}
		}
	}
	fn transform_selection(&mut self, index: usize, before: &Model) {
		let after = self.models[index].clone();
		let old_center = before.center();
		let new_center = after.center();
		let (old_lo, old_hi) = before.bounds();
		let (new_lo, new_hi) = after.bounds();
		for n in 0..self.models.len() {
			if n == index || !self.is_selected(n) {
				continue;
			}
			let m = &mut self.models[n];
			let center = m.center();
			match self.tool {
				Tool::Scale => {
					if self.uniform {
						m.scale *= after.scale / before.scale;
					} else {
						for a in 0..3 {
							m.resize_axis(
								a,
								(new_hi[a] - new_lo[a]) / (old_hi[a] - old_lo[a]).max(0.001),
								false,
							);
						}
					}
					m.keep_center(center);
				}
				Tool::Rotate => {
					for a in 0..3 {
						m.rotation[a] += after.rotation[a] - before.rotation[a];
					}
					m.keep_center(center);
				}
				_ => {
					for a in 0..3 {
						m.position[a] += new_center[a] - old_center[a];
					}
				}
			}
		}
	}
	fn delete_selected(&mut self) {
		if self.selected.is_some() {
			self.checkpoint();
			let mut indices: Vec<_> = (0..self.models.len()).filter(|&n| self.is_selected(n)).collect();
			indices.sort_unstable();
			for n in indices.into_iter().rev() {
				self.models.remove(n);
			}
			self.selection.clear();
			self.scale_original.clear();
			self.selected = None;
			self.status = "Selected model deleted".into();
		}
	}
	fn invalidate(&mut self) {
		self.slicing = None;
		self.playing = false;
		self.layers.clear();
		self.preview = false;
		self.code.clear();
	}
	fn import(&mut self) {
		if let Some(paths) = rfd::FileDialog::new()
			.add_filter(
				"3D models",
				&[
					"stl", "obj", "3mf", "dae", "3ds", "ply", "scn", "off", "png", "jpg", "jpeg", "bmp",
					"gcode",
				],
			)
			.pick_files()
		{
			self.checkpoint();
			for path in paths {
				self.load(&path);
			}
		}
	}
	fn load(&mut self, path: &std::path::Path) {
		if path
			.extension()
			.and_then(|e| e.to_str())
			.map_or(false, |e| e.eq_ignore_ascii_case("gcode"))
		{
			match std::fs::read_to_string(path)
				.map_err(|e| e.to_string())
				.and_then(|code| read_gcode(&code).map(|layers| (code, layers)))
			{
				Ok((code, layers)) => {
					self.code = code;
					self.layer = layers.len();
					self.layers = layers;
					self.preview = true;
					self.playing = false;
					self.status = "G-code loaded for simulation".into();
				}
				Err(error) => self.status = error,
			}
			return;
		}
		match Model::load(path) {
			Ok(m) => {
				self.status = format!("Imported {} · {} triangles", m.name, m.triangles.len());
				self.add_model(m);
			}
			Err(e) => self.status = e,
		}
	}
	fn add_model(&mut self, mut model: Model) {
		if model.find_free_position_for(&self.models, &self.settings.printer) {
			self.models.push(model);
			self.selection.clear();
			self.selected = Some(self.models.len() - 1);
		} else {
			self.status = "No free position on the plate. Move or remove an object first.".into();
		}
	}
	fn place_bottom(&mut self) {
		if self.selected.is_some() {
			self.checkpoint();
			for n in 0..self.models.len() {
				if self.is_selected(n) {
					self.models[n].ground();
				}
			}
			self.status = "Model bottom placed on the plate".into();
		}
	}
	fn start_slice(&mut self) {
		if self.slicing.is_some() {
			return;
		}
		self.invalidate();
		let models = self.models.clone();
		let settings = self.settings.clone();
		let (sender, receiver) = std::sync::mpsc::channel();
		self.slicing = Some(receiver);
		self.status = "Slicing…".into();
		std::thread::spawn(move || {
			let _ = sender.send(slice(&models, &settings));
		});
	}
	fn finish_slice(&mut self, result: Result<Vec<Layer>, String>) {
		match result {
			Ok(layers) => {
				self.code = gcode(&layers, &self.settings);
				self.playing = false;
				self.play_step = 0;
				self.layer = layers.len();
				self.layers = layers;
				self.preview = true;
				self.status = format!(
					"{} layers generated · Experimental paths: inspect before printing",
					self.layers.len()
				);
			}
			Err(e) => self.status = e,
		}
	}
	fn export(&mut self) {
		let printer = kebab_name(&self.settings.printer.name, "printer");
		let model = self
			.models
			.first()
			.map(|m| {
				let stem = std::path::Path::new(&m.name)
					.file_stem()
					.and_then(|s| s.to_str())
					.unwrap_or(&m.name);
				kebab_name(stem, "model")
			})
			.unwrap_or_else(|| "plate".into());
		if let Some(p) = rfd::FileDialog::new()
			.add_filter("G-code", &["gcode"])
			.set_file_name(format!("{printer}-{model}.gcode"))
			.save_file()
		{
			self.status = match std::fs::write(p, &self.code) {
				Ok(_) => "G-code exported · Verify paths and printer compatibility before use".into(),
				Err(e) => e.to_string(),
			};
		}
	}
	fn has_unsaved_changes(&self) -> bool {
		self.models != self.saved_models || self.settings != self.saved_settings
	}
	fn mark_saved(&mut self) {
		self.saved_models = self.models.clone();
		self.saved_settings = self.settings.clone();
	}
	fn save(&mut self) -> bool {
		if let Some(p) = rfd::FileDialog::new()
			.add_filter("3printer project", &["json"])
			.set_file_name("project.json")
			.save_file()
		{
			let models:Vec<_>=self.models.iter().map(|m|serde_json::json!({"name":m.name,"triangles":m.triangles,"position":m.position,"rotation":m.rotation,"scale":m.scale})).collect();
			let s = &self.settings;
			let value = serde_json::json!({"version":2,"models":models,"settings":s});
			match std::fs::write(p, value.to_string()) {
				Ok(_) => {
					self.mark_saved();
					self.status = "Project saved".into();
					return true;
				}
				Err(e) => self.status = format!("Could not save project: {e}"),
			}
		}
		false
	}
	fn open(&mut self) {
		if let Some(p) = rfd::FileDialog::new()
			.add_filter("3printer project", &["json"])
			.pick_file()
		{
			let result = (|| -> Result<(Vec<Model>, Settings), String> {
				let value: serde_json::Value =
					serde_json::from_slice(&std::fs::read(p).map_err(|e| e.to_string())?)
						.map_err(|e| e.to_string())?;
				let mut models = Vec::new();
				for v in value["models"].as_array().ok_or("Invalid project")? {
					let triangles: Vec<Triangle> =
						serde_json::from_value(v["triangles"].clone()).map_err(|e| e.to_string())?;
					if triangles.is_empty() || triangles.len() > 1_000_000 {
						return Err("Invalid project geometry".into());
					}
					models.push(Model {
						name: v["name"].as_str().unwrap_or("Model").into(),
						triangles,
						position: serde_json::from_value(v["position"].clone()).map_err(|e| e.to_string())?,
						rotation: serde_json::from_value(v["rotation"].clone()).map_err(|e| e.to_string())?,
						scale: v["scale"].as_f64().unwrap_or(1.) as f32,
					});
				}
				let s = Settings::from_json(&serde_json::to_vec(&value).map_err(|e| e.to_string())?)?;
				Ok((models, s))
			})();
			match result {
				Ok((m, s)) => {
					self.checkpoint();
					self.models = m;
					self.selection.clear();
					self.scale_original.clear();
					self.settings = s;
					self.selected = if self.models.is_empty() { None } else { Some(0) };
					self.mark_saved();
					self.status = "Project opened".into();
				}
				Err(e) => self.status = e,
			}
		}
	}
	fn menu_enabled(&self, action: menus::Action) -> bool {
		match action {
			menus::Action::Undo => !self.history.is_empty(),
			menus::Action::Clear => !self.models.is_empty(),
			_ => true,
		}
	}
	fn menu_action(&mut self, action: menus::Action) {
		if !self.menu_enabled(action) {
			return;
		}
		match action {
			menus::Action::Import => self.import(),
			menus::Action::Open => self.open(),
			menus::Action::Save => {
				self.save();
			}
			menus::Action::Undo => {
				self.models = self.history.pop().unwrap();
				self.selected = None;
				self.invalidate();
			}
			menus::Action::Clear => {
				self.checkpoint();
				self.models.clear();
				self.selected = None;
				self.invalidate();
			}
			menus::Action::Perspective => {
				self.yaw = -0.65;
				self.pitch = 0.8;
				self.zoom = 1.;
			}
			menus::Action::Top => {
				self.yaw = 0.;
				self.pitch = std::f32::consts::FRAC_PI_2;
			}
			menus::Action::Front => {
				self.yaw = 0.;
				self.pitch = 0.2;
			}
			menus::Action::Help => self.show_help = true,
			menus::Action::License => self.show_license = true,
		}
	}
	fn top(&mut self, ctx: &egui::Context) {
		egui::TopBottomPanel::top("menu")
			.exact_height(48.)
			.show(ctx, |ui| {
				ui.horizontal_centered(|ui| {
					icons::brand(ui);
					ui.label(RichText::new("3printer").strong().size(16.));
					ui.add_space(20.);
					ui.visuals_mut().widgets.inactive.fg_stroke.color = Color32::WHITE;
					for group in menus::GROUPS {
						ui.menu_button(group, |ui| {
							for action in menus::Action::ALL.iter().copied().filter(|a| a.group() == group) {
								let shortcut = action
									.shortcut()
									.map(|shortcut| ctx.format_shortcut(&shortcut))
									.unwrap_or_default();
								let enabled = self.menu_enabled(action);
								let response = ui
									.add_enabled_ui(enabled, |ui| {
										icons::menu_item(ui, action.icon(), action.label(), &shortcut)
									})
									.inner;
								if response.clicked() {
									self.menu_action(action);
									ui.close_menu();
								}
							}
						});
					}
					ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
						let name = self.settings.printer.name.clone();
						ui.menu_button(format!("Printer: {name}"), |ui| {
							for i in 0..self.printers.len() {
								if ui
									.selectable_label(
										self.settings.printer == self.printers[i],
										&self.printers[i].name,
									)
									.clicked()
								{
									self.settings.printer = self.printers[i].clone();
									self.settings.nozzle = self.settings.printer.nozzle;
									self.invalidate();
									ui.close_menu();
								}
							}
							ui.separator();
							if ui.button("Manage printers").clicked() {
								self.printer_draft = self.settings.printer.clone();
								self.printer_index =
									self.printers.iter().position(|p| p == &self.settings.printer);
								self.show_printers = true;
								ui.close_menu();
							}
						});
					});
				});
			});
		egui::TopBottomPanel::bottom("status")
			.exact_height(32.)
			.show(ctx, |ui| {
				ui.horizontal_centered(|ui| {
					icons::icon(ui, "check", BLUE);
					ui.label(RichText::new(&self.status).size(12.).color(MUTED));
				});
			});
	}
	fn sidebar(&mut self, ctx: &egui::Context) {
		egui::SidePanel::right("settings")
			.exact_width(300.)
			.frame(
				egui::Frame::new()
					.fill(Color32::from_rgb(27, 30, 35))
					.inner_margin(18.),
			)
			.resizable(false)
			.show(ctx, |ui| {
				if self.preview {
					ui.separator();
					ui.heading("Print report");
					let report = print_report(&self.layers, &self.settings);
					let seconds = report.seconds.ceil() as u64;
					ui.label(format!(
						"Printing time: {}h {}m {}s",
						seconds / 3600,
						seconds / 60 % 60,
						seconds % 60
					));
					ui.label(format!("Material weight: {:.2} g", report.grams));
					ui.label(format!("Material length: {:.2} m", report.meters));
					ui.label(
						RichText::new("Estimates exclude heating and acceleration.")
							.small()
							.color(MUTED),
					);
					ui.separator();
					ui.heading("Slice preview");
					ui.label(format!(
						"{} layers · {:.2} mm",
						self.layers.len(),
						self.settings.height
					));
					ui.label(
						RichText::new("Review the toolpaths before exporting. G-code is experimental.")
							.color(MUTED)
							.small(),
					);
					if ui
						.add_sized(
							[ui.available_width(), 38.],
							egui::Button::new(RichText::new("Export G-code").color(Color32::WHITE))
								.fill(BUTTON_BLUE),
						)
						.clicked()
					{
						self.export();
					}
					if ui
						.add_sized(
							[ui.available_width(), 38.],
							egui::Button::new("Back to preparation"),
						)
						.clicked()
					{
						self.preview = false;
						self.playing = false;
					}
				} else if ui
					.add_sized(
						[ui.available_width(), 38.],
						egui::Button::new(
							RichText::new(if self.slicing.is_some() {
								"Slicing…"
							} else {
								"Start slicing"
							})
							.color(Color32::WHITE),
						)
						.fill(BUTTON_BLUE),
					)
					.clicked()
				{
					self.start_slice();
				}
				ui.separator();
				egui::ScrollArea::vertical().show(ui, |ui| {
					ui.spacing_mut().item_spacing = Vec2::new(8., 10.);
					let panel_width = ui.available_width();
					let action_size = Vec2::new((panel_width - 8.) / 2., 36.);
					if let Some(i) = self.selected.filter(|i| *i < self.models.len()) {
						ui.heading("Transform");
						ui.label(
							RichText::new("Double-click a value to type it.")
								.small()
								.color(MUTED),
						);
						let field_size = Vec2::new(152., 30.);
						let button_size = Vec2::new((ui.available_width() - 16.) / 3., 32.);
						if self.tool == Tool::Scale {
							self.capture_scale();
						}
						let previous = self.models[i].clone();
						let before = (
							self.models[i].position,
							self.models[i].rotation,
							self.models[i].scale,
						);
						let m = &mut self.models[i];
						match self.tool {
							Tool::Scale => {
								ui.checkbox(&mut self.uniform, "Uniform scale");
								let (lo, hi) = m.bounds();
								for a in 0..3 {
									ui.horizontal(|ui| {
										ui.add_sized([24., 30.], egui::Label::new(["X", "Y", "Z"][a]));
										let mut size = hi[a] - lo[a];
										if ui
											.add_sized(
												field_size,
												egui::DragValue::new(&mut size)
													.range(0.01..=200.)
													.suffix(" mm"),
											)
											.changed()
										{
											m.resize_axis(a, size / (hi[a] - lo[a]).max(0.001), self.uniform);
										}
									});
								}
								let mut percent = m.scale * 100.;
								if ui
									.add_sized(
										field_size,
										egui::DragValue::new(&mut percent)
											.range(1.0..=500.)
											.prefix("Scale ")
											.suffix(" %"),
									)
									.changed()
								{
									m.scale = percent / 100.;
								}
							}
							Tool::Rotate => {
								ui.horizontal(|ui| {
									ui.label("Drag axis");
									for (axis, label) in ["X", "Y", "Z"].iter().enumerate() {
										ui.selectable_value(&mut self.rotation_axis, axis, *label);
									}
								});
								for (a, label) in ["X", "Y", "Z"].iter().enumerate() {
									ui.horizontal(|ui| {
										ui.add_sized([24., 30.], egui::Label::new(*label));
										if ui.add_sized([44., 30.], egui::Button::new("−45°")).clicked() {
											m.rotation[a] -= 45.;
										}
										ui.add_sized(
											[104., 30.],
											egui::DragValue::new(&mut m.rotation[a]).speed(1.).suffix("°"),
										);
										if ui.add_sized([44., 30.], egui::Button::new("+45°")).clicked() {
											m.rotation[a] += 45.;
										}
									});
								}
							}
							_ => {
								for (a, label) in ["X", "Y", "Z"].iter().enumerate() {
									ui.horizontal(|ui| {
										ui.add_sized([24., 30.], egui::Label::new(*label));
										ui.add_sized(
											field_size,
											egui::DragValue::new(&mut m.position[a])
												.speed(0.5)
												.max_decimals(1)
												.suffix(" mm"),
										);
									});
								}
							}
						}
						if before != (m.position, m.rotation, m.scale) || previous.triangles != m.triangles {
							if self.tool == Tool::Rotate || self.tool == Tool::Scale {
								m.keep_center(previous.center());
							}
							let mut snapshot = self.models.clone();
							snapshot[i] = previous.clone();
							self.transform_selection(i, &previous);
							self.history.push(snapshot);
							if self.history.len() > 25 {
								self.history.remove(0);
							}
							self.invalidate();
						}
						ui.horizontal(|ui| {
							if ui.add_sized(button_size, egui::Button::new("Center")).clicked() {
								self.checkpoint();
								let c = self.models[i].center();
								for n in 0..self.models.len() {
									if self.is_selected(n) {
										self.models[n].position[0] +=
											self.settings.printer.size[0] / 2. - c[0];
										self.models[n].position[1] +=
											self.settings.printer.size[1] / 2. - c[1];
									}
								}
							}
							if ui.add_sized(button_size, egui::Button::new("Reset")).clicked() {
								self.checkpoint();
								for n in 0..self.models.len() {
									if !self.is_selected(n) {
										continue;
									}
									let center = self.models[n].center();
									match self.tool {
										Tool::Rotate => self.models[n].rotation = [0.; 3],
										Tool::Scale => {
											if let Some(Some(original)) = self.scale_original.get(n) {
												self.models[n].triangles = original.triangles.clone();
												self.models[n].scale = original.scale;
												self.models[n].rotation = original.rotation;
											}
										}
										_ => {
											self.models[n].position = [0.; 3];
											self.models[n].ground();
										}
									}
									if matches!(self.tool, Tool::Rotate | Tool::Scale) {
										self.models[n].keep_center(center);
									}
								}
							}
							if ui
								.add_sized(button_size, egui::Button::new("P Bottom"))
								.on_hover_text("Place the lowest point of the object on the plate")
								.clicked()
							{
								self.checkpoint();
								for n in 0..self.models.len() {
									if self.is_selected(n) {
										self.models[n].ground();
									}
								}
							}
						});
					}
					ui.separator();
					ui.horizontal(|ui| {
						icons::icon(ui, "settings", BLUE);
						ui.label(RichText::new("Print setup").size(19.).strong());
					});
					ui.add_space(8.);
					ui.label(RichText::new("PRINTER").size(11.).color(MUTED));
					ui.strong(&self.settings.printer.name);
					ui.label(
						RichText::new("Arrange models, choose a print profile, then slice to preview.")
							.small()
							.color(MUTED),
					);
					ui.label(format!(
						"{} × {} × {} mm build volume",
						self.settings.printer.size[0],
						self.settings.printer.size[1],
						self.settings.printer.size[2]
					));
					ui.label(
						RichText::new(format!(
							"{:.2} mm nozzle / heated build plate",
							self.settings.nozzle
						))
						.small()
						.color(MUTED),
					);
					ui.add_space(10.);
					ui.separator();
					ui.horizontal(|ui| {
						ui.heading("Models");
						ui.label(RichText::new(format!("{:02}", self.models.len())).color(MUTED));
					});
					egui::Frame::new()
						.fill(Color32::from_rgb(31, 34, 39))
						.inner_margin(12.)
						.corner_radius(10.)
						.stroke(Stroke::new(1., Color32::from_rgb(57, 63, 72)))
						.show(ui, |ui| {
							ui.set_min_height(82.);
							ui.set_min_width((panel_width - 26.).max(0.));
							ui.set_max_width((panel_width - 26.).max(0.));
							if self.models.is_empty() {
								ui.label(RichText::new("Your build plate is empty").color(MUTED));
							}
							egui::ScrollArea::vertical().max_height(135.).show(ui, |ui| {
								for i in 0..self.models.len() {
									let m = &self.models[i];
									if ui
										.add_sized(
											[ui.available_width(), 32.],
											egui::Button::new(&m.name)
												.selected(self.is_selected(i))
												.truncate(),
										)
										.clicked()
									{
										self.select_model(
											Some(i),
											ui.input(|input| {
												input.modifiers.shift || input.modifiers.command
											}),
										);
									}
								}
							});
						});
					ui.horizontal(|ui| {
						if ui
							.add_sized(action_size, egui::Button::new("Import model"))
							.clicked()
						{
							self.import();
						}
						if ui.add_sized(action_size, egui::Button::new("Add cube")).clicked() {
							self.checkpoint();
							self.add_model(Model::cube());
						}
					});
					ui.horizontal(|ui| {
						ui.label("Number of copies");
						ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
							ui.add_sized(
								action_size,
								egui::DragValue::new(&mut self.clone_count).range(1..=1000),
							);
						});
					});
					ui.horizontal(|ui| {
						if ui
							.add_enabled(
								self.selected.is_some(),
								egui::Button::new("Duplicate").min_size(action_size),
							)
							.clicked()
						{
							self.checkpoint();
							let sources: Vec<_> = self
								.models
								.iter()
								.enumerate()
								.filter(|(i, _)| self.is_selected(*i))
								.map(|(_, m)| m.clone())
								.collect();
							'copies: for _ in 0..self.clone_count {
								for source in &sources {
									let mut copy = source.clone();
									copy.name += " copy";
									if !copy.find_free_position_for(&self.models, &self.settings.printer) {
										self.status =
											"Plate full · added only copies that fit in free space".into();
										break 'copies;
									}
									self.models.push(copy);
								}
							}
						}
						if ui
							.add_enabled(
								self.selected.is_some(),
								egui::Button::new("Delete").min_size(action_size),
							)
							.clicked()
						{
							self.delete_selected();
						}
					});
					ui.add_space(8.);
					ui.separator();
					ui.heading("Print profile");
					ui.label(
						RichText::new(
							"Choose a starting layer-height preset, then fine-tune the values below.",
						)
						.small()
						.color(MUTED),
					);
					for (i, label) in ["High precision", "Normal printing", "Fast printing"]
						.iter()
						.enumerate()
					{
						if ui
							.add_sized(
								[ui.available_width(), 36.],
								egui::Button::new(format!(
									"{}    ·    {} mm",
									label,
									["0.10", "0.15", "0.20"][i]
								))
								.selected(self.profile == i),
							)
							.clicked()
						{
							self.profile = i;
							self.settings.height = [0.1, 0.15, 0.2][i];
							self.settings.solid_layers = [6, 4, 3][i];
							self.invalidate();
						}
					}
					ui.add_space(8.);
					ui.label(RichText::new("MATERIAL").size(11.).color(MUTED));
					ui.label(
						RichText::new("Presets update temperature, cooling, speed, and density defaults.")
							.small()
							.color(MUTED),
					);
					egui::ComboBox::from_id_salt("material")
						.selected_text(&self.settings.material)
						.width(panel_width)
						.show_ui(ui, |ui| {
							for (name, temperature, bed, fan, speed) in [
								("PLA", 200., 50., 100., 50.),
								("PETG", 235., 80., 40., 40.),
								("ABS", 250., 100., 0., 45.),
								("TPU", 220., 50., 30., 25.),
							] {
								if ui
									.selectable_label(self.settings.material == name, name)
									.clicked()
								{
									self.settings.material = name.into();
									self.settings.density = match name {
										"PETG" => 1.27,
										"ABS" => 1.04,
										"TPU" => 1.21,
										_ => 1.24,
									};
									self.settings.temperature = temperature;
									self.settings.bed = bed;
									self.settings.fan = fan;
									self.settings.speed = speed;
									self.invalidate();
								}
							}
						});
					ui.horizontal(|ui| {
						ui.label(RichText::new("Layer height").color(MUTED));
						ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
							ui.label(format!("{:.2} mm", self.settings.height));
						});
					});
					ui.horizontal(|ui| {
						ui.label(RichText::new("Infill").color(MUTED));
						ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
							ui.label(format!("{:.0}%", self.settings.infill));
						});
					});
					if ui
						.add_sized(
							[ui.available_width(), 36.],
							egui::Button::new("Edit configuration"),
						)
						.clicked()
					{
						self.show_settings = true;
					}
					ui.collapsing("Supports", |ui| {
						ui.add_sized(
							[120., 24.],
							egui::DragValue::new(&mut self.support_grid)
								.range(2.0..=20.)
								.prefix("Grid ")
								.suffix(" mm"),
						);
						ui.add(
							egui::DragValue::new(&mut self.support_angle)
								.range(10.0..=85.)
								.prefix("Max angle ")
								.suffix("°"),
						);
						if ui
							.add_enabled(self.selected.is_some(), egui::Button::new("Automatic Support"))
							.clicked()
						{
							if let Some(i) = self.selected {
								let supports = self.models[i].supports(self.support_grid, self.support_angle);
								self.checkpoint();
								self.status = format!("Added {} support pillars", supports.len());
								self.models.extend(supports);
							}
						}
						if ui.button("Add support pillar").clicked() {
							if let Some(i) = self.selected {
								let (lo, hi) = self.models[i].bounds();
								let support = Model::pillar(
									[(lo[0] + hi[0]) / 2., (lo[1] + hi[1]) / 2.],
									lo[2].max(2.),
									1.2,
								);
								self.checkpoint();
								self.models.push(support);
								self.selected = Some(self.models.len() - 1);
								self.tool = Tool::Move;
							}
						}
						ui.label("Select a pillar to move, resize or delete it.");
						if ui.button("Clear All Supports").clicked() {
							self.checkpoint();
							self.models.retain(|m| !m.name.starts_with("Support pillar"));
							self.selected = None;
						}
					});
					ui.add_space(12.);
				});
			});
	}
	fn canvas(&mut self, ctx: &egui::Context) {
		egui::CentralPanel::default()
			.frame(egui::Frame::new().fill(Color32::from_rgb(48, 51, 57)))
			.show(ctx, |ui| {
				ui.horizontal(|ui| {
					ui.selectable_value(&mut self.preview, false, "Prepare");
					if ui
						.add_enabled(
							!self.layers.is_empty(),
							egui::Button::new("Layer preview").selected(self.preview),
						)
						.clicked()
					{
						self.preview = true;
					}
					ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
						ui.label(RichText::new("BUILD PLATE  01").size(11.).color(MUTED));
					});
				});
				ui.separator();
				let rect = ui.available_rect_before_wrap();
				let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());
				response.clone().on_hover_cursor(egui::CursorIcon::Grab);
				if response.dragged_by(egui::PointerButton::Secondary) {
					ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
				}
				if response.dragged_by(egui::PointerButton::Secondary) {
					let d = ctx.input(|i| i.pointer.delta());
					self.yaw -= d.x * 0.008;
					self.pitch = (self.pitch + d.y * 0.008).clamp(0.15, 1.57);
				}
				if response.hovered() {
					self.zoom =
						(self.zoom * ctx.input(|i| (i.smooth_scroll_delta.y * 0.002).exp())).clamp(0.3, 3.);
				}
				let painter = ui.painter_at(rect);
				let center = Pos2::new(rect.center().x, rect.center().y + 65.);
				let [width, depth, height] = self.settings.printer.size;
				let scale =
					(rect.width().min(rect.height()) / (width.max(depth).max(height) * 1.4)) * self.zoom;
				let (sy, cy) = self.yaw.sin_cos();
				let (sp, cp) = self.pitch.sin_cos();
				let project = |v: V3| {
					let x = v[0] - width / 2.;
					let y = v[1] - depth / 2.;
					Pos2::new(
						center.x + (x * cy - y * sy) * scale,
						center.y + ((x * sy + y * cy) * sp - v[2] * cp) * scale,
					)
				};
				if !self.preview {
					// Pick the frontmost projected triangle, matching the painter's depth order.
					if response.clicked() || response.drag_started_by(egui::PointerButton::Primary) {
						let pointer = if response.drag_started() {
							ctx.input(|i| i.pointer.press_origin())
						} else {
							response.interact_pointer_pos()
						};
						if let Some(pointer) = pointer {
							let mut hit = None;
							let mut nearest = f32::NEG_INFINITY;
							for (index, model) in self.models.iter().enumerate() {
								for tri in model.transformed() {
									let points = tri.map(project);
									if triangle_contains(points, pointer) {
										let depth = tri
											.iter()
											.map(|v| {
												((v[0] - width / 2.) * sy + (v[1] - depth / 2.) * cy) * cp
													+ v[2] * sp
											})
											.sum::<f32>();
										if depth > nearest {
											nearest = depth;
											hit = Some(index);
										}
									}
								}
							}
							self.drag_axis = None;
							if let Some(i) = self.selected.filter(|i| *i < self.models.len()) {
								let pivot = self.models[i].center();
								if matches!(self.tool, Tool::Move | Tool::Scale | Tool::Rotate) {
									for axis in 0..3 {
										let mut end = pivot;
										end[axis] += 35.;
										let ring_hit = self.tool == Tool::Rotate
											&& (0..128).any(|n| {
												let t = n as f32 * std::f32::consts::TAU / 128.;
												let mut v = pivot;
												v[(axis + 1) % 3] += 28. * t.cos();
												v[(axis + 2) % 3] += 28. * t.sin();
												project(v).distance(pointer) < 8.
											});
										if ring_hit
											|| (self.tool != Tool::Rotate
												&& project(end).distance(pointer) < 18.)
										{
											hit = Some(i);
											self.drag_axis = Some(axis);
											self.rotation_axis = axis;
										}
									}
								}
							}
							if self.drag_axis.is_none()
								&& !(response.drag_started() && hit.is_some_and(|i| self.is_selected(i)))
							{
								self.select_model(
									hit,
									ctx.input(|i| i.modifiers.shift || i.modifiers.command),
								);
							}
							if self.tool == Tool::PlaceBottom && response.clicked() {
								self.place_bottom();
							}
							self.drag_model = hit;
							if hit.is_some() && self.tool != Tool::Select && response.drag_started() {
								self.checkpoint();
							}
						}
					}
					if response.dragged_by(egui::PointerButton::Primary) && self.tool != Tool::Select {
						if let Some(index) = self.drag_model.filter(|i| *i < self.models.len()) {
							let delta = ctx.input(|i| i.pointer.delta());
							if self.tool == Tool::Scale {
								self.capture_scale();
							}
							let previous = self.models[index].clone();
							let model = &mut self.models[index];
							let pivot = model.center();
							match self.tool {
								Tool::Move => {
									if let Some(axis) = self.drag_axis {
										let start = project(pivot);
										let mut end = pivot;
										end[axis] += 1.;
										let direction = project(end) - start;
										model.position[axis] +=
											delta.dot(direction) / direction.length_sq().max(0.01);
									} else if ctx.input(|i| i.modifiers.shift) {
										model.position[2] -= delta.y / (scale * cp.max(0.15));
									} else {
										let x = delta.x / scale;
										let y = delta.y / (scale * sp);
										model.position[0] += x * cy + y * sy;
										model.position[1] += -x * sy + y * cy;
									}
								}
								Tool::Scale => {
									let axis = self.drag_axis.unwrap_or(0);
									let mut end = pivot;
									end[axis] += 1.;
									let direction = project(end) - project(pivot);
									let amount = if self.drag_axis.is_some() {
										delta.dot(direction) / direction.length_sq().max(0.01) / 35.
									} else {
										(delta.x - delta.y) * 0.01
									};
									model.resize_axis(axis, amount.exp().clamp(0.5, 2.), self.uniform);
								}
								Tool::Rotate => {
									model.rotation[self.rotation_axis] += delta.x - delta.y;
								}
								Tool::Select | Tool::PlaceBottom => {}
							}
							if self.tool == Tool::Rotate || self.tool == Tool::Scale {
								model.keep_center(pivot);
							}
							self.transform_selection(index, &previous);
							self.invalidate();
							ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
						}
					}
					if response.drag_stopped_by(egui::PointerButton::Primary) {
						self.drag_model = None;
					}
					painter.text(
						rect.left_top() + Vec2::new(12., 10.),
						egui::Align2::LEFT_TOP,
						match self.tool {
							Tool::Select => {
								"Click to select · Shift / Command + click to select multiple models"
							}
							Tool::PlaceBottom => "Click a model to place its bottom on the plate",
							Tool::Move => "Drag a model to move · Shift + drag changes height",
							Tool::Scale => "Drag up / right to enlarge · Down / left to shrink",
							Tool::Rotate => "Drag to rotate · Choose X, Y or Z in Transform",
						},
						egui::FontId::proportional(12.),
						MUTED,
					);
				}
				let plate = [[0., 0., 0.], [width, 0., 0.], [width, depth, 0.], [0., depth, 0.]].map(project);
				painter.add(egui::Shape::convex_polygon(
					if self.settings.printer.circular_bed {
						(0..96)
							.map(|i| {
								let angle = i as f32 * std::f32::consts::TAU / 96.;
								project([
									width / 2. + width / 2. * angle.cos(),
									depth / 2. + depth / 2. * angle.sin(),
									0.,
								])
							})
							.collect()
					} else {
						plate.to_vec()
					},
					Color32::from_rgb(59, 64, 70),
					Stroke::new(1.5, Color32::from_rgb(90, 100, 111)),
				));
				for axis in 0..2 {
					let limit = if axis == 0 { width } else { depth };
					for i in 0..=(limit / 10.) as usize {
						let p = i as f32 * 10.;
						let (mut a, mut b) = if axis == 0 {
							([p, 0., 0.], [p, depth, 0.])
						} else {
							([0., p, 0.], [width, p, 0.])
						};
						if self.settings.printer.circular_bed {
							let half = if axis == 0 { depth / 2. } else { width / 2. };
							let span = half * (1. - ((p - limit / 2.) / (limit / 2.)).powi(2)).max(0.).sqrt();
							a[1 - axis] = half - span;
							b[1 - axis] = half + span;
						}
						painter
							.line_segment([project(a), project(b)], Stroke::new(0.7, Color32::from_gray(85)));
					}
				}
				for (a, b) in [(0, 1), (1, 2), (2, 3), (3, 0)] {
					let corners = [
						[0., 0., height],
						[width, 0., height],
						[width, depth, height],
						[0., depth, height],
					];
					painter.line_segment(
						[project(corners[a]), project(corners[b])],
						Stroke::new(0.6, Color32::from_rgb(69, 75, 83)),
					);
					painter.line_segment(
						[plate[a], project(corners[a])],
						Stroke::new(0.6, Color32::from_rgb(69, 75, 83)),
					);
				}
				if self.preview {
					for l in self.layers.iter().take(self.layer) {
						let current = (l.z - self.layers[self.layer.saturating_sub(1)].z).abs() < 0.001;
						let stride = if current { 1 } else { 4 };
						for (paths, color) in [
							(
								&l.walls,
								if current {
									Color32::from_rgb(255, 186, 77)
								} else {
									Color32::from_rgb(30, 113, 151)
								},
							),
							(&l.infill, Color32::from_rgb(70, 178, 182)),
						] {
							for [a, b] in paths
								.iter()
								.take(if self.playing && current {
									self.play_step.saturating_sub(if std::ptr::eq(paths, &l.infill) {
										l.walls.len()
									} else {
										0
									})
								} else {
									paths.len()
								})
								.step_by(stride)
							{
								painter.line_segment(
									[project([a[0], a[1], l.z]), project([b[0], b[1], l.z])],
									Stroke::new(if current { 1.5 } else { 0.5 }, color),
								);
							}
						}
					}
					if self.playing {
						let l = &self.layers[self.layer - 1];
						if let Some([_, b]) = l
							.walls
							.iter()
							.chain(l.infill.iter())
							.nth(self.play_step.saturating_sub(1))
						{
							painter.circle_filled(project([b[0], b[1], l.z]), 4., Color32::WHITE);
						}
					}
				} else {
					let mut faces = Vec::new();
					for (i, m) in self.models.iter().enumerate() {
						for tri in m.transformed().into_iter() {
							let depth = tri
								.iter()
								.map(|v| {
									((v[0] - width / 2.) * sy + (v[1] - depth / 2.) * cy) * cp + v[2] * sp
								})
								.sum::<f32>();
							faces.push((depth, i, tri));
						}
					}
					faces.sort_by(|a, b| a.0.total_cmp(&b.0));
					for (_, i, tri) in faces {
						let a = tri[0];
						let b = tri[1];
						let c = tri[2];
						let normal = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
						let shade = if normal > 0. { 1.15 } else { 0.82 };
						let color = Color32::from_rgb(
							(32. * shade) as u8,
							(140. * shade) as u8,
							(192. * shade) as u8,
						);
						// Explicit triangles avoid convex-path stroke/AA extrusion on thin faces.
						// Mesh rendering accepts either winding and preserves every imported face.
						let mut mesh = egui::Mesh::default();
						let color = if self.is_selected(i) {
							color.gamma_multiply(1.12)
						} else {
							color
						};
						for pos in tri.map(project) {
							mesh.colored_vertex(pos, color);
						}
						mesh.add_triangle(0, 1, 2);
						painter.add(egui::Shape::mesh(mesh));
					}
				}
				if !self.preview {
					if let Some(i) = self.selected.filter(|i| *i < self.models.len()) {
						let (lo, hi) = self.models[i].bounds();
						for index in 0..self.models.len() {
							if !self.is_selected(index) {
								continue;
							}
							let (lo, hi) = self.models[index].bounds();
							for corner in 0..8 {
								let v: V3 =
									std::array::from_fn(
										|a| if corner & (1 << a) == 0 { lo[a] } else { hi[a] },
									);
								for axis in 0..3 {
									let mut end = v;
									let extent = hi[axis] - lo[axis];
									end[axis] += if corner & (1 << axis) == 0 {
										extent * 0.22
									} else {
										-extent * 0.22
									};
									painter.line_segment(
										[project(v), project(end)],
										Stroke::new(2., Color32::from_rgb(65, 220, 240)),
									);
								}
							}
						}
						let pivot = [(lo[0] + hi[0]) / 2., (lo[1] + hi[1]) / 2., (lo[2] + hi[2]) / 2.];
						for a in 0..3 {
							let color = [Color32::LIGHT_RED, Color32::LIGHT_GREEN, Color32::LIGHT_BLUE][a];
							if self.tool == Tool::Rotate {
								let points: Vec<_> = (0..=64)
									.map(|n| {
										let t = n as f32 * std::f32::consts::TAU / 64.;
										let mut v = pivot;
										v[(a + 1) % 3] += 28. * t.cos();
										v[(a + 2) % 3] += 28. * t.sin();
										project(v)
									})
									.collect();
								painter.add(egui::Shape::line(points, Stroke::new(2., color)));
							} else if matches!(self.tool, Tool::Move | Tool::Scale) {
								let mut end = pivot;
								end[a] += 35.;
								if self.tool == Tool::Scale {
									painter.rect_filled(
										egui::Rect::from_center_size(project(end), Vec2::splat(14.)),
										1.,
										color,
									);
								}
								painter.arrow(
									project(pivot),
									project(end) - project(pivot),
									Stroke::new(3., color),
								);
								painter.text(
									project(end),
									egui::Align2::CENTER_BOTTOM,
									["X", "Y", "Z"][a],
									egui::FontId::proportional(14.),
									color,
								);
							}
						}
					}
				}
				let machine_origin = if self.settings.printer.center_origin {
					[width / 2., depth / 2., 0.]
				} else {
					[0., 0., 0.]
				};
				let origin = project(machine_origin);
				for (v, color, label) in [
					([30., 0., 0.], Color32::from_rgb(237, 89, 94), "X"),
					([0., 30., 0.], Color32::from_rgb(108, 198, 109), "Y"),
					([0., 0., 30.], Color32::from_rgb(77, 143, 255), "Z"),
				] {
					let p = project([v[0] + machine_origin[0], v[1] + machine_origin[1], v[2]]);
					painter.line_segment([origin, p], Stroke::new(2., color));
					painter.text(
						p,
						egui::Align2::CENTER_BOTTOM,
						label,
						egui::FontId::proportional(11.),
						color,
					);
				}
				painter.text(
					project([width / 2., depth + 9., 0.]),
					egui::Align2::CENTER_TOP,
					format!("{width} × {depth} mm"),
					egui::FontId::proportional(12.),
					MUTED,
				);

				if self.models.is_empty() && !self.preview {
					painter.text(
						rect.center() - Vec2::new(0., 80.),
						egui::Align2::CENTER_CENTER,
						"Your next print starts here",
						egui::FontId::proportional(25.),
						Color32::from_rgb(206, 215, 225),
					);
					painter.text(
						rect.center() - Vec2::new(0., 45.),
						egui::Align2::CENTER_CENTER,
						"Import an STL, OBJ or 3MF, or add a calibration cube",
						egui::FontId::proportional(14.),
						MUTED,
					);
				}
				if self.preview {
					let r = egui::Rect::from_min_size(
						rect.left_bottom() + Vec2::new(75., -65.),
						Vec2::new((rect.width() - 100.).max(100.), 55.),
					);
					ui.scope_builder(egui::UiBuilder::new().max_rect(r), |ui| {
						egui::Frame::new()
							.fill(Color32::from_rgb(36, 40, 46))
							.inner_margin(8.)
							.show(ui, |ui| {
								ui.horizontal(|ui| {
									if ui
										.button(if self.playing { "Pause" } else { "Simulate" })
										.clicked()
									{
										if !self.playing && self.layer >= self.layers.len() {
											self.layer = 1;
											self.play_step = 0;
										}
										self.playing = !self.playing;
										self.play_clock = ctx.input(|i| i.time);
									}
									ui.add(
										egui::DragValue::new(&mut self.play_speed)
											.range(1.0..=2000.)
											.suffix(" paths/s"),
									);
									ui.label("Layer");
									if ui
										.add(egui::Slider::new(&mut self.layer, 1..=self.layers.len()))
										.changed()
									{
										self.play_step = 0;
									}
									ui.checkbox(&mut self.code_view, "G-code");
								});
							});
					});
				}
				painter.text(
					rect.right_top() + Vec2::new(-12., 18.),
					egui::Align2::RIGHT_TOP,
					"Drag right mouse to orbit\nScroll to zoom",
					egui::FontId::proportional(11.),
					MUTED,
				);
			});
	}
}
impl eframe::App for Studio {
	fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
		if ctx.input(|i| i.viewport().close_requested()) && !self.allow_close && self.has_unsaved_changes() {
			ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
			let answer = rfd::MessageDialog::new()
				.set_title("Save changes before exiting?")
				.set_description("Your project has unsaved changes. Save them before exiting?")
				.set_level(rfd::MessageLevel::Warning)
				.set_buttons(rfd::MessageButtons::YesNoCancel)
				.show();
			self.allow_close = match answer {
				rfd::MessageDialogResult::Yes => self.save(),
				rfd::MessageDialogResult::No => true,
				_ => false,
			};
			if self.allow_close {
				ctx.send_viewport_cmd(egui::ViewportCommand::Close);
			}
		}
		if let Some(receiver) = &self.slicing {
			match receiver.try_recv() {
				Ok(result) => {
					self.slicing = None;
					self.finish_slice(result);
				}
				Err(std::sync::mpsc::TryRecvError::Disconnected) => {
					self.slicing = None;
					self.status = "Slicing failed. Please try again.".into();
				}
				Err(_) => {
					ctx.request_repaint_after(std::time::Duration::from_millis(50));
				}
			}
		}
		self.frame += 1;
		if self.playing && self.preview && !self.layers.is_empty() {
			let now = ctx.input(|i| i.time);
			let steps = ((now - self.play_clock) * self.play_speed as f64) as usize;
			if steps > 0 {
				self.play_clock = now;
				self.play_step += steps;
				loop {
					let l = &self.layers[self.layer - 1];
					let count = l.walls.len() + l.infill.len();
					if self.play_step < count {
						break;
					}
					if self.layer == self.layers.len() {
						self.playing = false;
						break;
					}
					self.play_step -= count;
					self.layer += 1;
				}
			}
			ctx.request_repaint_after(std::time::Duration::from_millis(16));
		}
		#[cfg(target_os = "macos")]
		{
			for action in menus::native::drain() {
				self.menu_action(action);
			}
			menus::native::update_enabled(!self.history.is_empty(), !self.models.is_empty());
		}
		if !ctx.wants_keyboard_input() {
			let delete_selected = ctx.input_mut(|i| {
				if !i.modifiers.is_none() {
					return false;
				}
				let delete = i.consume_key(egui::Modifiers::NONE, egui::Key::Delete);
				let backspace = i.consume_key(egui::Modifiers::NONE, egui::Key::Backspace);
				delete || backspace
			});
			if delete_selected {
				self.delete_selected();
			}
			for action in menus::Action::ALL {
				if let Some(shortcut) = action.shortcut() {
					if ctx.input_mut(|i| i.consume_shortcut(&shortcut)) {
						self.menu_action(action);
					}
				}
			}
		}
		let dropped = ctx.input(|i| i.raw.dropped_files.clone());
		for file in dropped {
			if let Some(path) = file.path {
				self.checkpoint();
				self.load(&path);
			}
		}
		self.top(ctx);
		self.sidebar(ctx);
		self.actions(ctx);
		self.canvas(ctx);
		self.printer_window(ctx);
		let mut show = self.show_settings;
		egui::Window::new("Print configuration")
			.open(&mut show)
			.resizable(true)
			.default_width(480.)
			.show(ctx, |ui| {
				egui::ScrollArea::vertical().max_height(560.).show(ui, |ui| {
					let before = self.settings.clone();
					ui.label(
						RichText::new(format!(
							"{} · {} × {} × {} mm",
							self.settings.printer.name,
							self.settings.printer.size[0],
							self.settings.printer.size[1],
							self.settings.printer.size[2]
						))
						.color(MUTED),
					);
					for (title, fields) in [
						(
							"Quality & strength",
							vec![
								(
									"Layer height",
									"Vertical detail and print time.",
									&mut self.settings.height,
									0.04..=0.8,
									" mm",
								),
								(
									"Infill density",
									"Interior material percentage.",
									&mut self.settings.infill,
									0.0..=100.,
									" %",
								),
								(
									"Nozzle / line width",
									"Width of each deposited line.",
									&mut self.settings.nozzle,
									0.1..=2.,
									" mm",
								),
							],
						),
						(
							"Speed",
							vec![
								(
									"Wall speed",
									"Speed for outer contours.",
									&mut self.settings.wall_speed,
									5.0..=300.,
									" mm/s",
								),
								(
									"Infill speed",
									"Speed for interior lines.",
									&mut self.settings.infill_speed,
									5.0..=300.,
									" mm/s",
								),
								(
									"Small feature max length",
									"Threshold for slowing short paths.",
									&mut self.settings.small_length,
									0.0..=100.,
									" mm",
								),
								(
									"Small feature speed",
									"Speed multiplier for short paths.",
									&mut self.settings.small_speed,
									1.0..=100.,
									" %",
								),
								(
									"Print speed",
									"Overall extrusion speed limit.",
									&mut self.settings.speed,
									5.0..=300.,
									" mm/s",
								),
								(
									"Travel speed",
									"Speed when not extruding.",
									&mut self.settings.travel_speed,
									5.0..=300.,
									" mm/s",
								),
							],
						),
						(
							"Material & cooling",
							vec![
								(
									"Material density",
									"Filament mass used for flow calculations.",
									&mut self.settings.density,
									0.1..=10.,
									" g/cm³",
								),
								(
									"Wall flow",
									"Extrusion multiplier for walls.",
									&mut self.settings.wall_flow,
									0.0..=150.,
									" %",
								),
								(
									"Infill flow",
									"Extrusion multiplier for infill.",
									&mut self.settings.infill_flow,
									0.0..=150.,
									" %",
								),
								(
									"Nozzle temperature",
									"Target hotend temperature.",
									&mut self.settings.temperature,
									150.0..=300.,
									" °C",
								),
								(
									"Bed temperature",
									"Target build plate temperature.",
									&mut self.settings.bed,
									0.0..=120.,
									" °C",
								),
								(
									"Filament diameter",
									"Diameter used to calculate extrusion.",
									&mut self.settings.diameter,
									0.5..=4.,
									" mm",
								),
								(
									"Extrusion flow",
									"Global extrusion multiplier.",
									&mut self.settings.flow,
									50.0..=150.,
									" %",
								),
								(
									"Cooling fan",
									"Part cooling fan percentage.",
									&mut self.settings.fan,
									0.0..=100.,
									" %",
								),
							],
						),
					] {
						ui.separator();
						ui.strong(title);
						for (label, description, value, range, unit) in fields {
							ui.allocate_ui_with_layout(
								egui::Vec2::new(360., 46.),
								egui::Layout::left_to_right(egui::Align::Center),
								|ui| {
									ui.allocate_ui_with_layout(
										egui::Vec2::new(210., 46.),
										egui::Layout::top_down(egui::Align::Min),
										|ui| {
											ui.label(RichText::new(label).color(TITLE));
											ui.label(RichText::new(description).size(12.).color(MUTED));
										},
									);
									ui.allocate_ui_with_layout(
										egui::Vec2::new(120., 46.),
										egui::Layout::left_to_right(egui::Align::Center),
										|ui| {
											ui.add_sized(
												[120., 24.],
												egui::DragValue::new(value)
													.range(range)
													.speed(0.1)
													.suffix(unit),
											);
										},
									);
								},
							);
						}
					}
					ui.allocate_ui_with_layout(
						egui::Vec2::new(360., 46.),
						egui::Layout::left_to_right(egui::Align::Center),
						|ui| {
							ui.allocate_ui_with_layout(
								egui::Vec2::new(210., 46.),
								egui::Layout::top_down(egui::Align::Min),
								|ui| {
									ui.label(RichText::new("Top / bottom solid layers").color(TITLE));
									ui.label(
										RichText::new("Number of fully solid layers at each end.")
											.size(12.)
											.color(MUTED),
									);
								},
							);
							ui.allocate_ui_with_layout(
								egui::Vec2::new(120., 46.),
								egui::Layout::left_to_right(egui::Align::Center),
								|ui| {
									ui.add_sized(
										[120., 24.],
										egui::DragValue::new(&mut self.settings.solid_layers).range(0..=20),
									);
								},
							);
						},
					);
					ui.allocate_ui_with_layout(
						egui::Vec2::new(360., 46.),
						egui::Layout::left_to_right(egui::Align::Center),
						|ui| {
							let description = match self.settings.infill_pattern {
								InfillPattern::ZigZag => "Fast and economical.",
								InfillPattern::Lines => "Simple and material-efficient.",
								InfillPattern::Grid => "Stable square structure.",
								InfillPattern::Triangles => "Strong multi-direction structure.",
								InfillPattern::Gyroid => "Balanced strength in multiple directions.",
								InfillPattern::Cubic => "Lightweight 3D-style structure.",
								InfillPattern::Concentric => "Follows a simple repeated direction.",
								InfillPattern::Lightning => "Minimal material for low-stress interiors.",
							};
							ui.allocate_ui_with_layout(
								egui::Vec2::new(210., 46.),
								egui::Layout::top_down(egui::Align::Min),
								|ui| {
									ui.label(RichText::new("Infill pattern").color(TITLE));
									ui.label(RichText::new(description).size(12.).color(MUTED));
								},
							);
							ui.allocate_ui_with_layout(
								egui::Vec2::new(120., 46.),
								egui::Layout::left_to_right(egui::Align::Center),
								|ui| {
									egui::ComboBox::from_id_salt("infill-pattern")
										.width(120.)
										.selected_text(self.settings.infill_pattern.label())
										.show_ui(ui, |ui| {
											for pattern in InfillPattern::ALL {
												ui.selectable_value(
													&mut self.settings.infill_pattern,
													pattern,
													pattern.label(),
												);
											}
										});
								},
							);
						},
					);
					if before != self.settings {
						self.profile = 3;
						self.invalidate();
					}
					if let Err(error) = self.settings.validate() {
						ui.colored_label(Color32::LIGHT_RED, error);
					}
					ui.separator();
					ui.horizontal(|ui| {
						if ui.button("Import settings…").clicked() {
							self.import_settings();
						}
						if ui
							.add_enabled(
								self.settings.validate().is_ok(),
								egui::Button::new("Export settings…"),
							)
							.clicked()
						{
							self.export_settings();
						}
						if ui.button("Reset").clicked() {
							let printer = self.settings.printer.clone();
							self.settings = Settings::default();
							self.settings.nozzle = printer.nozzle;
							self.settings.printer = printer;
							self.profile = 1;
							self.invalidate();
						}
					});
				});
			});
		self.show_settings = show;
		egui::Window::new("About your workspace")
			.open(&mut self.show_help)
			.resizable(true)
			.show(ctx, |ui| {
				ui.heading("3printer");
				ui.label("A local desktop workspace for preparing models for 3D printing.");
				ui.separator();
				ui.strong("Workflow");
				ui.label("Import STL, OBJ, or 3MF models, then select, move, rotate, scale, clone, and center them on the build plate.");
				ui.label("Use Print configuration to choose a printer profile and adjust layer height, infill, speeds, temperatures, cooling, and support settings.");
				ui.label("Slice the plate to inspect layer paths and the experimental G-code preview, then export G-code for your printer.");
				ui.separator();
				ui.strong("Controls");
				ui.label("Right-drag orbits the view. The mouse wheel zooms. Drag files onto the window to import them.");
				ui.separator();
				ui.label("3printer includes a basic local slicer with single-wall contours, alternating line infill, and support pillars. Cloud services, network printing, model repair, and the original advanced slicing engine are not connected.");
				ui.colored_label(Color32::YELLOW, "G-code is experimental. Inspect toolpaths and verify printer compatibility before using it with hardware.");
			});
		if self.show_license {
			egui::Window::new("3printer License")
				.open(&mut self.show_license)
				.resizable(true)
				.default_size([620., 420.])
				.show(ctx, |ui| {
					ui.label("3printer is distributed under the CORE License.");
					ui.separator();
					egui::ScrollArea::vertical().show(ui, |ui| {
						ui.add(
							egui::Label::new(RichText::new(include_str!("../LICENSE")).monospace())
								.selectable(true),
						);
					});
				});
		}
		if self.code_view {
			egui::Window::new("G-code preview")
				.open(&mut self.code_view)
				.default_size([600., 450.])
				.show(ctx, |ui| {
					egui::ScrollArea::both().show(ui, |ui| {
						ui.add(egui::Label::new(RichText::new(&self.code).monospace()).selectable(true));
					});
				});
		}
		if self.screenshot.is_some() {
			if self.frame >= 120 && !self.requested {
				ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
				self.requested = true;
			}
			let captured = ctx.input(|i| {
				i.events.iter().find_map(|e| {
					if let egui::Event::Screenshot { image, .. } = e {
						Some(image.clone())
					} else {
						None
					}
				})
			});
			if let Some(img) = captured {
				let path = self.screenshot.as_ref().unwrap();
				let pixels: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
				if let Some(parent) = std::path::Path::new(path).parent() {
					let _ = std::fs::create_dir_all(parent);
				}
				match image::save_buffer(
					path,
					&pixels,
					img.size[0] as u32,
					img.size[1] as u32,
					image::ColorType::Rgba8,
				) {
					Ok(_) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
					Err(e) => {
						eprintln!("Screenshot failed: {e}");
						ctx.send_viewport_cmd(egui::ViewportCommand::Close);
					}
				}
			}
			ctx.request_repaint();
		}
	}
}
fn triangle_contains(points: [Pos2; 3], p: Pos2) -> bool {
	let cross = |a: Pos2, b: Pos2, c: Pos2| (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
	if cross(points[0], points[1], points[2]).abs() < 0.001 {
		return false;
	}
	let edges = [
		cross(points[0], points[1], p),
		cross(points[1], points[2], p),
		cross(points[2], points[0], p),
	];
	edges.iter().all(|v| *v >= 0.) || edges.iter().all(|v| *v <= 0.)
}

#[cfg(test)]
mod transform_tests {
	use super::*;

	#[test]
	fn picking_handles_both_windings_and_rejects_empty_space() {
		let points = [Pos2::new(0., 0.), Pos2::new(10., 0.), Pos2::new(0., 10.)];
		assert!(triangle_contains(points, Pos2::new(2., 2.)));
		assert!(triangle_contains(
			[points[2], points[1], points[0]],
			Pos2::new(2., 2.)
		));
		assert!(!triangle_contains(points, Pos2::new(9., 9.)));
		assert!(!triangle_contains([points[0]; 3], points[0]));
	}

	#[test]
	fn transforms_reach_geometry_and_sliced_output() {
		let mut model = Model::cube();
		model.scale = 2.;
		model.rotation[2] = 90.;
		model.position = [80., 70., 0.];
		let vertex = model.vertex([3., 4., 5.]);
		for (actual, expected) in vertex.into_iter().zip([72., 76., 10.]) {
			assert!((actual - expected).abs() < 0.001);
		}
		let layers = slice(&[model], &Settings::default()).unwrap();
		assert!(layers.len() > 260);
		assert!(layers
			.iter()
			.flat_map(|l| &l.walls)
			.flatten()
			.all(|p| p[0] >= 59.9 && p[0] <= 100.1 && p[1] >= 49.9 && p[1] <= 90.1));
	}
}

fn main() -> eframe::Result {
	let options = eframe::NativeOptions {
		viewport: egui::ViewportBuilder::default()
			.with_inner_size([1280., 800.])
			.with_min_inner_size([1000., 720.])
			.with_title("3printer")
			.with_icon(icons::app_icon()),
		..Default::default()
	};
	eframe::run_native("3printer", options, Box::new(|cc| Ok(Box::new(Studio::new(cc)))))
}
