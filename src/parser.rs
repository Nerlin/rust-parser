use std::fmt::{Display, Formatter};
use std::fs::read_to_string;

use indexmap::{IndexMap, IndexSet};
use regex::Regex;

use crate::tokenizer::{Pattern, Tokenizer};

pub struct Parser {
    pub grammars: IndexMap<String, GrammarVariants>,
    pub(crate) first: FirstSet,
    pub(crate) follow: FollowSet,
    pub(crate) table: ParsingTable,
}

pub type GrammarVariants = Vec<GrammarVariant>;

type GrammarVariant = Vec<NodeType>;
type TokenName = String;
type GrammarName = String;
type FirstSet = IndexMap<GrammarName, IndexSet<TokenName>>;
type FollowSet = IndexMap<GrammarName, IndexSet<TokenName>>;
type ParsingTable = IndexMap<(GrammarName, TokenName), GrammarVariant>;

#[derive(Debug, Clone)]
pub enum NodeType {
    Token { name: String, pattern: Pattern },
    Grammar { name: String },
}

impl Display for NodeType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeType::Token { name, pattern: _ } => {
                write!(f, "{name}")
            }
            NodeType::Grammar { name } => {
                write!(f, "{name}")
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

        let grammar_declaration = Regex::new(r"(?<name>.+)\s*->\s*(?<pattern>.*)").unwrap();

        let mut grammars: IndexMap<String, GrammarVariants> = IndexMap::new();

        for (index, line) in content.lines().enumerate() {
            if let Some(capture) = grammar_declaration.captures(line) {
                let (_, [name, pattern]) = capture.extract();

                let name = name.trim();
                let variant_patterns: Vec<&str> = pattern.split("|").map(|s| s.trim()).collect();

                let mut variants: GrammarVariants = vec![];
                for variant_pattern in variant_patterns {
                    let body = variant_pattern.split(" ");
                    let mut nodes: Vec<NodeType> = vec![];
                    for item in body {
                        if let Some(pattern) =
                            tokenizer.patterns.iter().find(|token| token.name == item)
                        {
                            nodes.push(NodeType::Token {
                                name: pattern.name.clone(),
                                pattern: pattern.clone(),
                            });
                        } else {
                            nodes.push(NodeType::Grammar {
                                name: String::from(item),
                            })
                        }
                    }

                    if index == 0 {
                        nodes.push(Parser::eof());
                    }
                    variants.push(nodes);
                }
                grammars.insert(String::from(name), variants);
            }
        }

        let mut first = IndexMap::new();
        let mut follow = IndexMap::new();

        for grammar in grammars.keys() {
            build_first(grammar, &grammars, &mut first);
        }

        for grammar in grammars.keys() {
            build_follow(grammar, &grammars, &first, &mut follow);
        }

        let table = build_parsing_table(&grammars, &first, &follow);
        Ok(Parser {
            grammars,
            first,
            follow,
            table,
        })
    }

    fn eof() -> NodeType {
        NodeType::Token {
            name: String::from("$"),
            pattern: Pattern {
                name: String::from("$"),
                value: Regex::new("").unwrap(),
            },
        }
    }
}

fn build_first(
    grammar_name: &GrammarName,
    grammars: &IndexMap<String, GrammarVariants>,
    first: &mut FirstSet,
) {
    if let Some(_) = first.get(grammar_name) {
        return;
    }

    let mut nodes = IndexSet::new();
    if let Some(variants) = grammars.get(grammar_name) {
        for variant in variants.iter() {
            if let Some(node) = variant.iter().next() {
                match node {
                    NodeType::Token { name, .. } => {
                        nodes.insert(name.clone());
                    }
                    NodeType::Grammar { name, .. } => {
                        build_first(name, grammars, first);
                        if let Some(children) = first.get(name) {
                            for child in children.iter() {
                                nodes.insert(child.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    first.insert(grammar_name.clone(), nodes);
}

fn build_follow(
    grammar_name: &GrammarName,
    grammars: &IndexMap<String, GrammarVariants>,
    first: &FirstSet,
    follow: &mut FollowSet,
) {
    if let Some(_) = follow.get(grammar_name) {
        return;
    }

    let mut nodes: IndexSet<String> = IndexSet::new();
    for (grammar, variants) in grammars.iter() {
        for variant in variants.iter() {
            let mut found = false;
            let mut resolved = false;

            for (index, node) in variant.iter().enumerate() {
                match node {
                    NodeType::Token { name, .. } => {
                        if name == "epsilon" {
                            // Epsilon tokens cannot be included in follow sets
                            continue;
                        }

                        let found_after_grammar = found && name != "$";
                        let found_as_last_token_in_grammar = grammar == grammar_name
                            && index == variant.len() - 1
                            && variant.len() != 1;

                        if found_after_grammar || found_as_last_token_in_grammar {
                            nodes.insert(name.clone());
                            resolved = true;
                            break;
                        }
                    }
                    NodeType::Grammar { name, .. } => {
                        if name == grammar_name {
                            found = true;
                        } else if found {
                            resolved = true;

                            if let Some(first_nodes) = first.get(name) {
                                for node in first_nodes.iter() {
                                    if node == "epsilon" {
                                        // Epsilon tokens cannot be included in follow sets
                                        continue;
                                    }
                                    nodes.insert(node.clone());
                                }
                            }

                            if has_epsilon(variants) {
                                if let Some(follow_nodes) = follow.get(grammar) {
                                    for node in follow_nodes.iter() {
                                        nodes.insert(node.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if found && !resolved && grammar.ne(grammar_name) {
                if let Some(follow_nodes) = follow.get(grammar) {
                    for node in follow_nodes.iter() {
                        nodes.insert(node.clone());
                    }
                }
            }
        }
    }

    follow.insert(grammar_name.clone(), nodes);
}

fn has_epsilon(variants: &GrammarVariants) -> bool {
    for variant in variants.iter() {
        if is_epsilon(variant) {
            return true;
        }
    }
    false
}

fn is_epsilon(variant: &GrammarVariant) -> bool {
    if variant.len() == 1 {
        let node = variant.first().unwrap();
        if let NodeType::Token { name, .. } = node {
            if name == "epsilon" {
                return true;
            }
        }
    }
    false
}

fn build_parsing_table(
    grammars: &IndexMap<String, Vec<GrammarVariant>>,
    first: &FirstSet,
    follow: &FollowSet,
) -> ParsingTable {
    let mut table: ParsingTable = IndexMap::new();

    for (grammar, variants) in grammars.iter() {
        for variant in variants.iter() {
            if is_epsilon(variant) {
                if let Some(tokens) = follow.get(grammar) {
                    insert_parsing_table_row(&mut table, &grammar, tokens, variant);
                }
            } else if let Some(tokens) = first.get(grammar) {
                if let Some(NodeType::Token { name, .. }) = variant.first() {
                    for token in tokens.iter() {
                        if token == name {
                            insert_parsing_table_row(
                                &mut table,
                                &grammar,
                                &IndexSet::from([token.clone()]),
                                variant,
                            );
                            break;
                        }
                    }
                } else {
                    insert_parsing_table_row(&mut table, &grammar, tokens, variant);
                }
            }
        }
    }

    table
}

fn insert_parsing_table_row(
    table: &mut ParsingTable,
    grammar: &String,
    tokens: &IndexSet<TokenName>,
    variant: &GrammarVariant,
) {
    for token in tokens.iter() {
        if token != "epsilon" {
            let mut nodes: GrammarVariant = vec![];
            for node in variant.iter() {
                nodes.push(node.clone());
            }
            table.insert((grammar.clone(), token.clone()), nodes);
        }
    }
}
