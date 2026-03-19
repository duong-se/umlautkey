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

        // Check if the recently pressed key is in the "function key" list
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
}
