mod parser;
mod tokenizer;

use crate::parser::Parser;
use crate::tokenizer::Tokenizer;

fn main() {
    let tokenizer = Tokenizer::from_file("tokens.txt").unwrap();

    println!("Patterns: ");
    for pattern in tokenizer.patterns.iter() {
        println!("{} = {}", pattern.name, pattern.value);
    }

    println!();
    println!("Result: ");
    match tokenizer.parse("(userName sw \"Steven\") and (primary eq true)") {
        Ok(result) => {
            for token in result.iter() {
                println!("{}", token)
            }
        }
        Err(err) => {
            println!("{}", err);
        }
    }

    let parser = Parser::from_file("grammar.txt", tokenizer).unwrap();

    println!();
    println!("Grammars: ");
    for (name, variants) in parser.grammar.iter() {
        for variant in variants.iter() {
            let body: String = variant
                .iter()
                .map(|variant| variant.to_string())
                .collect::<Vec<String>>()
                .join(" ");

            println!("{} -> {}", name, body);
        }
    }
}
