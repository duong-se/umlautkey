#[macro_use]
extern crate lazy_static;

mod de;

use de::methods::IncrementalBuffer;
use de::rules::{Action, get_default_rules};
use enigo::{Enigo, Keyboard, Settings};
use rdev::{Event, EventType, Key as RdevKey, listen};
use std::collections::{HashMap, HashSet};
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

    if let Err(error) = listen(handle_event) {
        eprintln!("rdev error: {:?}", error);
    }
}

fn handle_event(event: Event) {
    if is_spoofing() {
        return;
    }

    match event.event_type {
        EventType::KeyPress(key) => handle_key_press(key),
        EventType::KeyRelease(key) => handle_key_release(key),
        _ => {}
    }
}

fn handle_key_press(key: RdevKey) {
    update_key_state_on_press(key);

    if key == RdevKey::Backspace {
        handle_backspace();
        return;
    }

    if should_clear_tracker(key) {
        clear_tracker();
        return;
    }

    let (is_shift, is_caps_lock, has_modifier) = current_keyboard_flags();
    if has_modifier {
        return;
    }

    let Some(ch) = rdev_key_to_char(key, is_shift, is_caps_lock) else {
        return;
    };

    let current = {
        let mut tracker = RAW_TRACKER.lock().unwrap();
        tracker.push(ch);
        tracker.clone()
    };

    let transformed = handle_transform(&current);
    println!("pressed key: {:?}", key);
    println!("pushed char: {:?}", ch);
    println!("current: {:?}", current);
    println!("current chars: {}", current.chars().count());
    println!("current bytes: {}", current.len());
    println!("transformed: {:?}", transformed);
    println!(
        "transformed chars: {:?}",
        transformed.chars().collect::<Vec<_>>()
    );

    if current != transformed {
        apply_transformation(&current, &transformed);
    }
}

fn handle_key_release(key: RdevKey) {
    let mut keys = KEY_STATE.lock().unwrap();
    keys.remove(&key);
}

fn handle_backspace() {
    if is_spoofing() {
        return;
    }

    let mut tracker = RAW_TRACKER.lock().unwrap();
    tracker.pop();
    let _ = io::stdout().flush();
}

fn update_key_state_on_press(key: RdevKey) {
    let mut keys = KEY_STATE.lock().unwrap();
    keys.insert(key);
}

fn should_clear_tracker(key: RdevKey) -> bool {
    matches!(
        key,
        RdevKey::Return
            | RdevKey::Space
            | RdevKey::UpArrow
            | RdevKey::DownArrow
            | RdevKey::LeftArrow
            | RdevKey::RightArrow
            | RdevKey::PageDown
            | RdevKey::PageUp
            | RdevKey::Home
            | RdevKey::Tab
    )
}

fn clear_tracker() {
    let mut tracker = RAW_TRACKER.lock().unwrap();
    tracker.clear();
}

fn current_keyboard_flags() -> (bool, bool, bool) {
    let keys = KEY_STATE.lock().unwrap();

    let is_shift = keys.contains(&RdevKey::ShiftLeft) || keys.contains(&RdevKey::ShiftRight);

    let is_caps_lock = keys.contains(&RdevKey::CapsLock);

    let has_modifier = keys.contains(&RdevKey::ControlLeft)
        || keys.contains(&RdevKey::ControlRight)
        || keys.contains(&RdevKey::MetaLeft)
        || keys.contains(&RdevKey::MetaRight)
        || keys.contains(&RdevKey::Alt)
        || keys.contains(&RdevKey::AltGr);

    (is_shift, is_caps_lock, has_modifier)
}

fn apply_transformation(current: &str, transformed: &str) {
    set_spoofing(true);

    let old_len = current.chars().count();

    println!("delete key {:?}", old_len);
    println!("send {:?}", transformed);

    {
        let mut enigo = ENIGO.lock().unwrap();

        for _ in 0..old_len {
            let _ = enigo.key(enigo::Key::Backspace, enigo::Direction::Click);
        }
        println!("transformed bytes: {:?}", transformed.as_bytes());
        println!(
            "transformed chars: {:?}",
            transformed.chars().collect::<Vec<_>>()
        );
        let _ = enigo.text(transformed);
    }

    {
        let mut tracker = RAW_TRACKER.lock().unwrap();
        tracker.clear();
        tracker.push_str(transformed);
        println!("tracker {:?}", *tracker);
    }

    println!("transformed {:?}", transformed);
    let _ = io::stdout().flush();

    set_spoofing(false);
}

fn is_spoofing() -> bool {
    *IS_SPOOFING.lock().unwrap()
}

fn set_spoofing(value: bool) {
    let mut spoofing = IS_SPOOFING.lock().unwrap();
    *spoofing = value;
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

fn rdev_key_to_char(key: RdevKey, shift: bool, caps_lock: bool) -> Option<char> {
    use rdev::Key::*;

    let uppercase = shift ^ caps_lock;

    match key {
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

        Space => Some(' '),
        Return => Some('\n'),
        Tab => Some('\t'),

        _ => None,
    }
}
