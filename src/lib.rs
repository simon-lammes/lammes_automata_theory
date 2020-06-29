use std::collections::HashMap;

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
    transitions: HashMap<String, HashMap<char, String>>
}

impl Dfa {
    pub fn test(&self, input: &str) -> bool {
        let mut current_state: String = self.start_state.clone();
        for char in input.chars() {
            let next_state_option = self.transitions.get(&current_state[..])
                .and_then(|state| state.get(&char));
            match next_state_option {
                Some(next_state) => {
                    current_state = next_state.to_string();
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
    use crate::{Dfa};
    use std::collections::HashMap;

    #[test]
    fn test_dfa() {
        let mut transitions = HashMap::new();
        let mut q0_transitions = HashMap::new();
        q0_transitions.insert('1', "q1".to_string());
        q0_transitions.insert('0', "q0".to_string());
        let mut q1_transitions = HashMap::new();
        q1_transitions.insert('1', "q1".to_string());
        transitions.insert("q0".to_string(), q0_transitions);
        transitions.insert("q1".to_string(), q1_transitions);
        let dfa = Dfa {
            name: String::from("Accept if all '1' characters are placed at the end and there is at least one '1' character."),
            start_state: "q0".to_string(),
            accept_states: vec!["q1".to_string()],
            transitions,
        };
        assert!(dfa.test("000111"), "Should accept if there are at least one '1' characters and they are all at the end");
        assert!(!dfa.test("00010"), "Should not accept if input does not end with '1'.");
        assert!(!dfa.test("0101"), "Should not accept if there are '1' characters which are not placed at the end.");
    }
}
