use crate::de;

use de::methods::IncrementalBuffer;

use core::result::Result;
use libc::c_void;
use objc2_core_foundation::{CFMachPort, CFRunLoop, kCFRunLoopCommonModes};
use objc2_core_graphics::{
    CGEvent, CGEventField, CGEventFlags, CGEventTapLocation, CGEventTapOptions,
    CGEventTapPlacement, CGEventTapProxy, CGEventType,
};
use std::io::{self, Write};
use std::ptr::NonNull;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub trait EventPoster: Send + Sync {
    fn post_backspace(&self, proxy: CGEventTapProxy, count: usize) -> Result<(), String>;
    fn post_unicode(&self, proxy: CGEventTapProxy, text: &str) -> Result<(), String>;
}

pub struct RealEventPoster;

impl EventPoster for RealEventPoster {
    fn post_backspace(&self, proxy: CGEventTapProxy, count: usize) -> Result<(), String> {
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
    fn post_unicode(&self, proxy: CGEventTapProxy, text: &str) -> Result<(), String> {
            let utf16: Vec<u16> = text.encode_utf16().collect();

    let event = CGEvent::new_keyboard_event(None, 0, true)
        .ok_or_else(|| "create unicode event failed".to_string())?;

    unsafe {
        CGEvent::keyboard_set_unicode_string(Some(&event), utf16.len() as _, utf16.as_ptr());
        CGEvent::tap_post_event(proxy, Some(&event));
    }

    Ok(())
    }
}

pub struct MacOS<'def, P: EventPoster> {
    buffer: &'def Mutex<IncrementalBuffer<'static>>,
    raw_tracker: &'def Mutex<String>,
    spoof_until: &'def Mutex<Option<Instant>>,
    poster: P,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TapAction {
    Pass,
    Block,
}

unsafe extern "C-unwind" fn tap_callback<P: EventPoster>(
    proxy: CGEventTapProxy,
    event_type: CGEventType,
    event: NonNull<CGEvent>,
    user_info: *mut c_void,
) -> *mut CGEvent {
    let _ = user_info;
    let _ = proxy;
    let _ = event_type;
    let state = unsafe { &mut *(user_info as *mut MacOS<P>) };

    if state.is_in_spoof_window() {
        return std::ptr::null_mut();
    }

    match event_type {
        CGEventType::KeyDown => unsafe {
            let event_ref = event.as_ref();
            let event_opt = Some(event_ref);
            let source_state_id =
                CGEvent::integer_value_field(event_opt, CGEventField::EventSourceStateID);

            if source_state_id != 1 {
                return event.as_ptr();
            }

            let keycode =
                CGEvent::integer_value_field(event_opt, CGEventField::KeyboardEventKeycode) as u16;

            let flags = CGEvent::flags(event_opt);
            match state.handle_key_down(proxy, keycode, flags) {
                TapAction::Pass => event.as_ptr(),
                TapAction::Block => std::ptr::null_mut(),
            }
        },
        CGEventType::LeftMouseDown | CGEventType::RightMouseDown | CGEventType::OtherMouseDown => {
            state.clear_tracker();
            event.as_ptr()
        }
        _ => event.as_ptr(),
    }
}

impl<'def, P: EventPoster> MacOS<'def, P> {
    pub fn new(
        buffer: &'def Mutex<IncrementalBuffer<'static>>,
        raw_tracker: &'def Mutex<String>,
        spoof_until: &'def Mutex<Option<Instant>>,
        poster: P,
    ) -> Self {
        Self {
            buffer: buffer,
            raw_tracker: raw_tracker,
            spoof_until: spoof_until,
            poster,
        }
    }

    fn handle_key_down(
        &mut self,
        proxy: CGEventTapProxy,
        keycode: u16,
        flags: CGEventFlags,
    ) -> TapAction {
        if should_clear_tracker_keycode(keycode, flags) {
            self.clear_tracker();
            return TapAction::Pass;
        }

        if keycode == KC_BACKSPACE {
            self.handle_backspace();
            return TapAction::Pass;
        }

        if has_command_like_modifier(flags) {
            self.clear_tracker();
            return TapAction::Pass;
        }

        let shift = flags.contains(CGEventFlags::MaskShift);
        let caps_lock = flags.contains(CGEventFlags::MaskAlphaShift);

        let Some(ch) = macos_keycode_to_char(keycode, shift, caps_lock) else {
            return TapAction::Pass;
        };

        let current = {
            let mut tracker = self.raw_tracker.lock().unwrap();
            tracker.push(ch);
            tracker.clone()
        };

        let transformed = self.handle_transform(&current);

        if current != transformed {
            self.apply_transformation(proxy, &current, &transformed);
            return TapAction::Block;
        }

        TapAction::Pass
    }
    fn handle_backspace(&mut self) {
        let mut tracker = self.raw_tracker.lock().unwrap();
        tracker.pop();
    }

    fn clear_tracker(&mut self) {
        let mut tracker = self.raw_tracker.lock().unwrap();
        tracker.clear();
    }

    fn begin_spoof_window(&mut self, duration: Duration) {
        let until = Instant::now() + duration;
        let mut spoof_until = self.spoof_until.lock().unwrap();
        *spoof_until = Some(until);
    }

    fn is_in_spoof_window(&mut self) -> bool {
        let spoof_until = self.spoof_until.lock().unwrap();
        matches!(*spoof_until, Some(until) if Instant::now() <= until)
    }

    fn apply_transformation(&mut self, proxy: CGEventTapProxy, current: &str, transformed: &str) {
        let delete_count = current.chars().count();

        self.begin_spoof_window(Duration::from_millis(50));

        if let Err(err) = self.poster.post_backspace(proxy, delete_count) {
            eprintln!("send_backspace error: {}", err);
            return;
        }

        if let Err(err) = self.poster.post_unicode(proxy, transformed) {
            eprintln!("send_unicode error: {}", err);
            return;
        }
        let _ = io::stdout().flush();
    }

    fn handle_transform(&mut self, input: &str) -> String {
        let mut buffer = self.buffer.lock().unwrap();
        for c in input.chars() {
            buffer.push(c);
        }
        let result = buffer.view().to_string();
        buffer.clear();
        result
    }

    pub fn run_event_tap(&mut self) {
        let mask = (1 << CGEventType::KeyDown.0 as u64)
            | (1 << CGEventType::FlagsChanged.0 as u64)
            | (1 << CGEventType::LeftMouseDown.0 as u64)
            | (1 << CGEventType::RightMouseDown.0 as u64)
            | (1 << CGEventType::OtherMouseDown.0 as u64);
        let self_ptr = self as *mut MacOS<P> as *mut std::ffi::c_void;
        let tap: objc2_core_foundation::CFRetained<CFMachPort> = unsafe {
            CGEvent::tap_create(
                CGEventTapLocation::AnnotatedSessionEventTap,
                CGEventTapPlacement::HeadInsertEventTap,
                CGEventTapOptions::Default,
                mask,
                Some(tap_callback::<P>),
                self_ptr,
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
}

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

fn has_command_like_modifier(flags: CGEventFlags) -> bool {
    flags.contains(CGEventFlags::MaskControl)
        || flags.contains(CGEventFlags::MaskCommand)
        || flags.contains(CGEventFlags::MaskAlternate)
}

fn should_clear_tracker_keycode(keycode: u16, flags: CGEventFlags) -> bool {
    let is_basic_clear = matches!(
        keycode,
        KC_ENTER | KC_TAB | KC_SPACE | KC_ESCAPE | KC_LEFT | KC_RIGHT | KC_UP | KC_DOWN
    );
    let is_command_backspace =
        (keycode == KC_BACKSPACE) && flags.contains(CGEventFlags::MaskCommand);
    is_basic_clear || is_command_backspace
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

#[cfg(test)]
mod tests {

    use super::*;
    use de::rules::Action;
    use std::{collections::HashMap, sync::Mutex};

    use crate::{
        de::methods::A_UMLAUT_LOWERCASE, get_buffer, get_raw_tracker, get_rules, get_spoof_until,
    };

    struct MockPoster {
        pub logs: std::sync::Arc<Mutex<Vec<String>>>,
    }

    impl EventPoster for MockPoster {
        fn post_backspace(&self, _: CGEventTapProxy, count: usize) -> Result<(), String> {
            self.logs.lock().unwrap().push(format!("DELETE_{}", count));
            Ok(())
        }
        fn post_unicode(&self, _: CGEventTapProxy, text: &str) -> Result<(), String> {
            self.logs.lock().unwrap().push(text.to_string());
            Ok(())
        }
    }

    fn setup_test_macos<'a>(
        buffer: &'a Mutex<IncrementalBuffer<'static>>,
        tracker: &'a Mutex<String>,
        spoof: &'a Mutex<Option<Instant>>,
    ) -> (MacOS<'a, MockPoster>, std::sync::Arc<Mutex<Vec<String>>>) {
        let logs = std::sync::Arc::new(Mutex::new(vec![]));
        let poster = MockPoster { logs: logs.clone() };
        let macos = MacOS::new(buffer, tracker, spoof, poster);
        (macos, logs)
    }
    #[test]
    fn test_keycode_conversion() {
        assert_eq!(macos_keycode_to_char(KC_A, false, false), Some('a'));
        assert_eq!(macos_keycode_to_char(KC_A, true, false), Some('A'));
        assert_eq!(macos_keycode_to_char(KC_1, true, false), Some('!'));
        assert_eq!(macos_keycode_to_char(KC_SPACE, false, false), Some(' '));
    }

    #[test]
    fn test_should_clear_tracker() {
        assert!(should_clear_tracker_keycode(
            KC_ENTER,
            CGEventFlags::empty()
        ));

        assert!(should_clear_tracker_keycode(
            KC_BACKSPACE,
            CGEventFlags::MaskCommand
        ));

        assert!(!should_clear_tracker_keycode(
            KC_BACKSPACE,
            CGEventFlags::empty()
        ));
    }

    #[test]
    fn test_handle_backspace_logic() {
        let rules: &'static HashMap<char, Action> = get_rules();
        let buffer = get_buffer(rules);
        let raw_tracker = get_raw_tracker(String::from("hello"));
        let spoof_until = get_spoof_until();

        let (mut macos, _) = setup_test_macos(&buffer, &raw_tracker, &spoof_until);

        macos.handle_backspace();
        assert_eq!(*raw_tracker.lock().unwrap(), "hell");
    }

    #[test]
    fn test_handle_key_down_tracking() {
        let rules: &'static HashMap<char, Action> = get_rules();
        let buffer = get_buffer(rules);
        let raw_tracker = Mutex::new(String::new());
        let spoof_until = get_spoof_until();
        let (mut macos, logs) = setup_test_macos(&buffer, &raw_tracker, &spoof_until);

        let dummy_proxy: CGEventTapProxy = std::ptr::null_mut();

        macos.handle_key_down(dummy_proxy, KC_A, CGEventFlags::empty());
        assert_eq!(*raw_tracker.lock().unwrap(), "a");

        macos.handle_key_down(dummy_proxy, KC_B, CGEventFlags::empty());
        assert_eq!(*raw_tracker.lock().unwrap(), "ab");

        macos.handle_key_down(dummy_proxy, KC_SPACE, CGEventFlags::empty());
        macos.handle_key_down(dummy_proxy, KC_A, CGEventFlags::empty());
        macos.handle_key_down(dummy_proxy, KC_E, CGEventFlags::empty());
        assert_eq!(*raw_tracker.lock().unwrap(), "ae".to_string());
        let history = logs.lock().unwrap();
        assert_eq!(history[0], "DELETE_2");
        assert_eq!(history[1], A_UMLAUT_LOWERCASE.to_string());
    }
}
