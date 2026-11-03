use eframe::egui::{Key, KeyboardShortcut, Modifiers};

pub const GROUPS: [&str; 4] = ["File", "Edit", "View", "Help"];
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(usize)]
pub enum Action {
	Import,
	Open,
	Save,
	Undo,
	Clear,
	Perspective,
	Top,
	Front,
	Help,
	License,
}
impl Action {
	pub const ALL: [Self; 10] = [
		Self::Import,
		Self::Open,
		Self::Save,
		Self::Undo,
		Self::Clear,
		Self::Perspective,
		Self::Top,
		Self::Front,
		Self::Help,
		Self::License,
	];
	pub fn group(self) -> &'static str {
		match self {
			Self::Import | Self::Open | Self::Save => "File",
			Self::Undo | Self::Clear => "Edit",
			Self::Help | Self::License => "Help",
			_ => "View",
		}
	}
	pub fn label(self) -> &'static str {
		match self {
			Self::Import => "Import model",
			Self::Open => "Open project",
			Self::Save => "Save project",
			Self::Undo => "Undo",
			Self::Clear => "Clear plate",
			Self::Perspective => "Perspective",
			Self::Top => "Top",
			Self::Front => "Front",
			Self::Help => "Workspace help",
			Self::License => "License",
		}
	}
	pub fn icon(self) -> &'static str {
		match self {
			Self::Import => "import",
			Self::Open => "folder",
			Self::Save => "save",
			Self::Undo => "rotate",
			Self::Clear => "trash",
			Self::Perspective => "cube",
			Self::Top => "top",
			Self::Front => "front",
			Self::Help => "help",
			Self::License => "license",
		}
	}
	pub fn shortcut(self) -> Option<KeyboardShortcut> {
		if self == Self::License {
			return None;
		}
		Some(KeyboardShortcut::new(
			if self == Self::Clear {
				Modifiers::COMMAND | Modifiers::SHIFT
			} else {
				Modifiers::COMMAND
			},
			match self {
				Self::Import => Key::I,
				Self::Open => Key::O,
				Self::Save => Key::S,
				Self::Undo => Key::Z,
				Self::Clear => Key::Backspace,
				Self::Perspective => Key::Num1,
				Self::Top => Key::Num2,
				Self::Front => Key::Num3,
				Self::Help => Key::Slash,
				Self::License => Key::L,
			},
		))
	}
}

#[cfg(target_os = "macos")]
pub mod native {
	use super::*;
	use objc::declare::ClassDecl;
	use objc::runtime::{Object, Sel};
	use objc::{class, msg_send, sel, sel_impl};
	use std::{
		cell::RefCell,
		ffi::CString,
		sync::{Mutex, OnceLock},
	};
	static PENDING: Mutex<Vec<Action>> = Mutex::new(Vec::new());
	static CONTEXT: OnceLock<eframe::egui::Context> = OnceLock::new();
	thread_local! { static ITEMS: RefCell<Vec<(*mut Object, Action)>> = const { RefCell::new(Vec::new()) }; }
	unsafe fn string(value: &str) -> *mut Object {
		let value = CString::new(value).unwrap();
		msg_send![class!(NSString), stringWithUTF8String: value.as_ptr()]
	}
	extern "C" fn activate(_: &Object, _: Sel, sender: *mut Object) {
		let tag: usize = unsafe { msg_send![sender, tag] };
		if let Some(action) = Action::ALL.get(tag).copied() {
			PENDING.lock().unwrap().push(action);
			if let Some(ctx) = CONTEXT.get() {
				ctx.request_repaint();
			}
		}
	}
	extern "C" fn request_quit(_: &Object, _: Sel, _: *mut Object) {
		if let Some(ctx) = CONTEXT.get() {
			ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Close);
		}
	}
	pub fn install(ctx: eframe::egui::Context) {
		let _ = CONTEXT.set(ctx);
		unsafe {
			let mut decl = ClassDecl::new("Printer3MenuTarget", class!(NSObject)).unwrap();
			decl.add_method(
				sel!(performPrinter3Action:),
				activate as extern "C" fn(&Object, Sel, *mut Object),
			);
			decl.add_method(
				sel!(requestPrinter3Quit:),
				request_quit as extern "C" fn(&Object, Sel, *mut Object),
			);
			let target: *mut Object = msg_send![decl.register(), new];
			let app: *mut Object = msg_send![class!(NSApplication), sharedApplication];
			let bar: *mut Object = msg_send![app, mainMenu];
			// Keep the standard application menu (Quit, Hide, Services).
			let bar = if bar.is_null() {
				let menu: *mut Object = msg_send![class!(NSMenu), new];
				let _: () = msg_send![app, setMainMenu: menu];
				menu
			} else {
				bar
			};
			// Route the standard Quit menu and Command-Q through the close guard.
			let count: isize = msg_send![bar, numberOfItems];
			if count > 0 {
				let heading: *mut Object = msg_send![bar, itemAtIndex: 0isize];
				let menu: *mut Object = msg_send![heading, submenu];
				if !menu.is_null() {
					let count: isize = msg_send![menu, numberOfItems];
					for index in 0..count {
						let item: *mut Object = msg_send![menu, itemAtIndex: index];
						let action: Sel = msg_send![item, action];
						if action == sel!(terminate:) {
							let _: () = msg_send![item, setTarget: target];
							let _: () = msg_send![item, setAction: sel!(requestPrinter3Quit:)];
						}
					}
				}
			}
			for group in GROUPS {
				let heading: *mut Object = msg_send![class!(NSMenuItem), new];
				let _: () = msg_send![heading, setTitle: string(group)];
				let menu: *mut Object = msg_send![class!(NSMenu), new];
				let _: () = msg_send![menu, setTitle: string(group)];
				let _: () = msg_send![menu, setAutoenablesItems: false];
				for action in Action::ALL.into_iter().filter(|a| a.group() == group) {
					let key = match action {
						Action::Import => "i",
						Action::Open => "o",
						Action::Save => "s",
						Action::Undo => "z",
						Action::Clear => "\u{8}",
						Action::Perspective => "1",
						Action::Top => "2",
						Action::Front => "3",
						Action::Help => "/",
						Action::License => "",
					};
					let item: *mut Object = msg_send![class!(NSMenuItem), alloc];
					let item: *mut Object = msg_send![item, initWithTitle: string(action.label()) action: sel!(performPrinter3Action:) keyEquivalent: string(key)];
					let _: () = msg_send![item, setTarget: target];
					let _: () = msg_send![item, setTag: action as usize];
					let mask: usize = (1 << 20) | if action == Action::Clear { 1 << 17 } else { 0 };
					let _: () = msg_send![item, setKeyEquivalentModifierMask: mask];
					let symbol = match action {
						Action::Import => "square.and.arrow.down",
						Action::Open => "folder",
						Action::Save => "square.and.arrow.down.on.square",
						Action::Undo => "arrow.uturn.backward",
						Action::Clear => "trash",
						Action::Perspective => "cube",
						Action::Top => "square.topthird.inset.filled",
						Action::Front => "square",
						Action::Help => "questionmark.circle",
						Action::License => "doc.text",
					};
					let image: *mut Object = msg_send![class!(NSImage), imageWithSystemSymbolName: string(symbol) accessibilityDescription: string(action.label())];
					if !image.is_null() {
						let _: () = msg_send![item, setImage: image];
					}
					let _: () = msg_send![menu, addItem: item];
					ITEMS.with(|items| items.borrow_mut().push((item, action)));
					let _: () = msg_send![item, release];
				}
				let _: () = msg_send![heading, setSubmenu: menu];
				let _: () = msg_send![bar, addItem: heading];
				let _: () = msg_send![menu, release];
				let _: () = msg_send![heading, release];
			}
			// The application owns this target for its lifetime; NSMenuItem does not retain targets.
		}
	}
	pub fn drain() -> Vec<Action> {
		std::mem::take(&mut *PENDING.lock().unwrap())
	}
	pub fn update_enabled(undo: bool, clear: bool) {
		ITEMS.with(|items| {
			for &(item, action) in items.borrow().iter() {
				let enabled = match action {
					Action::Undo => undo,
					Action::Clear => clear,
					_ => true,
				};
				unsafe {
					let _: () = msg_send![item, setEnabled: enabled];
				}
			}
		});
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn menu_actions_have_unique_shortcuts_and_clean_labels() {
		for (index, action) in Action::ALL.iter().enumerate() {
			assert!(GROUPS.contains(&action.group()));
			assert!(!action.label().contains('…'));
			assert!(!action.icon().is_empty());
			for other in Action::ALL.iter().skip(index + 1) {
				assert_ne!(action.shortcut(), other.shortcut());
			}
		}
	}
}
