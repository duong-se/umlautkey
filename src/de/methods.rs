use crate::de::rules::{Action, Definition};

pub struct IncrementalBuffer<'def> {
    definition: &'def Definition,
    input_history: Vec<char>,
    output: String,
}

impl<'def> IncrementalBuffer<'def> {
    pub fn new(definition: &'def Definition) -> Self {
        Self {
            definition,
            input_history: Vec::new(),
            output: String::new(),
        }
    }

    pub fn push(&mut self, ch: char) {
        let lowercase_ch = ch.to_ascii_lowercase();

        if lowercase_ch == '$' {
            self.output.pop();
            self.output.push('€');
            return;
        }

        if lowercase_ch == 'e' && (self.output.ends_with('ä') || self.output.ends_with('Ä')) {
            let is_upper = self.output.ends_with('Ä');
            self.output.pop();
            self.output.push(if is_upper { 'A' } else { 'a' });
            self.output.push(if ch.is_uppercase() { 'E' } else { 'e' });
            return;
        }

        if lowercase_ch == 'e' && (self.output.ends_with('ü') || self.output.ends_with('Ü')) {
            let is_upper = self.output.ends_with('Ü');
            self.output.pop();
            self.output.push(if is_upper { 'U' } else { 'u' });
            self.output.push(if ch.is_uppercase() { 'E' } else { 'e' });
            return;
        }

        if lowercase_ch == 'e' && (self.output.ends_with('ö') || self.output.ends_with('Ö')) {
            let is_upper = self.output.ends_with('Ö');
            self.output.pop();
            self.output.push(if is_upper { 'O' } else { 'o' });
            self.output.push(if ch.is_uppercase() { 'E' } else { 'e' });
            return;
        }

        if lowercase_ch == 's' && self.output.ends_with('ß') {
            self.output.pop();
            self.output.push('s');
            self.output.push('s');
            return;
        }

        if let Some(action) = self.definition.get(&lowercase_ch) {
            match action {
                Action::ModifyToUmlaut => {
                    // Get the previous character in the history
                    if let Some(&last_char) = self.input_history.last() {
                        match last_char {
                            'a' => {
                                self.output.pop();
                                self.output.push('ä');
                            }
                            'A' => {
                                self.output.pop();
                                self.output.push('Ä');
                            }
                            'o' => {
                                self.output.pop();
                                self.output.push('ö');
                            }
                            'O' => {
                                self.output.pop();
                                self.output.push('Ö');
                            }
                            'u' => {
                                self.output.pop();
                                self.output.push('ü');
                            }
                            'U' => {
                                self.output.pop();
                                self.output.push('Ü');
                            }
                            _ => {
                                self.output.push(ch);
                            }
                        }
                    } else {
                        // If there is no previous character, just add the current character
                        self.output.push(ch);
                    }
                }
                Action::ModifyToEszett => {
                    // If typing 's' after another 's' -> 'ß'
                    if self.input_history.last() == Some(&'s') {
                        self.output.pop();
                        self.output.push('ß');
                    } else if self.input_history.last() == Some(&'S') {
                        self.output.pop();
                        self.output.push('ß');
                    } else {
                        self.output.push(ch);
                    }
                }
            }
        } else {
            // Normal key, not in the rules
            self.output.push(ch);
        }

        // Save to history for processing the next key
        self.input_history.push(ch);
    }

    pub fn view(&self) -> &str {
        &self.output
    }

    pub fn clear(&mut self) {
        self.input_history.clear();
        self.output.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::de::rules::get_default_rules;

    #[test]
    fn test_clear() {
        let rules = get_default_rules();
        let mut buffer = IncrementalBuffer::new(&rules);

        buffer.push('a');
        buffer.clear();
    }

    #[test]
    fn test_umlaut_transformation_with_previous_character_a() {
        let rules = get_default_rules();
        let mut buffer = IncrementalBuffer::new(&rules);

        buffer.push('a');
        buffer.push('e');
        assert_eq!(buffer.view(), "ä");
        buffer.clear();

        buffer.push('a');
        buffer.push('E');
        assert_eq!(buffer.view(), "ä");
        buffer.clear();

        buffer.push('A');
        buffer.push('E');
        assert_eq!(buffer.view(), "Ä");
        buffer.clear();

        buffer.push('a');
        buffer.push('E');
        assert_eq!(buffer.view(), "ä");
        buffer.clear();
    }

    #[test]
    fn test_umlaut_transformation_with_previous_character_o() {
        let rules = get_default_rules();
        let mut buffer = IncrementalBuffer::new(&rules);

        buffer.push('o');
        buffer.push('e');
        assert_eq!(buffer.view(), "ö");
        buffer.clear();

        buffer.push('o');
        buffer.push('E');
        assert_eq!(buffer.view(), "ö");
        buffer.clear();

        buffer.push('O');
        buffer.push('E');
        assert_eq!(buffer.view(), "Ö");
        buffer.clear();

        buffer.push('O');
        buffer.push('e');
        assert_eq!(buffer.view(), "Ö");
        buffer.clear();
    }

    #[test]
    fn test_umlaut_transformation_with_previous_character_u() {
        let rules = get_default_rules();
        let mut buffer = IncrementalBuffer::new(&rules);

        buffer.push('u');
        buffer.push('e');
        assert_eq!(buffer.view(), "ü");
        buffer.clear();

        buffer.push('u');
        buffer.push('E');
        assert_eq!(buffer.view(), "ü");
        buffer.clear();

        buffer.push('U');
        buffer.push('E');
        assert_eq!(buffer.view(), "Ü");
        buffer.clear();

        buffer.push('U');
        buffer.push('e');
        assert_eq!(buffer.view(), "Ü");
        buffer.clear();
    }

    #[test]
    fn test_umlaut_transformation_with_previous_character_a_umlaut() {
        let rules = get_default_rules();
        let mut buffer = IncrementalBuffer::new(&rules);

        buffer.push('ä');
        buffer.push('e');
        assert_eq!(buffer.view(), "ae");
        buffer.clear();

        buffer.push('Ä');
        buffer.push('e');
        assert_eq!(buffer.view(), "Ae");
        buffer.clear();

        buffer.push('Ä');
        buffer.push('E');
        assert_eq!(buffer.view(), "AE");
        buffer.clear();
    }

    #[test]
    fn test_umlaut_transformation_with_previous_character_u_umlaut() {
        let rules = get_default_rules();
        let mut buffer = IncrementalBuffer::new(&rules);

        buffer.push('ü');
        buffer.push('e');
        assert_eq!(buffer.view(), "ue");
        buffer.clear();

        buffer.push('Ü');
        buffer.push('e');
        assert_eq!(buffer.view(), "Ue");
        buffer.clear();

        buffer.push('Ü');
        buffer.push('E');
        assert_eq!(buffer.view(), "UE");
        buffer.clear();
    }

    #[test]
    fn test_umlaut_transformation_with_previous_character_o_umlaut() {
        let rules = get_default_rules();
        let mut buffer = IncrementalBuffer::new(&rules);

        buffer.push('ö');
        buffer.push('e');
        assert_eq!(buffer.view(), "oe");
        buffer.clear();

        buffer.push('Ö');
        buffer.push('e');
        assert_eq!(buffer.view(), "Oe");
        buffer.clear();

        buffer.push('Ö');
        buffer.push('E');
        assert_eq!(buffer.view(), "OE");
        buffer.clear();
    }

    #[test]
    fn test_normal_typing() {
        let rules = get_default_rules();
        let mut buffer = IncrementalBuffer::new(&rules);

        buffer.push('b');
        buffer.push('a');
        buffer.push('n');
        assert_eq!(buffer.view(), "ban");
    }

    #[test]
    fn test_eszett_transformation() {
        let rules = get_default_rules();
        let mut buffer = IncrementalBuffer::new(&rules);

        buffer.push('s');
        buffer.push('s');
        assert_eq!(buffer.view(), "ß");

        buffer.clear();

        buffer.push('S');
        buffer.push('s');
        assert_eq!(buffer.view(), "ß");

        buffer.clear();

        buffer.push('s');
        buffer.push('S');
        assert_eq!(buffer.view(), "ß");
    }

    #[test]
    fn test_umlaut_not_transform_when_previous_character_is_two_e() {
        let rules = get_default_rules();
        let mut buffer = IncrementalBuffer::new(&rules);

        buffer.push('a');
        buffer.push('e');
        buffer.push('e');
        assert_eq!(buffer.view(), "ae");
    }
    #[test]
    fn test_umlaut_not_transform_when_previous_character_is_three_e() {
        let rules = get_default_rules();
        let mut buffer = IncrementalBuffer::new(&rules);

        buffer.push('a');
        buffer.push('e');
        buffer.push('e');
        buffer.push('e');
        assert_eq!(buffer.view(), "aee");
    }

    #[test]
    fn test_eszett_transformation_with_three_s() {
        let rules = get_default_rules();
        let mut buffer = IncrementalBuffer::new(&rules);

        buffer.push('s');
        buffer.push('s');
        buffer.push('s');
        assert_eq!(buffer.view(), "ss");
    }
    #[test]
    fn test_eszett_transformation_with_four_s() {
        let rules = get_default_rules();
        let mut buffer = IncrementalBuffer::new(&rules);

        buffer.push('s');
        buffer.push('s');
        buffer.push('s');
        buffer.push('s');
        assert_eq!(buffer.view(), "sß");
    }
    #[test]
    fn test_eszett_transformation_with_five_s() {
        let rules = get_default_rules();
        let mut buffer = IncrementalBuffer::new(&rules);

        buffer.push('s');
        buffer.push('s');
        buffer.push('s');
        buffer.push('s');
        buffer.push('s');
        assert_eq!(buffer.view(), "sss");
    }
}
