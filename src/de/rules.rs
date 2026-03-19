#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    ModifyToUmlaut,   // Transform ae => ä, oe => ö, ue => ü
    ModifyToEszett,   // Transform s thành ß
}

use std::collections::HashMap;
pub type Definition = HashMap<char, Action>;

pub fn get_default_rules() -> Definition {
    let mut m = HashMap::new();
    m.insert('e', Action::ModifyToUmlaut);
    m.insert('E', Action::ModifyToUmlaut);
    m.insert('s', Action::ModifyToEszett);
    m.insert('S', Action::ModifyToEszett);
    m
}
