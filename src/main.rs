use de::methods::IncrementalBuffer;
use de::rules::{Action, get_default_rules};
use platform::macos::methods::{MacOS, RealEventPoster};
use std::sync::{OnceLock};
use std::{collections::HashMap, sync::Mutex, time::Instant};

mod de;
mod platform;

static RULES: OnceLock<HashMap<char, Action>> = OnceLock::new();
fn get_rules() -> &'static HashMap<char, Action> {
    RULES.get_or_init(|| return get_default_rules())
}

static BUFFER: OnceLock<Mutex<IncrementalBuffer<'static>>> = OnceLock::new();
fn get_buffer(rules: &'static HashMap<char, Action>) -> &'static Mutex<IncrementalBuffer<'static>> {
    BUFFER.get_or_init(|| return Mutex::new(IncrementalBuffer::new(rules)))
}

static RAW_TRACKER: OnceLock<Mutex<String>> = OnceLock::new();
fn get_raw_tracker(value: String) -> &'static Mutex<String> {
    RAW_TRACKER.get_or_init(|| return Mutex::new(value))
}

static SPOOF_UNTIL: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();
fn get_spoof_until() -> &'static Mutex<Option<Instant>> {
    SPOOF_UNTIL.get_or_init(|| {
        return Mutex::new(None);
    })
}

fn main() {
    let rules = get_rules(); // Trả về &'static HashMap
    let buffer = get_buffer(&rules);
    let raw_tracker = get_raw_tracker(String::new());
    let spoof_until = get_spoof_until();
    println!("🚀 UmlautKey macOS objc2-core-graphics backend");
    println!("Rules: ae -> ä, oe -> ö, ue -> ü, ss -> ß");
    println!("Remember Accessibility permission.");
    let poster = RealEventPoster{};
    let mut macos = MacOS::new(&buffer, &raw_tracker, &spoof_until, poster);
    macos.run_event_tap();
}
