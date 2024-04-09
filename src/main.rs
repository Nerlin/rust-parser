mod tokenizer;

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
}
