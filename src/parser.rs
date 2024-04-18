use std::cell::RefCell;
use std::fmt::{Display, Formatter};
use std::fs::read_to_string;
use std::ops::DerefMut;
use std::rc::Rc;

use indexmap::{IndexMap, IndexSet};
use regex::Regex;

use crate::tokenizer::{Pattern, Token, Tokenizer, EPSILON};

pub struct Parser {
    pub grammars: IndexMap<String, GrammarVariants>,
    pub(crate) first: FirstSet,
    pub(crate) follow: FollowSet,
    pub(crate) table: ParsingTable,
    pub(crate) tokenizer: Tokenizer,
}

const EOF: &'static str = "$";
const ROOT: &'static str = "__ROOT";

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

#[derive(Debug)]
pub enum AST {
    Token {
        name: String,
        value: String,
    },
    Grammar {
        name: String,
        children: Vec<Rc<RefCell<AST>>>,
    },
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

                    variants.push(nodes);
                }

                if index == 0 {
                    // Insert the root grammar as a parse start
                    grammars.insert(
                        String::from(ROOT),
                        vec![vec![
                            NodeType::Grammar {
                                name: String::from(name),
                            },
                            Parser::eof(),
                        ]],
                    );
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
            tokenizer,
        })
    }

    fn eof() -> NodeType {
        NodeType::Token {
            name: EOF.to_string(),
            pattern: Pattern {
                name: EOF.to_string(),
                value: Regex::new("").unwrap(),
            },
        }
    }

    pub fn parse(&self, content: &str) -> Result<Rc<RefCell<AST>>, String> {
        let result = self.tokenizer.parse(content);

        if let Err(err) = result {
            return Err(err);
        }

        let mut tokens = result.unwrap();
        tokens.push(Token {
            name: EOF.to_string(),
            value: EOF.to_string(),
            line: 0,
            column: 0,
        });

        let eof = Rc::new(RefCell::new(AST::Token {
            name: EOF.to_string(),
            value: EOF.to_string(),
        }));
        let mut stack: Vec<Rc<RefCell<AST>>> = vec![eof];

        let root;
        if let Some(root_name) = self.grammars.keys().next() {
            root = Rc::new(RefCell::new(AST::Grammar {
                name: root_name.clone(),
                children: vec![],
            }));
            stack.push(root.clone());
        } else {
            return Err(String::from("Parser doesn't have any grammars."));
        }

        let mut buffer = tokens.iter();
        let mut next_token: &Token;

        if let Some(value) = buffer.next() {
            next_token = value;
        } else {
            return Err(String::from("Unexpected end of stream."));
        }

        loop {
            if let Some(rc) = stack.pop() {
                let mut ast = rc.borrow_mut();
                match &mut ast.deref_mut() {
                    AST::Grammar {
                        name,
                        ref mut children,
                    } => match self.table.get(&(name.clone(), next_token.name.clone())) {
                        Some(variant) => {
                            if is_epsilon(variant) {
                                continue;
                            } else {
                                let mut nodes = variant.clone();
                                nodes.reverse();

                                for variant_node in nodes.iter() {
                                    let child: Rc<RefCell<AST>>;

                                    match variant_node {
                                        NodeType::Token { name, .. } => {
                                            child = Rc::new(RefCell::new(AST::Token {
                                                name: name.clone(),
                                                value: String::new(),
                                            }));
                                        }
                                        NodeType::Grammar { name } => {
                                            child = Rc::new(RefCell::new(AST::Grammar {
                                                name: name.clone(),
                                                children: vec![],
                                            }));
                                        }
                                    }

                                    let clone = child.clone();
                                    children.insert(0, child);
                                    stack.push(clone);
                                }
                            }
                        }
                        None => {
                            return unexpected_token(&next_token);
                        }
                    },
                    AST::Token { name, .. } => {
                        if name.clone().eq(&next_token.name.clone()) {
                            if name == EOF {
                                break;
                            }

                            *ast = AST::Token {
                                name: name.clone(),
                                value: next_token.value.clone(),
                            };

                            if let Some(next) = buffer.next() {
                                next_token = next;
                            } else {
                                return Err(String::from("Unexpected end of stream."));
                            }
                        } else {
                            return unexpected_token(&next_token);
                        }
                    }
                }
            } else {
                return unexpected_token(&next_token);
            }
        }

        if let AST::Grammar { ref children, .. } = root.borrow_mut().deref_mut() {
            // Remove ROOT grammar from the AST since it's an internal grammar used for parsing.
            return Ok(children.first().unwrap().clone());
        }

        Ok(root)
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
                        if name == EPSILON {
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
                                    if node == EPSILON {
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
            if name == EPSILON {
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
        if token != EPSILON {
            let mut nodes: GrammarVariant = vec![];
            for node in variant.iter() {
                nodes.push(node.clone());
            }
            table.insert((grammar.clone(), token.clone()), nodes);
        }
    }
}

fn unexpected_token(token: &Token) -> Result<Rc<RefCell<AST>>, String> {
    Err(format!(
        "Unexpected token {} on line {}, column {}.",
        token.value.escape_default(), token.line, token.column
    ))
}