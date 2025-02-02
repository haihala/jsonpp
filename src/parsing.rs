use std::collections::HashMap;

use log::debug;

use crate::jsonpp::JsonPP;

pub(crate) struct Parser {
    chars: Vec<char>,
    index: usize,
}

impl From<Vec<u8>> for Parser {
    fn from(bytes: Vec<u8>) -> Self {
        Parser {
            chars: bytes.into_iter().map(char::from).collect(),
            index: 0,
        }
    }
}

impl Parser {
    pub fn parse(&mut self) -> JsonPP {
        debug!("Parsing generic");
        self.skip_whitespace();

        let Some(first_char) = self.chars.get(self.index) else {
            panic!("Index out of bounds");
        };

        match first_char {
            '[' => self.parse_array(),
            '{' => self.parse_object(),
            '=' => self.parse_dynamic(),
            '"' => self.parse_string(),
            '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' | '-' => self.parse_number(),
            _ => self.parse_other(),
        }
    }

    fn skip(&mut self, mut cond: impl FnMut(char) -> bool) {
        while let Some(ch) = self.current() {
            if self.starts_with("//") {
                while !self.starts_with("\n") {
                    self.index += 1;
                }
                // Skip over the newline
                self.index += 1;
                continue;
            }

            if self.starts_with("/*") {
                while !self.starts_with("*/") {
                    self.index += 1;
                }
                // Skip over the closing comment
                self.index += 2;
                continue;
            }

            if cond(ch) {
                self.index += 1;
            } else {
                return;
            }
        }
    }

    fn skip_whitespace(&mut self) {
        self.skip(|ch| ch.is_whitespace());
    }

    fn skip_to_next_iterable(&mut self) {
        self.skip(|ch| ch.is_whitespace() || ch == ',');
    }

    fn take_while(&mut self, mut cond: impl FnMut(char) -> bool) -> String {
        let mut coll = vec![];
        while let Some(ch) = self.chars.get(self.index) {
            if cond(*ch) {
                coll.push(*ch);
                self.index += 1;
            } else {
                break;
            }
        }

        coll.into_iter().collect()
    }

    fn starts_with(&self, to_match: &str) -> bool {
        let bytes: Vec<char> = to_match.chars().collect();
        self.chars
            .iter()
            .skip(self.index)
            .zip(bytes)
            .all(|(a, b)| *a == b)
    }

    fn current(&self) -> Option<char> {
        self.chars.get(self.index).cloned()
    }

    fn parse_object(&mut self) -> JsonPP {
        debug!("Parsing object");

        // It starts with {
        assert!(self.current() == Some('{'));
        self.index += 1;

        // Recursively call parse for intermediate objects
        self.skip_whitespace();
        let mut coll = HashMap::new();
        while self.current() != Some('}') {
            let JsonPP::String(key) = self.parse_string() else {
                panic!("String parsing yields non-strings")
            };
            debug!("Key: {}", key);

            self.skip(|ch| ch.is_whitespace() || ch == ':');

            let value = self.parse();
            debug!("Value: {:?}", value);
            coll.insert(key, value);
            self.skip_to_next_iterable();
        }
        // It should end with the closing half
        assert!(self.current() == Some('}'));
        self.index += 1;
        JsonPP::Object(coll)
    }

    fn parse_array(&mut self) -> JsonPP {
        debug!("Parsing array");

        // It starts with [. Read until the other pair
        assert!(self.current() == Some('['));
        self.index += 1;

        // Recursively call parse for intermediate objects
        self.skip_whitespace();
        let mut coll = vec![];
        while self.current() != Some(']') {
            coll.push(self.parse());
            self.skip_to_next_iterable();
        }

        // It should end with the closing half
        assert!(self.current() == Some(']'));
        self.index += 1;

        JsonPP::Array(coll)
    }

    fn parse_string(&mut self) -> JsonPP {
        debug!("Parsing string");
        // It starts with double quotes
        assert!(self.current() == Some('"'));
        self.index += 1;

        // Read until other double quote
        // Ignore escaped double quotes
        // Ignore escaped escapes
        let mut being_escaped = false;
        let out = JsonPP::String(self.take_while(move |ch| {
            if ch == '"' {
                if being_escaped {
                    being_escaped = false;
                    true
                } else {
                    false
                }
            } else {
                if ch == '\\' {
                    // Odd amount cancel each other out
                    being_escaped = !being_escaped;
                } else {
                    // Any other character means we're not escaping
                    being_escaped = false;
                }
                true
            }
        }));

        assert!(self.current() == Some('"'));
        self.index += 1;
        out
    }

    fn parse_number(&mut self) -> JsonPP {
        debug!("Parsing number");
        // Can be an int or a float, positive or negative
        let curr = self.current().unwrap();
        assert!(curr.is_numeric() || curr == '-');

        // Read until comma, see if there is a period, do int or float based on that
        let string = self
            .take_while(|ch| ch.is_ascii_digit() || ".-+eE".contains(ch))
            .to_lowercase();

        // Rust default float parsing is very good, but panics on fractional exponents
        if ".e".chars().any(|ch| string.contains(ch)) {
            let (mant, exp) = string.split_once('e').unwrap_or((&string, "0"));
            let mantissa: f64 = mant.parse().unwrap();
            let exponent: f64 = exp.parse().unwrap();
            JsonPP::Float(mantissa * 10.0f64.powf(exponent))
        } else {
            JsonPP::Int(string.parse().unwrap())
        }
    }

    fn parse_dynamic(&mut self) -> JsonPP {
        debug!("Parsing dynamic");
        // It's gonna be a bunch of nested function calls and operators
        // Read until the first comma that's not within parenthesis
        todo!();
    }

    fn parse_other(&mut self) -> JsonPP {
        debug!("Parsing other");
        // Valid values: true, false, or null
        // Alternatively it can be anything else, in which case panic for now.

        for (matcher, value) in [
            ("true", JsonPP::Bool(true)),
            ("false", JsonPP::Bool(false)),
            ("null", JsonPP::Null),
        ] {
            if self.starts_with(matcher) {
                self.index += matcher.len();
                return value;
            }
        }

        panic!("Could not parse");
    }
}
