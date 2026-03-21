#[macro_use]
extern crate lazy_static;

mod de;

use de::methods::IncrementalBuffer;
use de::rules::{Action, get_default_rules};
use enigo::{Enigo, Keyboard, Settings}; // Lưu ý: Enigo có Key trùng tên rdev
use rdev::{EventType, Key as RdevKey, listen};
use std::collections::HashMap;
use std::collections::HashSet;
use std::io::{self, Write};
use std::sync::Mutex;

lazy_static! {
    static ref RULES: HashMap<char, Action> = get_default_rules();
    static ref BUFFER: Mutex<IncrementalBuffer<'static>> =
        Mutex::new(IncrementalBuffer::new(&RULES));
    static ref ENIGO: Mutex<Enigo> =
        Mutex::new(Enigo::new(&Settings::default()).expect("Cannot init Enigo"));
    static ref RAW_TRACKER: Mutex<String> = Mutex::new(String::new());
    static ref KEY_STATE: Mutex<HashSet<RdevKey>> = Mutex::new(HashSet::new());
    static ref IS_SPOOFING: Mutex<bool> = Mutex::new(false);
}

fn main() {
    println!("🚀 UmlautKey (V0.0.1) is running...");
    println!("Rules: ae -> ä, oe -> ö, ue -> ü, ss -> ß");

    // Rdev listen events
    if let Err(error) = listen(|event| {
        handle_event(event);
    }) {
        println!("rdev error: {:?}", error);
    }
}

fn handle_event(event: rdev::Event) {
    {
        if *IS_SPOOFING.lock().unwrap() {
            return;
        }
    }
    let mut tracker = RAW_TRACKER.lock().unwrap();
    let mut keys = KEY_STATE.lock().unwrap();
    let mut enigo = ENIGO.lock().unwrap();

    match event.event_type {
        EventType::KeyPress(key) => {
            keys.insert(key);
            if keys.contains(&RdevKey::Return)
                || keys.contains(&RdevKey::Space)
                || keys.contains(&RdevKey::UpArrow)
                || keys.contains(&RdevKey::DownArrow)
                || keys.contains(&RdevKey::LeftArrow)
                || keys.contains(&RdevKey::RightArrow)
                || keys.contains(&RdevKey::PageDown)
                || keys.contains(&RdevKey::PageUp)
                || keys.contains(&RdevKey::Home)
                || keys.contains(&RdevKey::Tab)
            {
                tracker.clear();
                return;
            }

            if keys.contains(&RdevKey::ControlLeft)
                || keys.contains(&RdevKey::ControlRight)
                || keys.contains(&RdevKey::MetaLeft)
                || keys.contains(&RdevKey::MetaRight)
                || keys.contains(&RdevKey::Alt)
                || keys.contains(&RdevKey::AltGr)
            {
                return;
            }

            let is_shift =
                keys.contains(&RdevKey::ShiftLeft) || keys.contains(&RdevKey::ShiftRight);
            let is_caplock = keys.contains(&RdevKey::CapsLock);
            if let Some(c) = rdev_key_to_char(key, is_shift, is_caplock) {
                tracker.push(c);
                let old_len: usize = tracker.to_owned().chars().count();
                let transformed = handle_transform(tracker.to_owned().to_string());
                let new_len = transformed.to_owned().chars().count();
                if old_len < new_len {
                    tracker.clear();
                    for ch in transformed.chars() {
                        tracker.push(ch);
                    }
                    {
                        *IS_SPOOFING.lock().unwrap() = true;
                    }
                    for _ in 0..=old_len {
                        let _ = enigo.key(enigo::Key::Backspace, enigo::Direction::Press);
                    }
                    let _ = enigo.text(&transformed);
                    {
                        *IS_SPOOFING.lock().unwrap() = false;
                    }
                }
            }
            if key == RdevKey::Backspace {
                {
                    if *IS_SPOOFING.lock().unwrap() {
                        return;
                    }
                }
                tracker.pop(); // Delete last char if user use back space
                io::stdout().flush().unwrap();
            }
        }
        EventType::KeyRelease(key) => {
            keys.remove(&key);
        }
        _ => {}
    }
}

fn handle_transform(s: String) -> String {
    let mut buffer = BUFFER.lock().unwrap();
    for c in s.chars() {
        buffer.push(c);
    }
    let result = buffer.view().to_string();
    buffer.clear();
    return result.to_string();
}

fn rdev_key_to_char(key: RdevKey, shift: bool, caps_lock: bool) -> Option<char> {
    use rdev::Key::*;
    let uppercase = shift ^ caps_lock;
    match key {
        // A-Z
        KeyA => Some(if uppercase { 'A' } else { 'a' }),
        KeyB => Some(if uppercase { 'B' } else { 'b' }),
        KeyC => Some(if uppercase { 'C' } else { 'c' }),
        KeyD => Some(if uppercase { 'D' } else { 'd' }),
        KeyE => Some(if uppercase { 'E' } else { 'e' }),
        KeyF => Some(if uppercase { 'F' } else { 'f' }),
        KeyG => Some(if uppercase { 'G' } else { 'g' }),
        KeyH => Some(if uppercase { 'H' } else { 'h' }),
        KeyI => Some(if uppercase { 'I' } else { 'i' }),
        KeyJ => Some(if uppercase { 'J' } else { 'j' }),
        KeyK => Some(if uppercase { 'K' } else { 'k' }),
        KeyL => Some(if uppercase { 'L' } else { 'l' }),
        KeyM => Some(if uppercase { 'M' } else { 'm' }),
        KeyN => Some(if uppercase { 'N' } else { 'n' }),
        KeyO => Some(if uppercase { 'O' } else { 'o' }),
        KeyP => Some(if uppercase { 'P' } else { 'p' }),
        KeyQ => Some(if uppercase { 'Q' } else { 'q' }),
        KeyR => Some(if uppercase { 'R' } else { 'r' }),
        KeyS => Some(if uppercase { 'S' } else { 's' }),
        KeyT => Some(if uppercase { 'T' } else { 't' }),
        KeyU => Some(if uppercase { 'U' } else { 'u' }),
        KeyV => Some(if uppercase { 'V' } else { 'v' }),
        KeyW => Some(if uppercase { 'W' } else { 'w' }),
        KeyX => Some(if uppercase { 'X' } else { 'x' }),
        KeyY => Some(if uppercase { 'Y' } else { 'y' }),
        KeyZ => Some(if uppercase { 'Z' } else { 'z' }),

        // Only check shift
        Num1 => Some(if shift { '!' } else { '1' }),
        Num2 => Some(if shift { '@' } else { '2' }),
        Num3 => Some(if shift { '#' } else { '3' }),
        Num4 => Some(if shift { '$' } else { '4' }),
        Num5 => Some(if shift { '%' } else { '5' }),
        Num6 => Some(if shift { '^' } else { '6' }),
        Num7 => Some(if shift { '&' } else { '7' }),
        Num8 => Some(if shift { '*' } else { '8' }),
        Num9 => Some(if shift { '(' } else { '9' }),
        Num0 => Some(if shift { ')' } else { '0' }),

        // Numpad
        Kp0 => Some('0'),
        Kp1 => Some('1'),
        Kp2 => Some('2'),
        Kp3 => Some('3'),
        Kp4 => Some('4'),
        Kp5 => Some('5'),
        Kp6 => Some('6'),
        Kp7 => Some('7'),
        Kp8 => Some('8'),
        Kp9 => Some('9'),
        KpPlus => Some('+'),
        KpMinus => Some('-'),
        KpMultiply => Some('*'),
        KpDivide => Some('/'),

        // Special chars
        Minus => Some(if shift { '_' } else { '-' }),
        Equal => Some(if shift { '+' } else { '=' }),
        LeftBracket => Some(if shift { '{' } else { '[' }),
        RightBracket => Some(if shift { '}' } else { ']' }),
        BackSlash => Some(if shift { '|' } else { '\\' }),
        SemiColon => Some(if shift { ':' } else { ';' }),
        Quote => Some(if shift { '"' } else { '\'' }),
        Comma => Some(if shift { '<' } else { ',' }),
        Dot => Some(if shift { '>' } else { '.' }),
        Slash => Some(if shift { '?' } else { '/' }),
        BackQuote => Some(if shift { '~' } else { '`' }),

        // System
        Space => Some(' '),
        Return => Some('\n'),
        Tab => Some('\t'),

        // KpDecimal => Some('.'),
        // KpEnter => Some('\n'),
        _ => None,
    }
}
