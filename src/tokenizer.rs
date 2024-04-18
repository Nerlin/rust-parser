use std::fmt;
use std::fmt::Formatter;
use std::fs::read_to_string;

use regex::Regex;

pub(crate) const EPSILON: &'static str = "epsilon";

#[derive(Clone, Debug)]
pub struct Pattern {
    pub name: String,
    pub value: Regex,
}

#[derive(Debug)]
pub struct Token {
    pub name: String,
    pub value: String,
    pub line: usize,
    pub column: usize,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}={}", &self.name, &self.value)
    }
}

#[derive(Debug)]
pub struct Tokenizer {
    pub patterns: Vec<Pattern>,
}

impl Tokenizer {
    pub fn from_file(path: &str) -> Result<Tokenizer, String> {
        let content = match read_to_string(path) {
            Err(e) => return Err(format!("Unable to open the specified file: {e}")),
            Ok(f) => f,
        };

        let token_declaration = Regex::new(r"(?<name>.+)\s*=\s*(?<pattern>.*)").unwrap();
        let mut patterns = vec![];

        for line in content.lines().into_iter() {
            if let Some(capture) = token_declaration.captures(line) {
                let (_, [name, raw_pattern]) = capture.extract();

                let parts: Vec<String> = raw_pattern
                    .split("|")
                    .map(|s| format!("^{}$", s.trim()))
                    .collect();

                let name = name.trim();
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
                return Err(String::from(
                    "The file must contain token declarations with NAME = PATTERN format.",
                ));
            }
        }

        patterns.push(Tokenizer::epsilon());
        Ok(Tokenizer { patterns })
    }

    pub fn epsilon() -> Pattern {
        Pattern {
            name: String::from(EPSILON),
            value: Regex::new("").unwrap(),
        }
    }

    pub fn parse(&self, s: &str) -> Result<Vec<Token>, String> {
        let mut result = vec![];

        let mut lookup: Option<Body> = None;
        let mut unmatched = String::new();

        for sub in split_keep(&s) {
            let mut matched: Option<Token> = None;

            let current: Body = match &lookup {
                Some(some) => {
                    Body {
                        value: some.clone().value + sub.value.as_str(),
                        line: some.line,
                        column: some.column,
                    }
                }
                None => sub.clone(),
            };

            for pattern in self.patterns.iter() {
                if pattern.value.is_match(current.value.as_str()) {
                    matched = Some(Token {
                        name: pattern.name.clone(),
                        value: current.value.clone(),
                        line: current.line,
                        column: current.column,
                    });

                    break;
                }
            }

            if let Some(token) = matched {
                result.push(token);
                lookup = None;
                unmatched = String::new();
            } else if unmatched.len() == 0 {
                unmatched = sub.value.clone();
            }
        }

        if unmatched.len() > 0 {
            Err(format!("Unknown token {unmatched}."))
        } else {
            Ok(result)
        }
    }
}

#[derive(Debug, Clone)]
struct Body {
    value: String,
    line: usize,
    column: usize,
}

fn split_keep(text: &str) -> Vec<Body> {
    let mut result: Vec<Body> = Vec::new();
    let mut last = 0;

    let mut line = 1;
    let mut line_start = 0;

    let mut column = 1;
    for (index, matched) in text.match_indices(|c: char| !c.is_alphanumeric()) {
        if last != index {
            let last_match = &text[last..index];

            result.push(Body {
                value: String::from(last_match),
                line,
                column,
            });

            column += last_match.len();
        }

        match matched {
            " " | "\t" => {
                column += 1;
            }
            "\r" => {
                column = 1;
                line_start = index;
            }
            "\n" => {
                line += 1;
                column = 1;
                line_start = index;
            }
            _ => {
                column = index - line_start;
                result.push(Body {
                    value: String::from(matched),
                    line,
                    column,
                });
                column += 1;
            }
        }
        last = index + matched.len();
    }

    if last < text.len() {
        result.push(Body {
            value: String::from(&text[last..]),
            line,
            column,
        });
    }
    result
}
