use std::collections::HashMap;

use crate::compiler::{CompiledMachine, ConditionResult, Cursor, StateRef};

pub(crate) struct Matcher<'a> {
    machine: CompiledMachine,
    cursor: Cursor<'a>,
    start_captured_groups: HashMap<usize, Vec<usize>>, // map a start state id to its captured group indices
    end_captured_groups: HashMap<usize, Vec<usize>>,   // map an end state id to its captured group indices
}

impl Matcher<'_> {
    pub(crate) fn new(machine: CompiledMachine, text: &str) -> Matcher {
        let cursor = Cursor::new(text);
        let mut start_captured_groups = HashMap::new();
        let mut end_captured_groups = HashMap::new();

        for (index, group) in machine.captured_groups.iter().enumerate() {
            start_captured_groups.entry(group.start.borrow().id).or_insert_with(Vec::new).push(index);
            end_captured_groups.entry(group.end.borrow().id).or_insert_with(Vec::new).push(index);
        }

        Matcher {
            machine,
            cursor,
            start_captured_groups,
            end_captured_groups,
        }
    }

    pub(crate) fn matches(&mut self) -> bool {
        // overlapping matches are not supported
        let mut cursor = self.cursor.clone();
        let mut start_captured_group_indices = HashMap::new();
        while !self.try_match(&mut cursor, self.machine.fsm.start.clone(), &mut start_captured_group_indices) {
            cursor.advance(1);
            if cursor.is_end() {
                return false;
            }
            start_captured_group_indices.clear();
        }

        self.cursor = cursor;
        true
    }

    fn try_match(&self, cursor: &mut Cursor, state: StateRef, start_captured_group_indices: &mut HashMap<usize, usize>) -> bool {
        // println!("{:?} '{}'", state.borrow().id, cursor.char().unwrap_or_default());

        if state.borrow().id == self.machine.fsm.end.borrow().id {
            return true;
        }

        if let Some(indices) = self.start_captured_groups.get(&state.borrow().id) {
            for &i in indices {
                let group = &self.machine.captured_groups[i];
                start_captured_group_indices.insert(group.index, cursor.index);
            }
        }

        if let Some(indices) = self.end_captured_groups.get(&state.borrow().id) {
            for &i in indices {
                let group = &self.machine.captured_groups[i];
                if let Some(&start_index) = start_captured_group_indices.get(&group.index) {
                    cursor.add_captured_group(group.index, start_index, cursor.index)
                }
            }
        }

        for transition in &state.borrow().transitions {
            if let ConditionResult::Accepted(n) = (transition.condition.evaluate)(cursor) {
                cursor.advance(n);
                let mut cloned_cursor = cursor.clone();
                if self.try_match(&mut cloned_cursor, transition.target.clone(), start_captured_group_indices) {
                    *cursor = cloned_cursor;
                    return true;
                }
            }
        }

        false
    }
}
