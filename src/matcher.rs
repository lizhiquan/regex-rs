use crate::compiler::{ConditionResult, Cursor, FSM};

pub(crate) struct Matcher<'a> {
    fsm: FSM,
    cursor: Cursor<'a>,
}

impl Matcher<'_> {
    pub(crate) fn new(fsm: FSM, pattern: &str) -> Matcher {
        let cursor = Cursor::new(pattern);
        Matcher { fsm, cursor }
    }

    pub(crate) fn matches(&mut self) -> bool {
        // overlapping matches are not supported
        let mut cursor = self.cursor.clone();
        while !self.find(&mut cursor) {
            cursor.advance(1);
            if cursor.is_end() {
                return false;
            }
        }

        cursor.advance(1);
        self.cursor = cursor;
        true
    }

    fn find(&self, cursor: &mut Cursor) -> bool {
        let mut states = vec![self.fsm.start.clone()];
        while !states.is_empty() {
            let mut new_states = Vec::new();
            for state in states {
                for transition in &state.borrow().transitions {
                    if let ConditionResult::Accepted(n) = (transition.condition)(cursor) {
                        if transition.target.borrow().id == self.fsm.end.borrow().id {
                            return true;
                        }

                        cursor.advance(n);
                        new_states.push(transition.target.clone());
                    }
                }

                for transition in &state.borrow().epsilon_transitions {
                    if transition.borrow().id == self.fsm.end.borrow().id {
                        return true;
                    }

                    new_states.push(transition.clone());
                }
            }
            states = new_states;
        }

        false
    }
}
