use std::collections::HashMap;

/// Describes to which next state a DFA switches when it reads a certain input while being in
/// a certain state.
struct Transition {
    state: String,
    input: char,
    next_state: String,
}

/// # [Deterministic finite acceptor](https://en.wikipedia.org/wiki/Deterministic_finite_automaton)
/// The DFA is modelled slightly different than in its mathematical model.
struct Dfa {
    name: String,
    start_state: String,
    accept_states: Vec<String>,
    /// Maps the state names to a second map, which maps the input character to the next state name.
    /// Example: The state 'start' is mapped to a second map which maps the input character '1' to
    /// 'started', meaning that when the DFA reads a '1' while being in state 'start', it will
    /// switch to state 'started'.
    transitions: Vec<Transition>,
}

impl Dfa {
    /// Checks whether a certain input is accepted by the DFA.
    pub fn check(&self, input: &str) -> bool {
        let mut current_state: String = self.start_state.clone();
        // Go over each character and find suitable transitions for the state.
        for char in input.chars() {
            let next_transition_option = self.transitions.iter()
                .find(|transition| transition.state.eq(&current_state) && transition.input.eq(&char));
            match next_transition_option {
                Some(next_transition) => {
                    current_state = next_transition.next_state.to_string();
                }
                None => {
                    // The next state cannot be determined, which means that we are in an error state.
                    return false;
                }
            }
        }
        self.accept_states.contains(&current_state)
    }
}


#[cfg(test)]
mod dfa_tests {
    use std::collections::HashMap;

    use crate::{Dfa, Transition};

    #[test]
    fn test_dfa() {
        let dfa = Dfa {
            name: String::from("Accept if all '1' characters are placed at the end and there is at least one '1' character."),
            start_state: "q0".to_string(),
            accept_states: vec!["q1".to_string()],
            transitions: vec![
                Transition {
                    state: "q0".to_string(),
                    input: '0',
                    next_state: "q0".to_string()
                },
                Transition {
                    state: "q0".to_string(),
                    input: '1',
                    next_state: "q1".to_string()
                },
                Transition {
                    state: "q1".to_string(),
                    input: '1',
                    next_state: "q1".to_string()
                },
            ],
        };
        assert!(dfa.check("000111"), "Should accept if there are at least one '1' characters and they are all at the end");
        assert!(!dfa.check("00010"), "Should not accept if input does not end with '1'.");
        assert!(!dfa.check("0101"), "Should not accept if there are '1' characters which are not placed at the end.");
    }
}
