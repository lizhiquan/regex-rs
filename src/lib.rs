use compiler::Compiler;
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
        let machine = Compiler::compile(&unit);
        let mut matcher = Matcher::new(machine, text);
        matcher.matches()
    }
}

#[cfg(test)]
mod tests {
    use crate::Regex;

    #[test]
    fn it_works() {
        let regex = Regex::new(r"((\w\w\w\w) (\d\d\d)) is doing \2 \3 times, and again \1 times");
        assert!(regex.matches("grep 101 is doing grep 101 times, and again grep 101 times"));
    }
}
