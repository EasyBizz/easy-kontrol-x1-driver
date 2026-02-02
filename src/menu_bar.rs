use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::OnceLock;

use cocoa::appkit::{NSApplication, NSImage, NSMenu, NSMenuItem, NSStatusBar};
use cocoa::base::{id, nil, NO, YES};
use cocoa::foundation::{NSAutoreleasePool, NSSize, NSString};
use objc::declare::ClassDecl;
use objc::runtime::{Class, Sel};
use objc::{class, msg_send, sel, sel_impl};

pub struct MenuBar {
    status_item: id,
    menu: id,
    handler: id,
}

#[derive(Clone, Copy, Debug)]
pub enum MenuCommand {
    Start,
    Stop,
    Quit,
}

static MENU_SENDER: OnceLock<Sender<MenuCommand>> = OnceLock::new();

pub fn set_menu_sender(sender: Sender<MenuCommand>) {
    let _ = MENU_SENDER.set(sender);
}

impl MenuBar {
    pub fn new(icon_path: Option<PathBuf>) -> Self {
        unsafe {
            let _pool = NSAutoreleasePool::new(nil);
            NSApplication::sharedApplication(nil);

            let status_item = NSStatusBar::systemStatusBar(nil).statusItemWithLength_(cocoa::appkit::NSVariableStatusItemLength);
            let menu = NSMenu::new(nil).autorelease();
            let _: () = msg_send![status_item, setMenu: menu];

            if let Some(path) = icon_path {
                if let Some(img) = load_image(&path) {
                    let button: id = msg_send![status_item, button];
                    let _: () = msg_send![button, setImage: img];
                }
            } else {
                let button: id = msg_send![status_item, button];
                let title = NSString::alloc(nil).init_str("🎛️");
                let _: () = msg_send![button, setTitle: title];
            }

            let handler = menu_handler_instance();
            Self { status_item, menu, handler }
        }
    }

    pub fn update_menu(&mut self, items: &HashMap<String, bool>, running: bool) {
        unsafe {
            // Clear existing items.
            let count: usize = msg_send![self.menu, numberOfItems];
            for _ in (0..count).rev() {
                let _: () = msg_send![self.menu, removeItemAtIndex: 0usize];
            }

            let start_item = menu_item("Start", sel!(startAction:), self.handler);
            let stop_item = menu_item("Stop", sel!(stopAction:), self.handler);
            let _: () = msg_send![start_item, setEnabled: if running { NO } else { YES }];
            let _: () = msg_send![stop_item, setEnabled: if running { YES } else { NO }];
            let _: () = msg_send![self.menu, addItem: start_item];
            let _: () = msg_send![self.menu, addItem: stop_item];
            let _: () = msg_send![self.menu, addItem: NSMenuItem::separatorItem(nil)];

            let mut names: Vec<(&String, &bool)> = items.iter().collect();
            names.sort_by(|a, b| a.0.cmp(b.0));
            for (idx, (_name, connected)) in names.iter().enumerate() {
                let prefix = if **connected { "🟢" } else { "🔴" };
                let label = if names.len() > 1 {
                    format!("EASY KONTROL X1 {}", idx + 1)
                } else {
                    "EASY KONTROL X1".to_string()
                };
                let title = NSString::alloc(nil).init_str(&format!("{} {}", prefix, label));
                let item = NSMenuItem::alloc(nil)
                    .initWithTitle_action_keyEquivalent_(title, sel!(noop:), NSString::alloc(nil).init_str(""));
                let _: () = msg_send![item, setEnabled: false];
                let _: () = msg_send![self.menu, addItem: item];
            }

            let _: () = msg_send![self.menu, addItem: NSMenuItem::separatorItem(nil)];
            let quit_item = menu_item("Quit", sel!(quitAction:), self.handler);
            let _: () = msg_send![self.menu, addItem: quit_item];
        }
    }
}

fn load_image(path: &Path) -> Option<id> {
    unsafe {
        let path_str = path.to_string_lossy();
        let ns_path = NSString::alloc(nil).init_str(&path_str);
        let image: id = NSImage::alloc(nil).initByReferencingFile_(ns_path);
        if image == nil {
            return None;
        }
        let _: () = msg_send![image, setSize: NSSize::new(18.0, 18.0)];
        let _: () = msg_send![image, setTemplate: YES];
        Some(image)
    }
}

fn menu_item(title: &str, action: Sel, target: id) -> id {
    unsafe {
        let title = NSString::alloc(nil).init_str(title);
        let item = NSMenuItem::alloc(nil)
            .initWithTitle_action_keyEquivalent_(title, action, NSString::alloc(nil).init_str(""));
        let _: () = msg_send![item, setTarget: target];
        item
    }
}

fn menu_handler_class() -> &'static Class {
    static CLASS: OnceLock<&'static Class> = OnceLock::new();
    *CLASS.get_or_init(|| {
        let mut decl = ClassDecl::new("EasyKontrolMenuHandler", class!(NSObject)).unwrap();
        unsafe {
            decl.add_method(sel!(startAction:), start_action as extern "C" fn(&objc::runtime::Object, Sel, id));
            decl.add_method(sel!(stopAction:), stop_action as extern "C" fn(&objc::runtime::Object, Sel, id));
            decl.add_method(sel!(quitAction:), quit_action as extern "C" fn(&objc::runtime::Object, Sel, id));
        }
        decl.register()
    })
}

fn menu_handler_instance() -> id {
    unsafe {
        let cls = menu_handler_class();
        let obj: id = msg_send![cls, new];
        obj
    }
}

extern "C" fn start_action(_: &objc::runtime::Object, _: Sel, _: id) {
    if let Some(sender) = MENU_SENDER.get() {
        let _ = sender.send(MenuCommand::Start);
    }
}

extern "C" fn stop_action(_: &objc::runtime::Object, _: Sel, _: id) {
    if let Some(sender) = MENU_SENDER.get() {
        let _ = sender.send(MenuCommand::Stop);
    }
}

extern "C" fn quit_action(_: &objc::runtime::Object, _: Sel, _: id) {
    if let Some(sender) = MENU_SENDER.get() {
        let _ = sender.send(MenuCommand::Quit);
    }
}
