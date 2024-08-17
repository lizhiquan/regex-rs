use anyhow::{anyhow, Result};
use std::{fmt, iter::Peekable, str::Chars};

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum CharacterClass {
    Char(char),
    // String(String),
    Digit,    // \d
    Word,     // \w
    Wildcard, // .
    Group {
        negative: bool,
        items: Vec<CharacterGroupItem>,
    }, // [abc] [^abc]
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum CharacterGroupItem {
    Digit, // \d
    Word,  // \w
    Char(char),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Anchor {
    StartOfString, // ^
    EndOfString,   // $
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Quantifier {
    OneOrMore,                   // +
    ZeroOrMore,                  // *
    ZeroOrOne,                   // ?
    Exact(usize),                // {n}
    Range(usize, Option<usize>), // {n,m}
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Unit {
    ImplicitGroup(Vec<Unit>),
    Group { index: i32, children: Vec<Unit> },
    CharacterClass(CharacterClass),
    Anchor(Anchor),
    QuantifiedExpr { expr: Box<Unit>, quantifier: Quantifier },
    Alternation(Vec<Unit>), // a|b
    Backreference(usize),   // (a)\1
}

fn fmt_with_indent(u: &Unit, f: &mut fmt::Formatter<'_>, indent: usize) -> fmt::Result {
    let indent_str = " ".repeat(indent);

    match u {
        Unit::ImplicitGroup(children) => {
            writeln!(f, "{}- Expression", indent_str)?;
            for child in children {
                fmt_with_indent(child, f, indent + 2)?;
            }
        }
        Unit::Group { index, children } => {
            writeln!(f, "{}- Group(index: {})", indent_str, index)?;
            for child in children {
                fmt_with_indent(child, f, indent + 2)?;
            }
        }
        Unit::CharacterClass(c) => match c {
            CharacterClass::Char(c) => writeln!(f, "{}- Char({})", indent_str, c)?,
            // CharacterClass::String(s) => writeln!(f, "{}- String(\"{}\")", indent_str, s)?,
            CharacterClass::Digit => writeln!(f, "{}- DigitClass", indent_str)?,
            CharacterClass::Word => writeln!(f, "{}- WordClass", indent_str)?,
            CharacterClass::Wildcard => writeln!(f, "{}- Wildcard", indent_str)?,
            CharacterClass::Group { negative, items } => {
                writeln!(f, "{}- CharacterGroup(negative: {})", indent_str, negative)?;
                let indent_str = " ".repeat(indent + 2);
                for item in items {
                    match item {
                        CharacterGroupItem::Char(c) => writeln!(f, "{}  Char({})", indent_str, c)?,
                        CharacterGroupItem::Digit => writeln!(f, "{}  DigitClass", indent_str)?,
                        CharacterGroupItem::Word => writeln!(f, "{}  WordClass", indent_str)?,
                    }
                }
            }
        },
        Unit::Anchor(a) => writeln!(f, "{}- Anchor({:?})", indent_str, a)?,
        Unit::QuantifiedExpr { expr, quantifier } => {
            writeln!(f, "{}- QuantifiedExpr({:?})", indent_str, quantifier)?;
            fmt_with_indent(expr, f, indent + 2)?;
        }
        Unit::Alternation(children) => {
            writeln!(f, "{}- Alternation", indent_str)?;
            for child in children {
                fmt_with_indent(child, f, indent + 2)?;
            }
        }
        Unit::Backreference(i) => writeln!(f, "{}- Backreference(index: {})", indent_str, i)?,
    }

    Ok(())
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_with_indent(self, f, 0)
    }
}

pub(crate) struct Parser<'a> {
    iter: Peekable<Chars<'a>>,
    group_index: i32,
}

impl Parser<'_> {
    pub(crate) fn new(pattern: &str) -> Parser {
        Parser {
            iter: pattern.chars().peekable(),
            group_index: 1,
        }
    }

    pub(crate) fn parse(&mut self) -> Result<Unit> {
        let mut units = Vec::new();
        if self.is_match('^') {
            units.push(Unit::Anchor(Anchor::StartOfString));
        }

        let expr = self.expression()?;
        units.push(expr);

        if self.iter.peek().is_some() {
            return Err(anyhow!("unexpected character: '{}'", self.iter.peek().unwrap()));
        }

        if units.len() == 1 {
            return Ok(units[0].clone());
        }

        Ok(Unit::ImplicitGroup(units))
    }

    fn expression(&mut self) -> Result<Unit> {
        let expr = self.subexpression()?;
        if self.iter.peek() != Some(&'|') {
            return Ok(expr);
        }

        let mut exprs = vec![expr];
        while self.is_match('|') {
            let expr = self.expression()?;
            exprs.push(expr);
        }

        Ok(Unit::Alternation(exprs))
    }

    fn subexpression(&mut self) -> Result<Unit> {
        let item = self.subexpression_item()?;
        if item.is_none() {
            return Err(anyhow!("expected subexpression"));
        }

        let mut exprs = vec![item.unwrap()];
        while let Some(e) = self.subexpression_item()? {
            // if let Unit::CharacterClass(CharacterClass::Char(cur)) = e {
            //     if let Unit::CharacterClass(CharacterClass::Char(prev)) = exprs.last().unwrap() {
            //         let str = format!("{}{}", prev, cur);
            //         exprs.pop();
            //         exprs.push(Unit::CharacterClass(CharacterClass::String(str)));
            //         continue;
            //     }

            //     if let Unit::CharacterClass(CharacterClass::String(str)) = exprs.last_mut().unwrap() {
            //         str.push(cur);
            //         continue;
            //     }
            // }

            // if let Unit::Quantifier(q) = e {
            //     let prev = exprs.pop().unwrap();
            //     exprs.push(Unit::QuantifiedExpr {
            //         expr: Box::new(prev),
            //         quantifier: q,
            //     });
            //     continue;
            // }

            exprs.push(e);
        }

        if exprs.len() == 1 {
            return Ok(exprs[0].clone());
        }

        Ok(Unit::ImplicitGroup(exprs))
    }

    fn subexpression_item(&mut self) -> Result<Option<Unit>> {
        if self.is_match('(') {
            return self.group().map(Some);
        }

        let result = self.anchor()?;
        if result.is_some() {
            return Ok(result);
        }

        let result = self.character_class()?;
        if result.is_some() {
            return Ok(result);
        }

        let result = self.backreference()?;
        if result.is_some() {
            return Ok(result);
        }

        Ok(None)
    }

    fn anchor(&mut self) -> Result<Option<Unit>> {
        if self.is_match('$') {
            return Ok(Some(Unit::Anchor(Anchor::EndOfString)));
        }

        Ok(None)
    }

    fn character_class(&mut self) -> Result<Option<Unit>> {
        let mut item = self.character_class_item()?;
        if item.is_none() {
            return Ok(None);
        }

        if let Some(quantifier) = self.quantifier()? {
            item = Some(Unit::QuantifiedExpr {
                expr: Box::new(item.unwrap()),
                quantifier,
            });
        }

        Ok(item)
    }

    fn character_class_item(&mut self) -> Result<Option<Unit>> {
        if self.is_match('.') {
            return Ok(Some(Unit::CharacterClass(CharacterClass::Wildcard)));
        }

        if self.is_match('[') {
            return self.character_group().map(Some);
        }

        self.character_group_item().map(|x| match x {
            Some(CharacterGroupItem::Char(c)) => Some(Unit::CharacterClass(CharacterClass::Char(c))),
            Some(CharacterGroupItem::Digit) => Some(Unit::CharacterClass(CharacterClass::Digit)),
            Some(CharacterGroupItem::Word) => Some(Unit::CharacterClass(CharacterClass::Word)),
            None => None,
        })
    }

    fn character_group_item(&mut self) -> Result<Option<CharacterGroupItem>> {
        let mut iter = self.iter.clone();
        if iter.next() == Some('\\') && iter.next().map_or(false, |x| !x.is_ascii_digit()) {
            self.iter.next();
            match self.iter.next().unwrap() {
                'd' => return Ok(Some(CharacterGroupItem::Digit)),
                'w' => return Ok(Some(CharacterGroupItem::Word)),
                c => return Ok(Some(CharacterGroupItem::Char(c))),
            }
        }

        if self.iter.peek().map_or(false, |&x| ![']', ')', '|', '\\'].contains(&x)) {
            let c = self.iter.next().unwrap();
            return Ok(Some(CharacterGroupItem::Char(c)));
        }

        Ok(None)
    }

    fn character_group(&mut self) -> Result<Unit> {
        let mut negative_modifier = false;
        if self.is_match('^') {
            negative_modifier = true;
        }

        let item = self.character_group_item()?;
        if item.is_none() {
            return Err(anyhow!("expected character group item"));
        }

        let mut items = vec![item.unwrap()];
        loop {
            if self.is_match(']') {
                break;
            }
            if self.iter.peek().is_none() {
                return Err(anyhow!("expected ]')"));
            }

            let item = self.character_group_item()?;
            if item.is_none() {
                return Err(anyhow!("expected character group item"));
            }
            items.push(item.unwrap());
        }

        Ok(Unit::CharacterClass(CharacterClass::Group {
            negative: negative_modifier,
            items,
        }))
    }

    fn backreference(&mut self) -> Result<Option<Unit>> {
        let mut iter = self.iter.clone();
        if iter.next() != Some('\\') || iter.next().map_or(false, |x| !x.is_ascii_digit()) {
            return Ok(None);
        }

        self.iter.next();
        let mut digits = String::new();
        while let Some(&d) = self.iter.peek() {
            if !d.is_ascii_digit() {
                break;
            }
            digits.push(d);
            self.iter.next();
        }
        let index = digits.parse::<usize>()?;
        Ok(Some(Unit::Backreference(index)))
    }

    fn group(&mut self) -> Result<Unit> {
        let index = self.group_index;
        self.group_index += 1;

        let expr = self.expression()?;
        self.consume(')')?;
        let group = Unit::Group {
            index,
            children: vec![expr],
        };

        if let Ok(Some(quantifier)) = self.quantifier() {
            return Ok(Unit::QuantifiedExpr {
                expr: Box::new(group),
                quantifier,
            });
        }

        Ok(group)
    }

    fn quantifier(&mut self) -> Result<Option<Quantifier>> {
        if self.is_match('*') {
            return Ok(Some(Quantifier::ZeroOrMore));
        }

        if self.is_match('+') {
            return Ok(Some(Quantifier::OneOrMore));
        }

        if self.is_match('?') {
            return Ok(Some(Quantifier::ZeroOrOne));
        }

        // TODO: range

        Ok(None)
    }

    fn is_match(&mut self, c: char) -> bool {
        match self.iter.peek() {
            Some(&ch) if ch == c => {
                self.iter.next();
                true
            }
            _ => false,
        }
    }

    fn consume(&mut self, c: char) -> Result<()> {
        if let Some(&ch) = self.iter.peek() {
            if ch == c {
                self.iter.next();
                return Ok(());
            }
        }
        Err(anyhow!("expected '{}'", c))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_regex() {
        // let mut parser = Parser::new("('(cat) and \\2') is the same as \\1");
        let mut parser = Parser::new("the ((red|blue) pill)$");
        let result = parser.parse();
        println!("{}", result.unwrap());
    }
}
