use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

mod de;
mod platform;
use de::methods::IncrementalBuffer;
use de::rules::{Action, get_default_rules};
use platform::macos::methods::{MacOS, RealEventPoster};

use std::collections::HashMap;

use objc2::rc::Retained;
use objc2_app_kit::{NSApplication, NSMenu, NSMenuItem, NSStatusBar, NSVariableStatusItemLength};
use objc2_foundation::{MainThreadMarker, ns_string};

static IS_ENABLED: AtomicBool = AtomicBool::new(true);

static RULES: OnceLock<HashMap<char, Action>> = OnceLock::new();
fn get_rules() -> &'static HashMap<char, Action> {
    RULES.get_or_init(|| get_default_rules())
}

static BUFFER: OnceLock<Mutex<IncrementalBuffer<'static>>> = OnceLock::new();
fn get_buffer(rules: &'static HashMap<char, Action>) -> &'static Mutex<IncrementalBuffer<'static>> {
    BUFFER.get_or_init(|| Mutex::new(IncrementalBuffer::new(rules)))
}

static RAW_TRACKER: OnceLock<Mutex<String>> = OnceLock::new();
fn get_raw_tracker(value: String) -> &'static Mutex<String> {
    RAW_TRACKER.get_or_init(|| Mutex::new(value))
}

static SPOOF_UNTIL: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();
fn get_spoof_until() -> &'static Mutex<Option<Instant>> {
    SPOOF_UNTIL.get_or_init(|| Mutex::new(None))
}

fn create_menu(mtm: MainThreadMarker) -> Retained<NSMenu> {
    let menu = NSMenu::new(mtm);

    unsafe {
        let toggle_item = NSMenuItem::new(mtm);
        toggle_item.setTitle(ns_string!("Toggle Umlaut (Ctrl+Shift+U)"));
        toggle_item.setKeyEquivalent(ns_string!("u"));

        menu.addItem(&toggle_item);

        menu.addItem(&NSMenuItem::separatorItem(mtm));

        let quit_item = NSMenuItem::new(mtm);
        quit_item.setTitle(ns_string!("Quit UmlautKey"));
        quit_item.setKeyEquivalent(ns_string!("q"));

        quit_item.setAction(Some(objc2::sel!(terminate:)));
        menu.addItem(&quit_item);
    }

    menu
}

fn main() {
    let mtm = MainThreadMarker::new().expect("Need to run in main thread");
    let app = NSApplication::sharedApplication(mtm);
    let status_bar = NSStatusBar::systemStatusBar();
    let status_item = status_bar.statusItemWithLength(NSVariableStatusItemLength);

    if let Some(button) = status_item.button(mtm) {
        button.setTitle(ns_string!("Ä"));
    }

    let menu = create_menu(mtm);
    status_item.setMenu(Some(&menu));

    std::thread::spawn(move || {
        let rules = get_rules();
        let buffer = get_buffer(rules);
        let raw_tracker = get_raw_tracker(String::new());
        let spoof_until = get_spoof_until();

        let poster = RealEventPoster {};
        let mut macos = MacOS::new(buffer, raw_tracker, spoof_until, poster);

        println!("⌨️  UmlautKey Engine started (Version 0.3.2 Stack)...");
        macos.run_event_tap();
    });
    app.run();
}
