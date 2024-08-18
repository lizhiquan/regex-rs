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
    use super::*;

    fn test(test_cases: &[(&str, &str, bool)]) {
        for (i, test) in test_cases.iter().enumerate() {
            let result = Regex::new(test.0).matches(test.1);
            assert_eq!(result, test.2, "Test case {} failed: ({}, {})", i, test.0, test.1);
        }
    }

    #[test]
    fn simple() {
        let test_cases = vec![("d", "dog", true), ("f", "dog", false)];
        test(&test_cases);
    }

    #[test]
    fn alphanumeric() {
        let test_cases = vec![("\\w", "word", true), ("\\w", "$!?", false)];
        test(&test_cases);
    }

    #[test]
    fn digit() {
        let test_cases = vec![("\\d", "123", true), ("\\d", "apple", false)];
        test(&test_cases);
    }

    #[test]
    fn anchor() {
        let test_cases = vec![
            ("^log", "log", true),
            ("^log", "slog", false),
            ("cat$", "cat", true),
            ("cat$", "cats", false),
        ];
        test(&test_cases);
    }

    #[test]
    fn character_group() {
        let test_cases = vec![
            ("[abcd]", "a", true),
            ("[abcd]", "efgh", false),
            ("[^xyz]", "apple", true),
            ("[^anb]", "banana", false),
        ];
        test(&test_cases);
    }

    #[test]
    fn quantifier() {
        let test_cases = vec![
            ("ca+t", "caaats", true),
            ("ca+t", "cat", true),
            ("ca+t", "act", false),
            ("ca?t", "cat", true),
            ("ca?t", "act", true),
            ("ca?t", "dog", false),
            ("ca?t", "cag", false),
        ];
        test(&test_cases);
    }

    #[test]
    fn wildcard() {
        let test_cases = vec![("c.t", "cat", true), ("c.t", "cot", true), ("c.t", "car", false)];
        test(&test_cases);
    }

    #[test]
    fn character_classes_combined() {
        let test_cases = vec![
            ("\\d apple", "sally has 3 apples", true),
            ("\\d apple", "sally has 1 orange", false),
            ("\\d\\d\\d apples", "sally has 124 apples", true),
            ("\\d\\\\d\\\\d apples", "sally has 12 apples", false),
            ("\\d \\w\\w\\ws", "sally has 3 dogs", true),
            ("\\d \\w\\w\\ws", "sally has 4 dogs", true),
            ("\\d \\w\\w\\ws", "sally has 1 dog", false),
        ];
        test(&test_cases);
    }

    #[test]
    fn alternation() {
        let test_cases = vec![
            ("a (cat|dog)", "a cat", true),
            ("a (cat|dog)", "a dog", true),
            ("a (cat|dog)", "a cow", false),
        ];
        test(&test_cases);
    }

    #[test]
    fn backreference_single() {
        let test_cases = vec![
            ("(cat) and \\1", "cat and cat", true),
            ("(cat) and \\1", "cat and dog", false),
            ("(\\w\\w\\w\\w \\d\\d\\d) is doing \\1 times", "grep 101 is doing grep 101 times", true),
            ("(\\w\\w\\w \\d\\d\\d) is doing \\1 times", "$?! 101 is doing $?! 101 times", false),
            ("(\\w\\w\\w\\w \\d\\d\\d) is doing \\1 times", "grep yes is doing grep yes times", false),
            ("([abcd]+) is \\1, not [^xyz]+", "abcd is abcd, not efg", true),
            ("([abcd]+) is \\1, not [^xyz]+", "efgh is efgh, not efg", false),
            ("([abcd]+) is \\1, not [^xyz]+", "abcd is abcd, not xyz", false),
            ("^(\\w+) starts and ends with \\1$", "this starts and ends with this", true),
            ("^(this) starts and ends with \\1$", "that starts and ends with this", false),
            ("^(this) starts and ends with \\1$", "this starts and ends with this?", false),
            ("once a (drea+mer), alwaysz? a \\1", "once a dreaaamer, always a dreaaamer", true),
            ("once a (drea+mer), alwaysz? a \\1", "once a dremer, always a dreaaamer", false),
            ("once a (drea+mer), alwaysz? a \\1", "once a dreaaamer, alwayszzz a dreaaamer", false),
            ("(b..s|c..e) here and \\1 there", "bugs here and bugs there", true),
            ("(b..s|c..e) here and \\1 there", "bugz here and bugs there", false),
        ];
        test(&test_cases);
    }

    #[test]
    fn backreference_multiple() {
        let test_cases = vec![
            ("(\\d+) (\\w+) squares and \\1 \\2 circles", "3 red squares and 3 red circles", true),
            ("(\\d+) (\\w+) squares and \\1 \\2 circles", "3 red squares and 4 red circles", false),
            (
                "(\\w\\w\\w\\w) (\\d\\d\\d) is doing \\1 \\2 times",
                "grep 101 is doing grep 101 times",
                true,
            ),
            ("(\\w\\w\\w) (\\d\\d\\d) is doing \\1 \\2 times", "$?! 101 is doing $?! 101 times", false),
            (
                "(\\w\\w\\w\\w) (\\d\\d\\d) is doing \\1 \\2 times",
                "grep yes is doing grep yes times",
                false,
            ),
            ("([abc]+)-([def]+) is \\1-\\2, not [^xyz]+", "abc-def is abc-def, not efg", true),
            ("([abc]+)-([def]+) is \\1-\\2, not [^xyz]+", "efg-hij is efg-hij, not efg", false),
            ("([abc]+)-([def]+) is \\1-\\2, not [^xyz]+", "abc-def is abc-def, not xyz", false),
            ("^(\\w+) (\\w+), \\1 and \\2$", "apple pie, apple and pie", true),
            ("^(apple) (\\w+), \\1 and \\2$", "pineapple pie, pineapple and pie", false),
            ("^(\\w+) (pie), \\1 and \\2$", "apple pie, apple and pies", false),
            ("(how+dy) (he?y) there, \\1 \\2", "howwdy hey there, howwdy hey", true),
            ("(how+dy) (he?y) there, \\1 \\2", "hody hey there, howwdy hey", false),
            ("(how+dy) (he?y) there, \\1 \\2", "howwdy heeey there, howwdy heeey", false),
            ("(c.t|d.g) and (f..h|b..d), \\1 with \\2", "cat and fish, cat with fish", true),
            ("(c.t|d.g) and (f..h|b..d), \\1 with \\2", "bat and fish, cat with fish", false),
        ];
        test(&test_cases);
    }

    #[test]
    fn backreference_nested() {
        let test_cases = vec![
            ("('(cat) and \\2') is the same as \\1", "'cat and cat' is the same as 'cat and cat'", true),
            (
                "('(cat) and \\2') is the same as \\1",
                "'cat and cat' is the same as 'cat and dog'",
                false,
            ),
            (
                "((\\w\\w\\w\\w) (\\d\\d\\d)) is doing \\2 \\3 times, and again \\1 times",
                "grep 101 is doing grep 101 times, and again grep 101 times",
                true,
            ),
            (
                "((\\w\\w\\w) (\\d\\d\\d)) is doing \\2 \\3 times, and again \\1 times",
                "$?! 101 is doing $?! 101 times, and again $?! 101 times",
                false,
            ),
            (
                "((\\w\\w\\w\\w) (\\d\\d\\d)) is doing \\2 \\3 times, and again \\1 times",
                "grep yes is doing grep yes times, and again grep yes times",
                false,
            ),
            (
                "(([abc]+)-([def]+)) is \\1, not ([^xyz]+), \\2, or \\3",
                "abc-def is abc-def, not efg, abc, or def",
                true,
            ),
            (
                "(([abc]+)-([def]+)) is \\1, not ([^xyz]+), \\2, or \\3",
                "efg-hij is efg-hij, not klm, efg, or hij",
                false,
            ),
            (
                "(([abc]+)-([def]+)) is \\1, not ([^xyz]+), \\2, or \\3",
                "abc-def is abc-def, not xyz, abc, or def",
                false,
            ),
            (
                "^((\\w+) (\\w+)) is made of \\2 and \\3. love \\1$",
                "apple pie is made of apple and pie. love apple pie",
                true,
            ),
            (
                "^((apple) (\\w+)) is made of \\2 and \\3. love \\1$",
                "pineapple pie is made of apple and pie. love apple pie",
                false,
            ),
            (
                "^((\\w+) (pie)) is made of \\2 and \\3. love \\1$",
                "apple pie is made of apple and pie. love apple pies",
                false,
            ),
            (
                "'((how+dy) (he?y) there)' is made up of '\\2' and '\\3'. \\1",
                "'howwdy hey there' is made up of 'howwdy' and 'hey'. howwdy hey there",
                true,
            ),
            (
                "'((how+dy) (he?y) there)' is made up of '\\2' and '\\3'. \\1",
                "'hody hey there' is made up of 'hody' and 'hey'. hody hey there",
                false,
            ),
            (
                "'((how+dy) (he?y) there)' is made up of '\\2' and '\\3'. \\1",
                "'howwdy heeey there' is made up of 'howwdy' and 'heeey'. howwdy heeey there",
                false,
            ),
            (
                "((c.t|d.g) and (f..h|b..d)), \\2 with \\3, \\1",
                "cat and fish, cat with fish, cat and fish",
                true,
            ),
            (
                "((c.t|d.g) and (f..h|b..d)), \\2 with \\3, \\1",
                "bat and fish, bat with fish, bat and fish",
                false,
            ),
        ];

        test(&test_cases)
    }
}
