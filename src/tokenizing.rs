#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    Colon,
    OpenParanthesis,
    CloseParanthesis,
    OpenBrace,
    CloseBrace,
    OpenBracket,
    CloseBracket,
    Text(String),  // Quoted string
    Ident(String), // Unquoted alphabetic text
    Int(i64),      // Whole numbers, can be negative
    Float(f64),    // Floating point numbers, can be negative
}

pub fn tokenize(input: Vec<u8>) -> impl Iterator<Item = Token> {
    let mut stream = str::from_utf8(&input).unwrap().chars().peekable();
    let mut tokens = vec![];
    let mut prev_token_complete = true;

    while let Some(next_char) = stream.next() {
        if !prev_token_complete {
            // We are either collecting Text or Ident
            let prev_token = tokens.last_mut().unwrap();
            match prev_token {
                Token::Text(content) => {
                    if next_char == '"' {
                        // Under most conditions this ends the string
                        // Not if it's escaped
                        // But yes if the escape is escaped
                        // Count how many trailing escape characters we have
                        // On an even number, it terminates the string
                        let count = content.chars().rev().take_while(|ch| *ch == '\\').count();
                        if count % 2 == 0 {
                            prev_token_complete = true;
                            *content = handle_escape_characters(content.clone());
                        } else {
                            content.push(next_char);
                        }
                    } else {
                        content.push(next_char);
                    }
                    continue;
                }
                Token::Ident(content) => {
                    if is_valid_ident_char(next_char) {
                        content.push(next_char);
                        continue;
                    } else {
                        *prev_token = complete_ident(content);
                        prev_token_complete = true;
                        // No continue because we want to process this char
                    }
                }
                other => panic!(
                    "We should be collecting Text or Ident, but found {:?}",
                    other
                ),
            }
        }

        // We're not collecting a bigger item
        if next_char.is_ascii_whitespace() {
            // Skip whitespace
            continue;
        }

        if next_char == ',' {
            // Commas are optional
            continue;
        }

        if next_char == '/' {
            if let Some(&after) = stream.peek() {
                if after == '/' {
                    // Line comment, ignore until newline
                    for next in stream.by_ref() {
                        if next == '\n' {
                            break;
                        }
                    }
                } else if after == '*' {
                    // Block comment, ignore until */
                    while let Some(next) = stream.next() {
                        if next == '*' && stream.peek() == Some(&'/') {
                            assert_eq!(stream.next(), Some('/'));
                            break;
                        }
                    }
                }
            }
        }

        if let Some(special_token) = special(next_char) {
            // Scoop up special single char tokens
            tokens.push(special_token);
            continue;
        }

        if next_char == '"' {
            // Start Text tokens
            tokens.push(Token::Text("".into()));
            prev_token_complete = false;
            continue;
        }

        // Rest start off as Ident.
        // Because this can just be a minus sign, we can't represent it as a number
        // Instead let's use Ident and then convert to numeric at the end
        if is_valid_ident_char(next_char) {
            tokens.push(Token::Ident(next_char.into()));
            prev_token_complete = false;
        }
    }

    if !prev_token_complete {
        // Chars ended mid collectable
        // If it's an ident, just end it here
        if let Some(last_token) = tokens.last_mut() {
            if let Token::Ident(content) = last_token {
                *last_token = complete_ident(content)
            } else {
                panic!("Text token incomplete when input ended: {:?}", last_token);
            }
        }
    }
    tokens.into_iter()
}

fn complete_ident(content: &str) -> Token {
    if let Ok(integer) = content.parse::<i64>() {
        return Token::Int(integer);
    }

    // Rust default float parsing is very good, but panics on fractional exponents
    let lower = content.to_ascii_lowercase();
    let numeric_char = |ch: char| "-+.e".contains(ch) || ch.is_ascii_digit();
    if lower.chars().all(numeric_char) {
        let (mant, exp) = lower.split_once('e').unwrap_or((&lower, "0"));
        let mantissa: f64 = mant.parse().unwrap();
        let exponent: f64 = exp.parse().unwrap();
        return Token::Float(mantissa * 10.0f64.powf(exponent));
    }

    Token::Ident(content.into())
}

fn is_valid_ident_char(test_char: char) -> bool {
    if test_char.is_ascii_whitespace() {
        return false;
    }

    if test_char == '/' {
        return false;
    }

    if test_char == ',' {
        return false;
    }

    if special(test_char).is_some() {
        return false;
    }

    true
}

fn special(input: char) -> Option<Token> {
    Some(match input {
        ':' => Token::Colon,
        '(' => Token::OpenParanthesis,
        ')' => Token::CloseParanthesis,
        '{' => Token::OpenBrace,
        '}' => Token::CloseBrace,
        '[' => Token::OpenBracket,
        ']' => Token::CloseBracket,
        _ => return None,
    })
}

fn handle_escape_characters(input: String) -> String {
    let mut iter = input.chars().peekable();
    let mut coll = vec![];

    let mut skip_next = false;
    while let Some(current) = iter.next() {
        if skip_next {
            skip_next = false;
            continue;
        }

        if current == '\\' {
            if let Some(special) = iter.peek().and_then(|next| match next {
                'n' => Some("\n"),
                't' => Some("\t"),
                '"' => Some("\""),
                '\\' => Some("\\"),
                _ => None,
            }) {
                skip_next = true;
                coll.push(special.to_string());
                continue;
            }
        }

        coll.push(current.to_string());
    }

    coll.join("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input() {
        assert!(tokenize(vec![]).next().is_none())
    }

    #[test]
    fn simple_types() {
        for (input, output) in [
            ("\"foo\"", Token::Text("foo".into())),
            ("simple_ident", Token::Ident("simple_ident".into())),
            ("123", Token::Int(123)),
            ("-123", Token::Int(-123)),
            ("123.5", Token::Float(123.5)),
            ("-123.5", Token::Float(-123.5)),
            ("-123.5", Token::Float(-123.5)),
        ] {
            assert_eq!(tokenize(input.bytes().collect()).next().unwrap(), output);
        }
    }

    #[test]
    fn escape_sequences() {
        for (input, output) in [
            ("\"\n\"", Token::Text("\n".into())), // Newline
            ("\"\\n\"", Token::Text("\n".into())),
            ("\"\t\"", Token::Text("\t".into())), // Tab character
            ("\"\\t\"", Token::Text("\t".into())),
            ("\"\\\\\"", Token::Text("\\".into())),
        ] {
            assert_eq!(tokenize(input.bytes().collect()).next().unwrap(), output);
        }
    }

    #[test]
    fn commaless_arrays() {
        for input in ["[1 2 3]", "[1,2,3]"] {
            assert_eq!(
                tokenize(input.bytes().collect()).collect::<Vec<_>>(),
                vec![
                    Token::OpenBracket,
                    Token::Int(1),
                    Token::Int(2),
                    Token::Int(3),
                    Token::CloseBracket,
                ]
            );
        }
    }
}
