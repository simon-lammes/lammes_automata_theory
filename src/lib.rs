use std::borrow::Borrow;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::copy;
use std::iter::FromIterator;
use std::ops::Deref;

use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::RandomState;

/// Describes to which next state a DFA switches when it reads a certain input while being in
/// a certain state.
#[derive(Ord, PartialOrd, Eq, PartialEq, Hash, Clone, Debug, Serialize, Deserialize)]
pub struct Transition {
    state: String,
    input: char,
    next_state: String,
}

/// # [Deterministic finite acceptor](https://en.wikipedia.org/wiki/Deterministic_finite_automaton)
/// The DFA is modelled slightly different than in its mathematical model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dfa {
    name: String,
    start_state: String,
    accept_states: HashSet<String>,
    transitions: Vec<Transition>,
}

impl Dfa {
    /// Checks whether a certain input is accepted by the DFA.
    /// Additionally returns a list of the states that have been traversed while processing the input.
    /// The start_state is included in that list of traversed states.
    pub fn check(&self, input: &str) -> (bool, Vec<String>) {
        let mut traversed_states: Vec<String> = Vec::new();
        // The first traversed state is obviously the start state.
        traversed_states.push(self.start_state.clone());
        // Go over each character and find suitable transitions for the current state.
        for char in input.chars() {
            let current_state = traversed_states.last().unwrap();
            let next_transition_option = self.get_transition(&current_state, &char);
            match next_transition_option {
                Some(next_transition) => {
                    // Transition to the next state.
                    traversed_states.push(next_transition.next_state.to_string());
                }
                None => {
                    // The next state cannot be determined, which means that we are in an error state.
                    return (false, traversed_states);
                }
            }
        }
        // Accepts if the last state we visited (the current state) is an accepting state.
        (self.accept_states.contains(&traversed_states.last().unwrap().clone()), traversed_states)
    }

    /// Tries to find a transition that fits for the current situation. Calling this function is like
    /// saying: What happens when the DFA is in state q0 and reads an A?
    /// If no transition can be found, the DFA is in an error state.
    pub fn get_transition(&self, state: &str, input: &char) -> Option<&Transition> {
        self.transitions.iter()
            .find(|transition| transition.state.eq(&String::from(state)) && transition.input.eq(&input))
    }

    pub fn get_all_input_symbols(&self) -> HashSet<char> {
        HashSet::from_iter(self.transitions.iter().map(|transition| transition.input))
    }

    pub fn get_all_states(&self) -> HashSet<String> {
        HashSet::from_iter(self.transitions.iter().flat_map(|transition| vec![transition.state.clone(), transition.next_state.clone()]))
    }

    /// Minimizes the DFA with the algorithm found on [here.](https://www.geeksforgeeks.org/minimization-of-dfa/)
    /// Usually, when the states "q0" and "q1" are equivalent, you would expect this algorithm to merge them into
    /// a state called something like "q0,q1". This, however, could lead to name collisions as there might already exist
    /// another state called "q0,q1". Therefore, the new name for the merged state would just be "qo". This method concentrates
    /// on reliably minimizing the DFA. Finding suitable names (like "q0,q1") for the merged states,
    /// while avoiding name collisions, might be the job for some other method.
    /// Returns a hash map with all renaming operations. For example, if q0 and q1 are merged into a new
    /// state called q0, the returned object will map the old names qo and q1 to the new name q0.
    pub fn minimize(&mut self) -> HashMap<String, String> {
        self.remove_inaccessible_states();
        let all_input_symbols = self.get_all_input_symbols();
        let rejecting_states = HashSet::from_iter(self.get_all_states().difference(&self.accept_states).map(|x| x.clone()));
        // Initially, states are only split into accepting and rejecting states. Those are obviously distinguishable states that must
        // belong into different equivalence classes.
        let mut equivalence_classes = vec![self.accept_states.clone(), rejecting_states];
        // We further split distinguishable states into separate equivalence classes until we do not find any
        // distinguishable states within one equivalence class any more. Then we know for sure,
        // that every one of our equivalence classes only contains indistinguishable states.
        loop {
            // The following list keeps track of state pairs that are indistinguishable for the current equivalence classes.
            let mut indistinguishable_states_list: Vec<(&String, &String)> = Vec::new();
            for equivalence_class in &equivalence_classes {
                for state_1 in equivalence_class {
                    for state_2 in equivalence_class {
                        let are_indistinguishable = self.are_states_indistinguishable(&state_1, &state_2, &all_input_symbols, &equivalence_classes);
                        if are_indistinguishable {
                            indistinguishable_states_list.push((state_1, state_2))
                        }
                    }
                }
            }
            let mut new_equivalence_classes: Vec<HashSet<String>> = Vec::new();
            for indistinguishable_states in indistinguishable_states_list {
                // Within the new_equivalence_classes, find an equivalence class into which the current indistinguishable_states can be put.
                // If the current indistinguishable_states are (q0, q1) and we know that q1 is indistinguishable from q3 and there is already
                // an equivalence class with q3, we'll put q0 and q1 into that equivalence class. q0, q1 and q3 are indistinguishable and
                // belong in the same equivalence class.
                let equivalence_class_to_put_indistinguishable_states_into = new_equivalence_classes.iter_mut()
                    .find(|class| class.contains(indistinguishable_states.0) || class.contains(indistinguishable_states.1));
                // There are two options: Either a suitable equivalence class already exists or we have to create a new one.
                // We build new equivalence classes because we build new_equivalence_classes from scratch within each iteration.
                match equivalence_class_to_put_indistinguishable_states_into {
                    Some(equivalence_class) => {
                        equivalence_class.insert(indistinguishable_states.0.clone());
                        equivalence_class.insert(indistinguishable_states.1.clone());
                    }
                    None => {
                        new_equivalence_classes.push(HashSet::from_iter(vec![indistinguishable_states.0.clone(), indistinguishable_states.1.clone()]));
                    }
                }
            }
            let has_split_occurred = equivalence_classes.len() < new_equivalence_classes.len();
            if !has_split_occurred {
                break;
            }
            equivalence_classes = new_equivalence_classes;
        }
        // We build a hash map that maps the old names to the new names.
        // If q0 and q1 are indistinguishable and thus in the same equivalence class,
        // q0 will be mapped to q0 and q1 will also be mapped to q0. Thus,
        // the states q0 and q1 are merged into the "new" state q0.
        let mut renaming_operations = HashMap::new();
        for equivalence_class in equivalence_classes {
            // If the equivalence class consists only of one state, there is no merge necessary and thus
            // renaming operation for that equivalence class.
            if equivalence_class.len() <= 1 {
                continue;
            }
            let new_name = equivalence_class.iter().sorted().next().unwrap().clone();
            for old_state_name in equivalence_class {
                renaming_operations.insert(old_state_name, new_name.clone());
            }
        }
        // Rename the state names of every transition.
        // Then remove duplicates.
        self.transitions = Vec::from_iter(self.transitions.iter().map(|transition| {
            Transition {
                state: renaming_operations.get(transition.state.as_str()).unwrap_or(&transition.state).clone(),
                input: transition.input,
                next_state: renaming_operations.get(transition.next_state.as_str()).unwrap_or(&transition.next_state).clone(),
            }
        }).sorted().dedup());
        renaming_operations
    }

    /// Two states are considered indistinguishable if they transition to states of the same equivalence class __for every input__.
    /// Put simply: Given any input symbol, it does not matter whether you are in state_1 or state_2, you will transition to the same
    /// equivalence class.
    fn are_states_indistinguishable(&self, state_1: &str, state_2: &str, all_input_symbols: &HashSet<char>, equivalence_classes: &Vec<HashSet<String>>) -> bool {
        if state_1 == state_2 {
            return true;
        }
        for input in all_input_symbols {
            // Determine for state 1 and 2 to which equivalence class the DFA would transition for a given input.
            // When the DFA transitions to a different equivalence classes depending on whether the DFA is in state 1 or 2,
            // state 1 and 2 are distinguishable.
            // The next equivalence class is determined by first determining the next state (via transition) and then looking up
            // to which equivalence class this next state belongs.
            let next_equivalence_class_for_state_1 = self.get_transition(&state_1, input)
                .and_then(|transition| equivalence_classes.iter().find(|equivalence_class| equivalence_class.contains(&transition.next_state[..])));
            let next_equivalence_class_for_state_2 = self.get_transition(&state_2, input)
                .and_then(|transition| equivalence_classes.iter().find(|equivalence_class| equivalence_class.contains(&transition.next_state[..])));
            if next_equivalence_class_for_state_1 != next_equivalence_class_for_state_2 {
                return false;
            }
        }
        true
    }

    /// Removes all states that cannot be reached by removing all transitions that have this state
    /// either as start or end point. Uses the breath first algorithm to traverse the whole DFA and fit
    /// all accessible states. All other states are inaccessible.
    fn remove_inaccessible_states(&mut self) {
        // Keep track of all states we visited. Those are accessible and can stay.
        let mut visited_states: HashSet<&str> = HashSet::new();
        // Keep track of the neighbors of our visited states so that we can visit them later.
        // We'll use the "first come - first serve" approach which is typical for the breath first algorithm.
        let mut states_to_visit: VecDeque<&str> = VecDeque::new();
        // We'll start visiting the start state, of course.
        states_to_visit.push_back(&self.start_state);
        // Traverse the graph until there are no states to visit any more.
        while states_to_visit.len() > 0 {
            // First come - first serve. Just as the breath first algorithm is described.
            let currently_visited_state = states_to_visit.pop_front().unwrap();
            visited_states.insert(&currently_visited_state[..]);
            // Find all neighbors of currently visited state (via transitions).
            let transitions_for_currently_visited_state = self.transitions.iter().filter(|transition| transition.state[..] == *currently_visited_state);
            // Loop over each neighbor (via transition) but only add it to our states_to_visit if it has not been visited before.
            // Otherwise, we'll visit states over and over again and be in an infinite loop.
            for transition in transitions_for_currently_visited_state {
                if visited_states.contains(&transition.next_state[..]) {
                    continue;
                }
                states_to_visit.push_back(&transition.next_state)
            }
        }
        // Only keep transitions that have an accessible state and an accessible next_state. The other transitions cannot be accessed and thus should be removed.
        self.transitions = Vec::from_iter(self.transitions.iter().filter(|transition|
            visited_states.contains(&*transition.state) && visited_states.contains(&*transition.next_state)).cloned());
    }
}


#[cfg(test)]
mod dfa_tests {
    use std::collections::{HashMap, HashSet};
    use std::iter::FromIterator;

    use crate::{Dfa, Transition};

    /// Creates DFA that accepts input if all '1' characters are placed at the end and there is at least one '1' character.
    fn create_example_dfa() -> Dfa {
        Dfa {
            name: String::from("Accept if all '1' characters are placed at the end and there is at least one '1' character."),
            start_state: "q0".to_string(),
            accept_states: HashSet::from_iter(vec!["q1".to_string()]),
            transitions: vec![
                Transition {
                    state: "q0".to_string(),
                    input: '0',
                    next_state: "q0".to_string(),
                },
                Transition {
                    state: "q0".to_string(),
                    input: '1',
                    next_state: "q1".to_string(),
                },
                Transition {
                    state: "q1".to_string(),
                    input: '1',
                    next_state: "q1".to_string(),
                },
            ],
        }
    }

    fn create_example_dfa_that_can_be_minimized() -> Dfa {
        Dfa {
            name: String::from(""),
            start_state: "q1".to_string(),
            accept_states: HashSet::from_iter(vec!["q8".to_string()]),
            transitions: vec![
                Transition {
                    state: "q1".to_string(),
                    input: 'a',
                    next_state: "q2".to_string(),
                },
                Transition {
                    state: "q1".to_string(),
                    input: 'b',
                    next_state: "q3".to_string(),
                },
                Transition {
                    state: "q2".to_string(),
                    input: 'a',
                    next_state: "q6".to_string(),
                },
                Transition {
                    state: "q2".to_string(),
                    input: 'b',
                    next_state: "q4".to_string(),
                },
                Transition {
                    state: "q3".to_string(),
                    input: 'a',
                    next_state: "q5".to_string(),
                },
                Transition {
                    state: "q3".to_string(),
                    input: 'b',
                    next_state: "q6".to_string(),
                },
                Transition {
                    state: "q4".to_string(),
                    input: 'a',
                    next_state: "q2".to_string(),
                },
                Transition {
                    state: "q4".to_string(),
                    input: 'b',
                    next_state: "q6".to_string(),
                },
                Transition {
                    state: "q5".to_string(),
                    input: 'a',
                    next_state: "q6".to_string(),
                },
                Transition {
                    state: "q5".to_string(),
                    input: 'b',
                    next_state: "q3".to_string(),
                },
                Transition {
                    state: "q6".to_string(),
                    input: 'a',
                    next_state: "q8".to_string(),
                },
                Transition {
                    state: "q6".to_string(),
                    input: 'b',
                    next_state: "q7".to_string(),
                },
                Transition {
                    state: "q7".to_string(),
                    input: 'a',
                    next_state: "q8".to_string(),
                },
                Transition {
                    state: "q7".to_string(),
                    input: 'b',
                    next_state: "q7".to_string(),
                },
                Transition {
                    state: "q8".to_string(),
                    input: 'a',
                    next_state: "q8".to_string(),
                },
                Transition {
                    state: "q8".to_string(),
                    input: 'b',
                    next_state: "q8".to_string(),
                },
                Transition {
                    state: "inaccessible state".to_string(),
                    input: 'a',
                    next_state: "q8".to_string(),
                }
            ],
        }
    }

    #[test]
    fn test_check() {
        let dfa = create_example_dfa();
        assert!(dfa.check("000111").0, "Should accept if there are at least one '1' characters and they are all at the end");
        assert_eq!(dfa.check("000111").1, vec!["q0", "q0", "q0", "q0", "q1", "q1", "q1"], "Should have traversed those states.");
        assert!(!dfa.check("00010").0, "Should not accept if input does not end with '1'.");
        assert!(!dfa.check("0101").0, "Should not accept if there are '1' characters which are not placed at the end.");
    }

    #[test]
    fn test_get_all_input_symbols() {
        let dfa = create_example_dfa();
        assert_eq!(dfa.get_all_input_symbols(), HashSet::from_iter(vec!['0', '1']));
    }

    #[test]
    fn test_minimize() {
        let mut dfa = create_example_dfa_that_can_be_minimized();
        // Its too verbose to test for the exact new transitions and state names of the minimized DFA.
        // Instead we test for the number of states (which should be minimal) and we test that the DFA
        // still works like before.
        // Before minimizing:
        assert_eq!(dfa.get_all_states().len(), 9);
        assert!(dfa.check("ababba").0, "should accept input");
        // After minimizing:
        dfa.minimize();
        assert_eq!(dfa.get_all_states().len(), 5);
        assert!(dfa.check("ababba").0, "should accept input");
    }
}
