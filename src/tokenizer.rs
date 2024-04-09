use std::fmt;
use std::fmt::Formatter;
use std::fs::read_to_string;

use regex::Regex;

pub struct Pattern {
    pub name: String,
    pub value: Regex
}

pub struct Token {
    pub name: String,
    pub value: String
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} -> {}", &self.name, &self.value)
    }
}

pub struct Tokenizer {
    pub patterns: Vec<Pattern>
}

impl Tokenizer {

    pub fn from_file(path: &str) -> Result<Tokenizer, String> {
        let content = match read_to_string(path) {
            Err(e) => return Err(
                format!("Unable to open the specified file: {e}")
            ),
            Ok(f) => f
        };

        let token_declaration = Regex::new(r"(?<name>\w+) = (?<pattern>.*)").unwrap();
        let mut patterns = vec![];

        for line in content.lines().into_iter() {
            if let Some(capture) = token_declaration.captures(line) {
                let (_, [name, raw_pattern])  = capture.extract();

                let parts: Vec<String> = raw_pattern.split("|").map(|s| {
                    format!("^{}$", s.trim())
                }).collect();

                let pattern = parts.join("|");

                let regex = match Regex::new(pattern.as_str()) {
                    Ok(r) => {
                        r
                    }
                    Err(..) => {
                        return Err(
                            format!("Unable to parse {name} token - {pattern} is an incorrect regular expression.")
                        )
                    }
                };
                let token = Pattern {
                    name: String::from(name),
                    value: regex,
                };
                patterns.push(token);
            } else {
                return Err(
                    String::from("The file must contain token declarations with NAME = PATTERN format.")
                )
            }
        }

        Ok(
            Tokenizer { patterns }
        )
    }

    pub fn parse(&self, s: &str) -> Result<Vec<Token>, String> {
        let mut result = vec![];

        let mut lookup = String::new();
        let mut unmatched = "";

        for sub in split_keep(&s) {
            let mut matched: Option<Token> = None;

            lookup = lookup + sub;

            let lookup_str = lookup.as_str();
            for pattern in self.patterns.iter() {
                if pattern.value.is_match(lookup_str) {
                    matched = Some(Token {
                        name: pattern.name.clone(),
                        value: lookup.clone()
                    });

                    break;
                }
            };

            if let Some(token) = matched {
                result.push(token);
                lookup = String::new();
                unmatched = "";
            } else if unmatched.len() == 0 {
                unmatched = sub;
            }
        }

        if unmatched.len() > 0 {
            Err(format!("Unknown token {unmatched}."))
        } else {
            Ok(result)
        }
    }
}

fn split_keep(text: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut last = 0;
    for (index, matched) in text.match_indices(|c: char| !c.is_alphanumeric()) {
        if last != index {
            result.push(&text[last..index]);
        }
        if matched != " " {
            result.push(matched);
        }
        last = index + matched.len();
    }
    if last < text.len() {
        result.push(&text[last..]);
    }
    result
}