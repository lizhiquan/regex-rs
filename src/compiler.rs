use crate::parser::*;
use std::collections::{HashMap, HashSet};
use std::fmt;
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
        start.borrow_mut().transitions.push(Transition::new(condition, end.clone()));
        FSM { start, end }
    }

    pub(crate) fn get_all_states(&self) -> Vec<StateRef> {
        let mut visited = HashSet::new();
        let mut states = Vec::new();
        self.collect_states(Rc::clone(&self.start), &mut visited, &mut states);
        states
    }

    fn collect_states(&self, state_ref: StateRef, visited: &mut HashSet<usize>, states: &mut Vec<StateRef>) {
        let state = state_ref.borrow();
        if visited.insert(state.id) {
            states.push(Rc::clone(&state_ref));
            for transition in &state.transitions {
                self.collect_states(Rc::clone(&transition.target), visited, states);
            }
        }
    }
}

impl fmt::Display for FSM {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "FSM: start {} end {}", self.start.borrow().id, self.end.borrow().id)?;
        for state in self.get_all_states() {
            for transition in &state.borrow().transitions {
                writeln!(
                    f,
                    "{} --- {} --> {}",
                    state.borrow().id,
                    transition.condition.name,
                    transition.target.borrow().id
                )?;
            }
        }
        Ok(())
    }
}

pub(crate) struct State {
    pub(crate) id: usize,
    pub(crate) transitions: Vec<Transition>,
}

pub(crate) type StateRef = Rc<RefCell<State>>;

impl State {
    fn new() -> StateRef {
        Rc::new(RefCell::new(State {
            id: COUNTER.fetch_add(1, Ordering::Relaxed),
            transitions: Vec::new(),
        }))
    }
}

pub(crate) struct Transition {
    pub(crate) condition: Condition,
    pub(crate) target: StateRef,
}

impl Transition {
    fn new(condition: Condition, target: StateRef) -> Transition {
        Transition { condition, target }
    }

    fn epsilon(target: StateRef) -> Transition {
        Transition {
            condition: Condition::epsilon(),
            target,
        }
    }
}

#[derive(Clone)]
pub(crate) struct Cursor<'a> {
    text: &'a str,
    pub(crate) index: usize,
    captured_groups: HashMap<usize, &'a str>,
}

impl<'a> Cursor<'a> {
    pub(crate) fn new(text: &str) -> Cursor {
        Cursor {
            text,
            index: 0,
            captured_groups: HashMap::new(),
        }
    }

    pub(crate) fn char(&self) -> Option<char> {
        self.text.chars().nth(self.index)
    }

    pub(crate) fn is_end(&self) -> bool {
        self.index >= self.text.len()
    }

    pub(crate) fn advance(&mut self, n: usize) {
        self.index += n;
    }

    pub(crate) fn add_captured_group(&mut self, index: usize, from: usize, to: usize) {
        self.captured_groups.insert(index, self.text.get(from..to).unwrap());
    }
}

pub(crate) struct Condition {
    name: String,
    pub(crate) evaluate: Box<dyn Fn(&Cursor) -> ConditionResult>,
}

pub(crate) enum ConditionResult {
    Accepted(usize),
    Rejected,
}

impl Condition {
    fn epsilon() -> Condition {
        Condition {
            name: "epsilon".to_string(),
            evaluate: Box::new(|_| ConditionResult::Accepted(0)),
        }
    }

    fn match_character(c: char, case_insensitive: bool) -> Condition {
        Condition {
            name: format!("'{}'", c),
            evaluate: Box::new(move |cursor: &Cursor| {
                let ch = match cursor.char() {
                    Some(c) => c,
                    None => return ConditionResult::Rejected,
                };
                if case_insensitive && ch.eq_ignore_ascii_case(&c) || ch == c {
                    ConditionResult::Accepted(1)
                } else {
                    ConditionResult::Rejected
                }
            }),
        }
    }

    fn match_digit() -> Condition {
        Condition {
            name: "digit".to_string(),
            evaluate: Box::new(move |cursor: &Cursor| {
                let ch = match cursor.char() {
                    Some(c) => c,
                    None => return ConditionResult::Rejected,
                };
                if ch.is_ascii_digit() {
                    ConditionResult::Accepted(1)
                } else {
                    ConditionResult::Rejected
                }
            }),
        }
    }

    fn match_word() -> Condition {
        Condition {
            name: "word".to_string(),
            evaluate: Box::new(move |cursor: &Cursor| {
                let ch = match cursor.char() {
                    Some(c) => c,
                    None => return ConditionResult::Rejected,
                };
                if ch.is_ascii_alphanumeric() || ch == '_' {
                    ConditionResult::Accepted(1)
                } else {
                    ConditionResult::Rejected
                }
            }),
        }
    }

    fn match_any() -> Condition {
        Condition {
            name: "any".to_string(),
            evaluate: Box::new(|cursor: &Cursor| {
                if cursor.is_end() {
                    ConditionResult::Rejected
                } else {
                    ConditionResult::Accepted(1)
                }
            }),
        }
    }

    fn match_start_of_string() -> Condition {
        Condition {
            name: "start_of_string".to_string(),
            evaluate: Box::new(move |cursor: &Cursor| {
                if cursor.index == 0 {
                    ConditionResult::Accepted(0)
                } else {
                    ConditionResult::Rejected
                }
            }),
        }
    }

    fn match_end_of_string() -> Condition {
        Condition {
            name: "end_of_string".to_string(),
            evaluate: Box::new(move |cursor: &Cursor| {
                if cursor.is_end() {
                    ConditionResult::Accepted(0)
                } else {
                    ConditionResult::Rejected
                }
            }),
        }
    }

    fn match_character_group(negative: bool, items: Vec<CharacterGroupItem>, case_insensitive: bool) -> Condition {
        Condition {
            name: format!("character_group {}{:?}", if negative { "^" } else { "" }, items),
            evaluate: Box::new(move |cursor: &Cursor| {
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
            }),
        }
    }

    fn match_captured_group(index: usize) -> Condition {
        Condition {
            name: format!("captured_group[{}]", index),
            evaluate: Box::new(move |cursor: &Cursor| {
                let group = match cursor.captured_groups.get(&index) {
                    Some(g) => g,
                    None => return ConditionResult::Rejected,
                };

                if let Some(substring) = cursor.text.get(cursor.index..) {
                    if substring.starts_with(group) {
                        return ConditionResult::Accepted(group.len());
                    }
                }
                ConditionResult::Rejected
            }),
        }
    }
}

pub(crate) struct CompiledMachine {
    pub(crate) fsm: FSM,
    pub(crate) captured_groups: Vec<CapturedGroup>,
}

pub(crate) struct CapturedGroup {
    pub(crate) index: usize,
    pub(crate) start: StateRef,
    pub(crate) end: StateRef,
}

pub(crate) struct Compiler {
    captured_groups: Vec<CapturedGroup>,
    match_case_insensitive: bool,
}

impl Compiler {
    pub(crate) fn compile(ast: &Unit) -> CompiledMachine {
        let mut compiler = Compiler {
            captured_groups: Vec::new(),
            match_case_insensitive: true,
        };

        CompiledMachine {
            fsm: compiler.compile_unit(ast),
            captured_groups: compiler.captured_groups,
        }
    }

    fn compile_unit(&mut self, unit: &Unit) -> FSM {
        match unit {
            Unit::ImplicitGroup(children) => concat(children.iter().map(|child| self.compile_unit(child)).collect()),
            Unit::Group { index, children } => {
                let fsm = concat(children.iter().map(|child| self.compile_unit(child)).collect());
                let group = CapturedGroup {
                    index: *index,
                    start: fsm.start.clone(),
                    end: fsm.end.clone(),
                };
                self.captured_groups.push(group);
                fsm
            }
            Unit::Backreference(index) => FSM::new(Condition::match_captured_group(*index)),
            Unit::Alternation(children) => alternation(children.iter().map(|child| self.compile_unit(child)).collect()),
            Unit::CharacterClass(c) => match c {
                CharacterClass::Char(c) => FSM::new(Condition::match_character(*c, self.match_case_insensitive)),
                CharacterClass::Digit => FSM::new(Condition::match_digit()),
                CharacterClass::Word => FSM::new(Condition::match_word()),
                CharacterClass::Wildcard => FSM::new(Condition::match_any()),
                CharacterClass::Group { negative, items } => {
                    FSM::new(Condition::match_character_group(*negative, items.clone(), self.match_case_insensitive))
                }
            },
            Unit::Anchor(a) => match a {
                Anchor::StartOfString => FSM::new(Condition::match_start_of_string()),
                Anchor::EndOfString => FSM::new(Condition::match_end_of_string()),
            },
            Unit::QuantifiedExpr { expr, quantifier } => match quantifier {
                Quantifier::ZeroOrOne => zero_or_one(self.compile_unit(expr)),
                Quantifier::ZeroOrMore => zero_or_more(self.compile_unit(expr)),
                Quantifier::OneOrMore => one_or_more(self.compile_unit(expr)),
                _ => panic!("not implemented: {:?}", quantifier),
            },
        }
    }
}

fn alternation(machines: Vec<FSM>) -> FSM {
    let start = State::new();
    let end = State::new();

    for machine in machines {
        start.borrow_mut().transitions.push(Transition::epsilon(machine.start));
        machine.end.borrow_mut().transitions.push(Transition::epsilon(end.clone()));
    }

    FSM { start, end }
}

fn concat(machines: Vec<FSM>) -> FSM {
    fn concat_pair(lhs: FSM, rhs: FSM) -> FSM {
        lhs.end.borrow_mut().transitions.push(Transition::epsilon(rhs.start.clone()));
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
    start.borrow_mut().transitions.push(Transition::epsilon(machine.start.clone()));
    start.borrow_mut().transitions.push(Transition::epsilon(end.clone()));
    machine.end.borrow_mut().transitions.push(Transition::epsilon(end.clone()));
    machine.end.borrow_mut().transitions.push(Transition::epsilon(machine.start.clone()));

    FSM { start, end }
}

fn one_or_more(machine: FSM) -> FSM {
    let start = State::new();
    let end = State::new();

    start.borrow_mut().transitions.push(Transition::epsilon(machine.start.clone()));
    machine.end.borrow_mut().transitions.push(Transition::epsilon(end.clone()));
    machine.end.borrow_mut().transitions.push(Transition::epsilon(machine.start.clone()));

    FSM { start, end }
}

fn zero_or_one(machine: FSM) -> FSM {
    let start = State::new();
    let end = State::new();

    start.borrow_mut().transitions.push(Transition::epsilon(machine.start.clone()));
    start.borrow_mut().transitions.push(Transition::epsilon(end.clone()));
    machine.end.borrow_mut().transitions.push(Transition::epsilon(end.clone()));

    FSM { start, end }
}
