use clap::Parser as CLIParser;

use crate::parser::{AST, Parser};
use crate::tokenizer::Tokenizer;

mod parser;
mod tokenizer;

#[derive(CLIParser)]
struct Cli {
    token_path: String,
    grammar_path: String,
    content: String
}


fn main() {
    let args = Cli::parse();

    let tokenizer = Tokenizer::from_file(args.token_path.as_str()).unwrap();

    println!("Patterns: ");
    for pattern in tokenizer.patterns.iter() {
        println!("{} = {}", pattern.name, pattern.value);
    }

    println!();
    println!("Tokens: ");
    match tokenizer.parse(args.content.as_str()) {
        Ok(result) => {
            for token in result.iter() {
                println!("{}", token)
            }
        }
        Err(err) => {
            println!("{}", err);
        }
    }

    let parser = Parser::from_file(args.grammar_path.as_str(), tokenizer).unwrap();

    println!();
    println!("Grammars: ");
    for (name, variants) in parser.grammars.iter() {
        for variant in variants.iter() {
            let body: String = variant
                .iter()
                .map(|variant| variant.to_string())
                .collect::<Vec<String>>()
                .join(" ");

            println!("{} -> {}", name, body);
        }
    }

    println!();
    println!("FIRST:");
    for (k, v) in parser.first.iter() {
        let values: Vec<&str> = v.iter().map(|value| value.as_str()).collect();
        println!("FIRST({}) = {}", k, values.join(", "))
    }

    println!();
    println!("FOLLOW:");
    for (k, v) in parser.follow.iter() {
        let values: Vec<&str> = v.iter().map(|value| value.as_str()).collect();
        println!("FOLLOW({}) = {}", k, values.join(", "));
    }

    println!();
    println!("PARSING TABLE:");
    for ((grammar_name, token_name), variant) in parser.table.iter() {
        let nodes: Vec<String> = variant.iter().map(|node| node.to_string()).collect();
        println!(
            "({grammar_name}, {token_name}) = {grammar_name} -> {}",
            nodes.join(" ")
        )
    }

    let result = parser.parse(args.content.as_str());
    println!();
    match result {
        Ok(ast) => {
            println!("Result: ");
            print_ast(&ast.borrow(), 0);
        }
        Err(err) => {
            println!("Parsing error:");
            println!("{}", err);
        }
    }
}


fn print_ast(ast: &AST, level: usize) {
    let indent = "  ".repeat(level);

    match &ast {
        AST::Token { value, .. } => {
           println!("{indent}{value}");
        }
        AST::Grammar { name, children } => {
            println!("{indent}{name}");
            for item in children.iter() {
                print_ast(&item.borrow(), level + 1);
            }
        }
    }
}