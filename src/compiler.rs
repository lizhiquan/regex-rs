use crate::parser::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{cell::RefCell, rc::Rc};

static COUNTER: AtomicUsize = AtomicUsize::new(1);

pub(crate) struct FSM {
    pub(crate) start: StateRef,
    pub(crate) end: StateRef,
}

impl FSM {
    fn new(condition: Condition) -> FSM {
        let start = State::new();
        let end = State::new();
        start.borrow_mut().transitions.push(Transition {
            condition,
            target: end.clone(),
        });
        FSM { start, end }
    }
}

pub(crate) struct State {
    pub(crate) id: usize,
    pub(crate) transitions: Vec<Transition>,
    pub(crate) epsilon_transitions: Vec<StateRef>,
}

type StateRef = Rc<RefCell<State>>;

impl State {
    fn new() -> StateRef {
        Rc::new(RefCell::new(State {
            id: COUNTER.fetch_add(1, Ordering::Relaxed),
            transitions: Vec::new(),
            epsilon_transitions: Vec::new(),
        }))
    }
}

pub(crate) struct Transition {
    pub(crate) condition: Condition,
    pub(crate) target: StateRef,
}

#[derive(Clone)]
pub(crate) struct Cursor<'a> {
    pub(crate) string: &'a str,
    pub(crate) index: usize,
}

impl Cursor<'_> {
    pub(crate) fn new(string: &str) -> Cursor {
        Cursor { string, index: 0 }
    }

    fn char(&self) -> Option<char> {
        self.string.chars().nth(self.index)
    }

    pub(crate) fn is_end(&self) -> bool {
        self.index >= self.string.len()
    }

    pub(crate) fn advance(&mut self, n: usize) {
        self.index += n;
    }
}

type Condition = Box<dyn Fn(&Cursor) -> ConditionResult>;

pub(crate) enum ConditionResult {
    Accepted(usize),
    Rejected,
}

fn match_character(c: char, case_insensitive: bool) -> Condition {
    Box::new(move |cursor: &Cursor| {
        let ch = match cursor.char() {
            Some(c) => c,
            None => return ConditionResult::Rejected,
        };
        if case_insensitive && ch.eq_ignore_ascii_case(&c) || ch == c {
            ConditionResult::Accepted(1)
        } else {
            ConditionResult::Rejected
        }
    })
}

fn match_digit() -> Condition {
    Box::new(move |cursor: &Cursor| {
        let ch = match cursor.char() {
            Some(c) => c,
            None => return ConditionResult::Rejected,
        };
        if ch.is_ascii_digit() {
            ConditionResult::Accepted(1)
        } else {
            ConditionResult::Rejected
        }
    })
}

fn match_word() -> Condition {
    Box::new(move |cursor: &Cursor| {
        let ch = match cursor.char() {
            Some(c) => c,
            None => return ConditionResult::Rejected,
        };
        if ch.is_ascii_alphanumeric() || ch == '_' {
            ConditionResult::Accepted(1)
        } else {
            ConditionResult::Rejected
        }
    })
}

fn match_any() -> Condition {
    Box::new(|cursor: &Cursor| {
        if cursor.is_end() {
            ConditionResult::Rejected
        } else {
            ConditionResult::Accepted(1)
        }
    })
}

fn match_start_of_string() -> Condition {
    Box::new(move |cursor: &Cursor| {
        if cursor.index == 0 {
            ConditionResult::Accepted(0)
        } else {
            ConditionResult::Rejected
        }
    })
}

fn match_end_of_string() -> Condition {
    Box::new(move |cursor: &Cursor| {
        if cursor.is_end() {
            ConditionResult::Accepted(0)
        } else {
            ConditionResult::Rejected
        }
    })
}

fn match_group(negative: bool, items: Vec<CharacterGroupItem>, case_insensitive: bool) -> Condition {
    Box::new(move |cursor: &Cursor| {
        let ch = match cursor.char() {
            Some(c) => c,
            None => return ConditionResult::Rejected,
        };
        let mut result = items.iter().any(|item| match item {
            CharacterGroupItem::Char(c) => {
                if case_insensitive {
                    c.eq_ignore_ascii_case(&ch)
                } else {
                    c == &ch
                }
            }
            CharacterGroupItem::Digit => ch.is_ascii_digit(),
            CharacterGroupItem::Word => ch.is_ascii_alphanumeric() || ch == '_',
        });
        if negative {
            result = !result
        }
        if result {
            ConditionResult::Accepted(1)
        } else {
            ConditionResult::Rejected
        }
    })
}

pub(crate) fn compile(unit: Unit) -> FSM {
    let case_insensitive = true;
    match unit {
        Unit::ImplicitGroup(children) => concat(children.into_iter().map(compile).collect()),
        Unit::Group { index, children } => concat(children.into_iter().map(compile).collect()),
        Unit::Backreference(index) => backreference(index),
        Unit::Alternation(children) => alternation(children.into_iter().map(compile).collect()),
        Unit::CharacterClass(c) => match c {
            CharacterClass::Char(c) => FSM::new(match_character(c, case_insensitive)),
            CharacterClass::Digit => FSM::new(match_digit()),
            CharacterClass::Word => FSM::new(match_word()),
            CharacterClass::Wildcard => FSM::new(match_any()),
            CharacterClass::Group { negative, items } => FSM::new(match_group(negative, items, case_insensitive)),
        },
        Unit::Anchor(a) => match a {
            Anchor::StartOfString => FSM::new(match_start_of_string()),
            Anchor::EndOfString => FSM::new(match_end_of_string()),
        },
        Unit::QuantifiedExpr { expr, quantifier } => match quantifier {
            Quantifier::ZeroOrOne => zero_or_one(compile(*expr)),
            Quantifier::ZeroOrMore => zero_or_more(compile(*expr)),
            Quantifier::OneOrMore => one_or_more(compile(*expr)),
            _ => panic!("not implemented: {:?}", quantifier),
        },
        _ => panic!("not implemented: {:?}", unit),
    }
}

fn alternation(machines: Vec<FSM>) -> FSM {
    let start = State::new();
    let end = State::new();

    for machine in machines {
        start.borrow_mut().epsilon_transitions.push(machine.start);
        machine.end.borrow_mut().epsilon_transitions.push(end.clone());
    }

    FSM { start, end }
}

fn concat(machines: Vec<FSM>) -> FSM {
    fn concat_pair(lhs: FSM, rhs: FSM) -> FSM {
        lhs.end.borrow_mut().epsilon_transitions.push(rhs.start.clone());
        FSM {
            start: lhs.start,
            end: rhs.end,
        }
    }

    machines.into_iter().reduce(concat_pair).unwrap()
}

fn zero_or_more(machine: FSM) -> FSM {
    let start = State::new();
    let end = State::new();

    // Kleene Star
    start.borrow_mut().epsilon_transitions.push(machine.start.clone());
    start.borrow_mut().epsilon_transitions.push(end.clone());
    machine.end.borrow_mut().epsilon_transitions.push(end.clone());
    machine.end.borrow_mut().epsilon_transitions.push(machine.start.clone());

    FSM { start, end }
}

fn one_or_more(machine: FSM) -> FSM {
    let start = State::new();
    let end = State::new();

    start.borrow_mut().epsilon_transitions.push(machine.start.clone());
    machine.end.borrow_mut().epsilon_transitions.push(end.clone());
    machine.end.borrow_mut().epsilon_transitions.push(machine.start.clone());

    FSM { start, end }
}

fn zero_or_one(machine: FSM) -> FSM {
    let start = State::new();
    let end = State::new();

    start.borrow_mut().epsilon_transitions.push(machine.start.clone());
    start.borrow_mut().epsilon_transitions.push(end.clone());
    machine.end.borrow_mut().epsilon_transitions.push(end.clone());

    FSM { start, end }
}

fn backreference(index: usize) -> FSM {
    todo!()
}
