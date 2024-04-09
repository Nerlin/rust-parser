use std::collections::HashMap;

struct Lexem {
    name: str,
    pattern: str
}


struct Lexer {
    patterns: HashMap<str, Lexem>
}

impl Lexer {
    fn from_file(path: &str) -> Lexer {
        
    }
}