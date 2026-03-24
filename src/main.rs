use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

mod de;
mod platform;
use de::methods::IncrementalBuffer;
use de::rules::{Action, get_default_rules};
use platform::macos::methods::{MacOS, RealEventPoster};

use std::collections::HashMap;

use objc2::rc::Retained;
use objc2::{MainThreadOnly, define_class, sel};
use objc2_app_kit::{
    NSApplication,
    NSMenu,
    NSMenuItem,
    NSStatusBar,
    NSStatusItem,
    NSVariableStatusItemLength,
};
use objc2_foundation::{MainThreadMarker, NSObject, ns_string};

pub static IS_ENABLED: AtomicBool = AtomicBool::new(true);

struct RawStatusItem(*const NSStatusItem);
unsafe impl Send for RawStatusItem {}
unsafe impl Sync for RawStatusItem {}

static GLOBAL_STATUS_ITEM: OnceLock<RawStatusItem> = OnceLock::new();

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

fn toggle_engine_rust() {
    let current = IS_ENABLED.load(Ordering::SeqCst);
    IS_ENABLED.store(!current, Ordering::SeqCst);
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "MenuHandler"]
    #[derive(Debug)]
    pub struct MenuHandler;
    impl MenuHandler {
        #[unsafe(method(toggleEngine:))]
        fn toggle_engine_objc(&self, _sender: Option<&NSMenuItem>) {
            toggle_engine_rust();
            let is_enabled = IS_ENABLED.load(Ordering::SeqCst);
            let new_title = if is_enabled { ns_string!("Ä") } else { ns_string!("E") };
            if let Some(raw_item) = GLOBAL_STATUS_ITEM.get() {
                let mtm = MainThreadMarker::from(self);
                let status_item = unsafe { &*raw_item.0 };

                if let Some(button) = status_item.button(mtm) {
                    button.setTitle(new_title);
                }
                if let Some(item) = _sender {
                    item.setState(if is_enabled { 1 } else { 0 });
                }
            }
        }
    }
);

impl MenuHandler {
    pub fn new(mtm: objc2_foundation::MainThreadMarker) -> Retained<Self> {
        unsafe {
            let obj = mtm.alloc::<Self>();
            objc2::msg_send![obj, init]
        }
    }
}

fn create_menu(mtm: MainThreadMarker, handler: &MenuHandler) -> Retained<NSMenu> {
    let menu = NSMenu::new(mtm);

    unsafe {
        let toggle_item = NSMenuItem::new(mtm);
        toggle_item.setTitle(ns_string!("Toggle Umlaut"));
        toggle_item.setTarget(Some(handler));
        toggle_item.setAction(Some(sel!(toggleEngine:)));
        let initial_state = if IS_ENABLED.load(Ordering::SeqCst) { 1 } else { 0 };
        toggle_item.setState(initial_state);
        menu.addItem(&toggle_item);

        menu.addItem(&NSMenuItem::separatorItem(mtm));

        let quit_item = NSMenuItem::new(mtm);
        quit_item.setTitle(ns_string!("Quit UmlautKey"));
        quit_item.setAction(Some(sel!(terminate:)));
        menu.addItem(&quit_item);
    }

    menu
}

fn main() {
    let mtm = MainThreadMarker::new().expect("Need to run in main thread");
    let app = NSApplication::sharedApplication(mtm);
    let status_bar = NSStatusBar::systemStatusBar();
    let status_item_leaked = Box::leak(Box::new(
        status_bar.statusItemWithLength(NSVariableStatusItemLength),
    ));
    let handler_leaked: &mut Retained<MenuHandler> = Box::leak(Box::new(MenuHandler::new(mtm)));
    let _ = GLOBAL_STATUS_ITEM.set(RawStatusItem(&**status_item_leaked as *const NSStatusItem));

    if let Some(button) = status_item_leaked.button(mtm) {
        button.setTitle(if IS_ENABLED.load(Ordering::Relaxed) {
            ns_string!("Ä")
        } else {
            ns_string!("E")
        });
    }

    let menu = create_menu(mtm, handler_leaked);
    status_item_leaked.setMenu(Some(&menu));

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
