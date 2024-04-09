use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::fs::read_to_string;

use regex::Regex;

use crate::tokenizer::{Pattern, Tokenizer};

pub struct Parser {
    pub grammar: HashMap<String, GrammarVariants>,
}

pub type GrammarVariants = Vec<Vec<NodeType>>;

pub enum NodeType {
    Token {
        name: String,
        pattern: Pattern,
        optional: bool,
    },
    Grammar {
        name: String,
        optional: bool,
    },
}

impl Display for NodeType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeType::Token {
                name,
                pattern: _,
                optional,
            } => {
                if *optional {
                    write!(f, "[Token(\"{}\")]", &name)
                } else {
                    write!(f, "Token(\"{}\")", &name)
                }
            }
            NodeType::Grammar { name, optional } => {
                if *optional {
                    write!(f, "[Grammar(\"{}\")]", &name)
                } else {
                    write!(f, "Grammar(\"{}\")", &name)
                }
            }
        }
    }
}

impl Parser {
    pub fn from_file(path: &str, tokenizer: Tokenizer) -> Result<Parser, String> {
        let content = match read_to_string(path) {
            Err(e) => return Err(format!("Unable to open the specified file: {e}")),
            Ok(f) => f,
        };

        let grammar_declaration = Regex::new(r"(?<name>\w+)\s*->\s*(?<pattern>.*)").unwrap();

        let mut grammar = HashMap::new();

        for line in content.lines().into_iter() {
            if let Some(capture) = grammar_declaration.captures(line) {
                let (_, [name, pattern]) = capture.extract();
                let variant_patterns: Vec<&str> = pattern.split("|").map(|s| s.trim()).collect();

                let mut variants: GrammarVariants = vec![];
                for variant_pattern in variant_patterns {
                    let body = variant_pattern.split(" ");
                    let mut nodes: Vec<NodeType> = vec![];
                    for item in body {
                        let optional = item.starts_with("[") && item.ends_with("]");
                        let item_name = if optional { &item[1..item.len() - 1] } else { &item };

                        if let Some(pattern) =
                            tokenizer.patterns.iter().find(|token| token.name == item_name)
                        {
                            nodes.push(NodeType::Token {
                                name: pattern.name.clone(),
                                pattern: pattern.clone(),
                                optional,
                            });
                        } else {
                            nodes.push(NodeType::Grammar {
                                name: String::from(item_name),
                                optional,
                            })
                        }
                    }
                    variants.push(nodes);
                }
                grammar.insert(String::from(name), variants);
            }
        }

        Ok(Parser { grammar })
    }
}
