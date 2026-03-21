#[macro_use]
extern crate lazy_static;

mod de;

use de::methods::IncrementalBuffer;
use de::rules::{Action, get_default_rules};

use libc::c_void;
use objc2_core_foundation::{CFMachPort, CFRunLoop, kCFRunLoopCommonModes};
use objc2_core_graphics::{
    CGEvent, CGEventField, CGEventFlags, CGEventTapLocation, CGEventTapOptions,
    CGEventTapPlacement, CGEventTapProxy, CGEventType,
};
use std::collections::HashMap;
use std::io::{self, Write};
use std::ptr::NonNull;
use std::sync::Mutex;
use std::time::{Duration, Instant};

lazy_static! {
    static ref RULES: HashMap<char, Action> = get_default_rules();
    static ref BUFFER: Mutex<IncrementalBuffer<'static>> =
        Mutex::new(IncrementalBuffer::new(&RULES));
    static ref RAW_TRACKER: Mutex<String> = Mutex::new(String::new());
    static ref SPOOF_UNTIL: Mutex<Option<Instant>> = Mutex::new(None);
}

// Apple ANSI keycodes cơ bản
const KC_A: u16 = 0;
const KC_S: u16 = 1;
const KC_D: u16 = 2;
const KC_F: u16 = 3;
const KC_H: u16 = 4;
const KC_G: u16 = 5;
const KC_Z: u16 = 6;
const KC_X: u16 = 7;
const KC_C: u16 = 8;
const KC_V: u16 = 9;
const KC_B: u16 = 11;
const KC_Q: u16 = 12;
const KC_W: u16 = 13;
const KC_E: u16 = 14;
const KC_R: u16 = 15;
const KC_Y: u16 = 16;
const KC_T: u16 = 17;
const KC_1: u16 = 18;
const KC_2: u16 = 19;
const KC_3: u16 = 20;
const KC_4: u16 = 21;
const KC_6: u16 = 22;
const KC_5: u16 = 23;
const KC_EQUAL: u16 = 24;
const KC_9: u16 = 25;
const KC_7: u16 = 26;
const KC_MINUS: u16 = 27;
const KC_8: u16 = 28;
const KC_0: u16 = 29;
const KC_RBRACKET: u16 = 30;
const KC_O: u16 = 31;
const KC_U: u16 = 32;
const KC_LBRACKET: u16 = 33;
const KC_I: u16 = 34;
const KC_P: u16 = 35;
const KC_ENTER: u16 = 36;
const KC_L: u16 = 37;
const KC_J: u16 = 38;
const KC_QUOTE: u16 = 39;
const KC_K: u16 = 40;
const KC_SEMICOLON: u16 = 41;
const KC_BACKSLASH: u16 = 42;
const KC_COMMA: u16 = 43;
const KC_SLASH: u16 = 44;
const KC_N: u16 = 45;
const KC_M: u16 = 46;
const KC_DOT: u16 = 47;
const KC_TAB: u16 = 48;
const KC_SPACE: u16 = 49;
const KC_BACKSPACE: u16 = 51;
const KC_ESCAPE: u16 = 53;
const KC_LEFT: u16 = 123;
const KC_RIGHT: u16 = 124;
const KC_DOWN: u16 = 125;
const KC_UP: u16 = 126;

fn main() {
    println!("🚀 UmlautKey macOS objc2-core-graphics backend");
    println!("Rules: ae -> ä, oe -> ö, ue -> ü, ss -> ß");
    println!("Remember Accessibility permission.");

    run_event_tap();
}

unsafe extern "C-unwind" fn tap_callback(
    proxy: CGEventTapProxy,
    event_type: CGEventType,
    event: NonNull<CGEvent>,
    user_info: *mut c_void,
) -> *mut CGEvent {
    let _ = user_info;
    let _ = proxy;
    let _ = event_type;

    if is_in_spoof_window() {
        return std::ptr::null_mut();
    }

    match event_type {
        CGEventType::KeyDown => {
            unsafe {
                // 1. Chuyển đổi NonNull thành reference (&CGEvent)
                let event_ref = event.as_ref();

                // 2. Bọc vào Some() để truyền vào hàm
                let event_opt = Some(event_ref);

                // Lấy Source State ID
                let source_state_id =
                    CGEvent::integer_value_field(event_opt, CGEventField::EventSourceStateID);

                if source_state_id != 1 {
                    return event.as_ptr();
                }

                let keycode =
                    CGEvent::integer_value_field(event_opt, CGEventField::KeyboardEventKeycode)
                        as u16;

                let flags = CGEvent::flags(event_opt);
                match handle_key_down(proxy, keycode, flags) {
                    TapAction::Pass => event.as_ptr(),
                    TapAction::Block => std::ptr::null_mut(),
                }
            }
        }
        CGEventType::LeftMouseDown | CGEventType::RightMouseDown | CGEventType::OtherMouseDown => {
            clear_tracker();
            event.as_ptr()
        }
        _ => event.as_ptr(),
    }
}

fn run_event_tap() {
    let mask = (1 << CGEventType::KeyDown.0 as u64)
        | (1 << CGEventType::FlagsChanged.0 as u64)
        | (1 << CGEventType::LeftMouseDown.0 as u64)
        | (1 << CGEventType::RightMouseDown.0 as u64)
        | (1 << CGEventType::OtherMouseDown.0 as u64);

    let tap: objc2_core_foundation::CFRetained<CFMachPort> = unsafe {
        CGEvent::tap_create(
            // Đừng dùng HID-entry tap cho app thường.
            // Với objc2 binding, variant đúng thường là Session.
            CGEventTapLocation::AnnotatedSessionEventTap,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::Default,
            mask,
            Some(tap_callback),
            std::ptr::null_mut(),
        )
    }
    .expect("Cannot create event tap. Check Accessibility permission.");
    unsafe {
        CGEvent::tap_enable(&tap, true);

        let run_loop_source = CFMachPort::new_run_loop_source(None, Some(&tap), 0)
            .expect("failed to create run loop source");

        let run_loop = CFRunLoop::current().expect("no current run loop");
        run_loop.add_source(Some(&run_loop_source), kCFRunLoopCommonModes);

        CFRunLoop::run();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TapAction {
    Pass,
    Block,
}

fn handle_key_down(proxy: CGEventTapProxy, keycode: u16, flags: CGEventFlags) -> TapAction {
    if should_clear_tracker_keycode(keycode) {
        clear_tracker();
        return TapAction::Pass;
    }

    if keycode == KC_BACKSPACE {
        handle_backspace();
        return TapAction::Pass;
    }

    if has_command_like_modifier(flags) {
        clear_tracker();
        return TapAction::Pass;
    }

    let shift = flags.contains(CGEventFlags::MaskShift);
    let caps_lock = flags.contains(CGEventFlags::MaskAlphaShift);

    let Some(ch) = macos_keycode_to_char(keycode, shift, caps_lock) else {
        return TapAction::Pass;
    };

    let current = {
        let mut tracker = RAW_TRACKER.lock().unwrap();
        tracker.push(ch);
        tracker.clone()
    };

    let transformed = handle_transform(&current);

    if current != transformed {
        apply_transformation(proxy, &current, &transformed);
        return TapAction::Block;
    }

    TapAction::Pass
}

fn apply_transformation(proxy: CGEventTapProxy, current: &str, transformed: &str) {
    let delete_count = current.chars().count();

    begin_spoof_window(Duration::from_millis(50));

    if let Err(err) = send_backspace(proxy, delete_count) {
        eprintln!("send_backspace error: {}", err);
        return;
    }

    if let Err(err) = send_unicode(proxy, transformed) {
        eprintln!("send_unicode error: {}", err);
        return;
    }
    let _ = io::stdout().flush();
}

fn send_backspace(proxy: CGEventTapProxy, count: usize) -> Result<(), String> {
    let down = CGEvent::new_keyboard_event(None, KC_BACKSPACE as u16, true)
        .ok_or_else(|| "create backspace down failed".to_string())?;
    let up = CGEvent::new_keyboard_event(None, KC_BACKSPACE as u16, false)
        .ok_or_else(|| "create backspace up failed".to_string())?;

    for _ in 0..count {
        unsafe {
            CGEvent::tap_post_event(proxy, Some(&down));
            CGEvent::tap_post_event(proxy, Some(&up));
        }
    }

    Ok(())
}

fn send_unicode(proxy: CGEventTapProxy, text: &str) -> Result<(), String> {
    let utf16: Vec<u16> = text.encode_utf16().collect();

    let event = CGEvent::new_keyboard_event(None, 0, true)
        .ok_or_else(|| "create unicode event failed".to_string())?;

    unsafe {
        CGEvent::keyboard_set_unicode_string(Some(&event), utf16.len() as _, utf16.as_ptr());
        CGEvent::tap_post_event(proxy, Some(&event));
    }

    Ok(())
}

fn handle_backspace() {
    let mut tracker = RAW_TRACKER.lock().unwrap();
    tracker.pop();
}

fn clear_tracker() {
    let mut tracker = RAW_TRACKER.lock().unwrap();
    tracker.clear();
}

fn begin_spoof_window(duration: Duration) {
    let until = Instant::now() + duration;
    let mut spoof_until = SPOOF_UNTIL.lock().unwrap();
    *spoof_until = Some(until);
}

fn is_in_spoof_window() -> bool {
    let spoof_until = SPOOF_UNTIL.lock().unwrap();
    matches!(*spoof_until, Some(until) if Instant::now() <= until)
}

fn has_command_like_modifier(flags: CGEventFlags) -> bool {
    flags.contains(CGEventFlags::MaskControl)
        || flags.contains(CGEventFlags::MaskCommand)
        || flags.contains(CGEventFlags::MaskAlternate)
}

fn should_clear_tracker_keycode(keycode: u16) -> bool {
    matches!(
        keycode,
        KC_ENTER | KC_TAB | KC_SPACE | KC_ESCAPE | KC_LEFT | KC_RIGHT | KC_UP | KC_DOWN
    )
}

fn handle_transform(input: &str) -> String {
    let mut buffer = BUFFER.lock().unwrap();
    for c in input.chars() {
        buffer.push(c);
    }
    let result = buffer.view().to_string();
    buffer.clear();
    result
}

fn macos_keycode_to_char(keycode: u16, shift: bool, caps_lock: bool) -> Option<char> {
    let uppercase = shift ^ caps_lock;

    match keycode {
        KC_A => Some(if uppercase { 'A' } else { 'a' }),
        KC_S => Some(if uppercase { 'S' } else { 's' }),
        KC_D => Some(if uppercase { 'D' } else { 'd' }),
        KC_F => Some(if uppercase { 'F' } else { 'f' }),
        KC_H => Some(if uppercase { 'H' } else { 'h' }),
        KC_G => Some(if uppercase { 'G' } else { 'g' }),
        KC_Z => Some(if uppercase { 'Z' } else { 'z' }),
        KC_X => Some(if uppercase { 'X' } else { 'x' }),
        KC_C => Some(if uppercase { 'C' } else { 'c' }),
        KC_V => Some(if uppercase { 'V' } else { 'v' }),
        KC_B => Some(if uppercase { 'B' } else { 'b' }),
        KC_Q => Some(if uppercase { 'Q' } else { 'q' }),
        KC_W => Some(if uppercase { 'W' } else { 'w' }),
        KC_E => Some(if uppercase { 'E' } else { 'e' }),
        KC_R => Some(if uppercase { 'R' } else { 'r' }),
        KC_Y => Some(if uppercase { 'Y' } else { 'y' }),
        KC_T => Some(if uppercase { 'T' } else { 't' }),
        KC_O => Some(if uppercase { 'O' } else { 'o' }),
        KC_U => Some(if uppercase { 'U' } else { 'u' }),
        KC_I => Some(if uppercase { 'I' } else { 'i' }),
        KC_P => Some(if uppercase { 'P' } else { 'p' }),
        KC_L => Some(if uppercase { 'L' } else { 'l' }),
        KC_J => Some(if uppercase { 'J' } else { 'j' }),
        KC_K => Some(if uppercase { 'K' } else { 'k' }),
        KC_N => Some(if uppercase { 'N' } else { 'n' }),
        KC_M => Some(if uppercase { 'M' } else { 'm' }),

        KC_1 => Some(if shift { '!' } else { '1' }),
        KC_2 => Some(if shift { '@' } else { '2' }),
        KC_3 => Some(if shift { '#' } else { '3' }),
        KC_4 => Some(if shift { '$' } else { '4' }),
        KC_5 => Some(if shift { '%' } else { '5' }),
        KC_6 => Some(if shift { '^' } else { '6' }),
        KC_7 => Some(if shift { '&' } else { '7' }),
        KC_8 => Some(if shift { '*' } else { '8' }),
        KC_9 => Some(if shift { '(' } else { '9' }),
        KC_0 => Some(if shift { ')' } else { '0' }),

        KC_MINUS => Some(if shift { '_' } else { '-' }),
        KC_EQUAL => Some(if shift { '+' } else { '=' }),
        KC_LBRACKET => Some(if shift { '{' } else { '[' }),
        KC_RBRACKET => Some(if shift { '}' } else { ']' }),
        KC_BACKSLASH => Some(if shift { '|' } else { '\\' }),
        KC_SEMICOLON => Some(if shift { ':' } else { ';' }),
        KC_QUOTE => Some(if shift { '"' } else { '\'' }),
        KC_COMMA => Some(if shift { '<' } else { ',' }),
        KC_DOT => Some(if shift { '>' } else { '.' }),
        KC_SLASH => Some(if shift { '?' } else { '/' }),

        KC_SPACE => Some(' '),
        KC_ENTER => Some('\n'),
        KC_TAB => Some('\t'),

        _ => None,
    }
}
