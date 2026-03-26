use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

mod de;
mod platform;
use de::methods::IncrementalBuffer;
use de::rules::{Action, get_default_rules};
use platform::macos::methods::{MacOS, RealEventPoster};
use platform::windowos::methods::{WindowsOS};

use std::collections::HashMap;

use objc2::rc::Retained;
use objc2::{MainThreadOnly, define_class, sel};
use objc2_app_kit::{
    NSAlert, NSApplication, NSApplicationActivationPolicy, NSMenu, NSMenuItem, NSStatusBar, NSStatusItem, NSVariableStatusItemLength, NSWorkspace
};
use objc2_foundation::{MainThreadMarker, NSObject, NSURL, ns_string};
use std::io::{self, Write};

pub static IS_ENABLED: AtomicBool = AtomicBool::new(true);

struct RawStatusItem(*const NSStatusItem);
unsafe impl Send for RawStatusItem {}
unsafe impl Sync for RawStatusItem {}
static GLOBAL_TRAY_ICON_TITLE: OnceLock<RawStatusItem> = OnceLock::new();

struct RawMenuItem(*const NSMenuItem);
unsafe impl Send for RawMenuItem {}
unsafe impl Sync for RawMenuItem {}
static GLOBAL_TOGGLE_MENU_ITEM: OnceLock<RawMenuItem> = OnceLock::new();

struct RawMenuHandler(*const MenuHandler);
unsafe impl Send for RawMenuHandler {}
unsafe impl Sync for RawMenuHandler {}
static GLOBAL_HANDLER: OnceLock<RawMenuHandler> = OnceLock::new();

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
    io::stdout().flush().unwrap();
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
            if let Some(raw_tray_icon_item) = GLOBAL_TRAY_ICON_TITLE.get() {
                let mtm = MainThreadMarker::from(self);
                let tray_icon = unsafe { &*raw_tray_icon_item.0 };

                if let Some(button) = tray_icon.button(mtm) {
                    button.setTitle(new_title);
                }
            }
            if let Some(raw_toggle_menu_item) = GLOBAL_TOGGLE_MENU_ITEM.get() {
                unsafe {
                    let item = &*raw_toggle_menu_item.0;
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
        let toggle_item: Retained<NSMenuItem> = NSMenuItem::new(mtm);
        toggle_item.setTitle(ns_string!("Toggle Umlaut"));
        toggle_item.setKeyEquivalent(ns_string!("z"));
        toggle_item.setKeyEquivalentModifierMask(
            objc2_app_kit::NSEventModifierFlags::Command
                | objc2_app_kit::NSEventModifierFlags::Shift,
        );
        toggle_item.setTarget(Some(handler));
        toggle_item.setAction(Some(sel!(toggleEngine:)));
        let initial_state = if IS_ENABLED.load(Ordering::SeqCst) {
            1
        } else {
            0
        };
        toggle_item.setState(initial_state);
        let _ = GLOBAL_TOGGLE_MENU_ITEM.set(RawMenuItem(Retained::as_ptr(&toggle_item)));
        menu.addItem(&toggle_item);

        menu.addItem(&NSMenuItem::separatorItem(mtm));

        let quit_item = NSMenuItem::new(mtm);
        quit_item.setTitle(ns_string!("Quit UmlautKey"));
        quit_item.setAction(Some(sel!(terminate:)));
        menu.addItem(&quit_item);
    }

    menu
}

fn check_accessibility_permission(mtm: MainThreadMarker) -> bool {
    unsafe {
        // Cách nhanh nhất để check quyền mà không cần thư viện ngoài phức tạp:
        // Thử tạo một EventTap ảo hoặc dùng command line check.
        // Tuy nhiên, cách chuẩn nhất là dùng AXIsProcessTrusted()
        
        #[link(name = "ApplicationServices", kind = "framework")]
        unsafe extern "C" {
            fn AXIsProcessTrusted() -> bool;
        }

        if !AXIsProcessTrusted() {
            let alert = NSAlert::new(mtm);
            alert.setMessageText(ns_string!("Accessibility Permission Missing"));
            alert.setInformativeText(ns_string!(
                "UmlautKey requires Accessibility permissions to detect keyboard shortcuts. \n\nPlease grant permission in System Settings > Privacy & Security > Accessibility, then restart the application."
            ));
            alert.addButtonWithTitle(ns_string!("Open System Settings"));
            alert.addButtonWithTitle(ns_string!("Quit"));

            let response = alert.runModal();
            
            // Nếu bấm nút đầu tiên (Mở Settings)
            if response == 1000 {
                let url = NSURL::URLWithString(ns_string!(
                    "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
                ));
                if let Some(url) = url {
                    NSWorkspace::sharedWorkspace().openURL(&url);
                }
            }
            return false;
        }
    }
    true
}

fn main() {
    let mtm = MainThreadMarker::new().expect("Need to run in main thread");
    let app = NSApplication::sharedApplication(mtm);
    if !check_accessibility_permission(mtm) {
        return;
    }
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    app.activate();
    let status_bar = NSStatusBar::systemStatusBar();
    let status_item_leaked = Box::leak(Box::new(
        status_bar.statusItemWithLength(NSVariableStatusItemLength),
    ));
    let _ = GLOBAL_TRAY_ICON_TITLE.set(RawStatusItem(&**status_item_leaked as *const NSStatusItem));
    let handler_leaked: &mut Retained<MenuHandler> = Box::leak(Box::new(MenuHandler::new(mtm)));
    let _ = GLOBAL_HANDLER.set(RawMenuHandler(Retained::as_ptr(handler_leaked)));
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
        let mut windowsos: WindowsOS<'_, RealEventPoster> = WindowsOS::new(buffer, raw_tracker, spoof_until, poster);
        println!("⌨️  UmlautKey Engine started (Version 0.3.2 Stack)...");
        macos.run_event_tap();
        windowsos.run_loop();
    });
    app.run();
}
