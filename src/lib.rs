use compiler::compile;
use matcher::Matcher;
use parser::Parser;

mod compiler;
mod matcher;
mod parser;

pub struct Regex {
    pub pattern: String,
}

impl Regex {
    pub fn new(pattern: &str) -> Regex {
        Regex {
            pattern: String::from(pattern),
        }
    }

    pub fn matches(&self, text: &str) -> bool {
        let mut parser = Parser::new(&self.pattern);
        let unit = parser.parse().unwrap();
        let fsm = compile(unit);
        let mut matcher = Matcher::new(fsm, text);
        matcher.matches()
    }
}

#[cfg(test)]
mod tests {
    use crate::Regex;

    #[test]
    fn it_works() {
        let regex = Regex::new("\\d \\w\\w\\ws");
        assert!(regex.matches("sally has 1 dog"));
    }
}
