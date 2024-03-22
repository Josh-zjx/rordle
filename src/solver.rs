use super::game::*;
use std::collections::{hash_map, BTreeSet, HashMap};
use std::io::prelude::*;
use std::sync::Arc;

const CACHE: bool = true;
#[derive(Debug)]
pub struct Solver {
    patterns: Vec<Pattern>,
    valid_table: Vec<bool>,
    second_cache: Vec<String>,
    candidates: Vec<String>,
    pub current_candidate: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pattern {
    pub chars: Arc<String>,
    pub state: [GuessState; 5],
}

impl Solver {
    pub fn bind(game: &Game) -> Solver {
        let table_size = game.candidates.len();
        let mut cache_strings = String::new();
        {
            let mut cache_file = std::fs::File::open("./data/cache").unwrap();
            cache_file.read_to_string(&mut cache_strings).unwrap();
        }
        let cache_lines: Vec<String> = serde_json::from_str(&cache_strings).unwrap();

        Solver {
            patterns: Vec::new(),
            valid_table: vec![true; table_size],
            second_cache: cache_lines,
            candidates: game.candidates.clone(),
            current_candidate: String::new(),
        }
    }
    pub fn new_guess(&self, round: u8) -> (Guess, f64) {
        if round == 0 {
            return (
                Guess {
                    state: "tares".to_string(),
                },
                0.0,
            );
        }
        let mut score = -1.0;
        let mut index = 0;
        let mut rank: Vec<(f64, &str)> = vec![];
        for i in 0..self.candidates.len() {
            let temp = self.valid_word(i);
            if temp {
                let new_score = self.calculate_score(i);
                rank.push((-new_score, self.candidates[i].as_str()));
                if new_score > score {
                    score = new_score;
                    index = i;
                }
            }
        }
        rank.sort_by(|a, b| a.partial_cmp(b).unwrap());
        for (x, y) in rank.into_iter().take(5) {
            println!("{}: {}", y, -x);
        }
        (
            Guess {
                state: self.candidates[index].clone(),
            },
            score,
        )
    }
    pub fn try_guess(&mut self, guess: Guess, game: &mut Game) -> Option<Arc<Match>> {
        if !game.check_valid_guess(&guess) {
            return None;
        }
        let one_match = game.grade_guess(&guess);
        let shared_match = Arc::new(one_match);
        game.progress_game(shared_match.clone());
        self.add_pattern(guess.state, shared_match.clone());
        Some(shared_match)
    }
    fn valid_word(&self, table_index: usize) -> bool {
        let word = &self.candidates[table_index];
        for i in self.patterns.iter() {
            if !self.try_match(word, i) {
                return false;
            }
        }

        true
    }
    pub fn reset(&mut self) {
        self.valid_table = vec![true; self.candidates.len()];
        self.patterns = Vec::new();
        self.current_candidate = String::new();
    }

    fn calculate_score(&self, table_index: usize) -> f64 {
        let word = Arc::new(self.candidates[table_index].clone());

        let mut score = 0.0;
        let patterns = generate_pattern(word.clone());
        let mut total = 0;
        let mut pattern_matched = vec![0; patterns.len()];
        for j in 0..self.candidates.len() {
            if j == table_index {
                continue;
            }
            if self.valid_word(j) {
                total += 1;
                for i in 0..patterns.len() {
                    if self.try_match(&self.candidates[j], &patterns[i]) {
                        pattern_matched[i] += 1;
                    }
                }
            }
        }
        for i in 0..patterns.len() {
            if pattern_matched[i] != 0 {
                let p = pattern_matched[i] as f64 / total as f64;
                score += 0.0 - p * p.log2();
            }
        }

        score
    }

    /// check whether the guess word is compatible with a match pattern
    ///
    ///
    fn try_match(&self, word: &String, pattern: &Pattern) -> bool {
        let mut nonexist = BTreeSet::new();
        let mut exist = BTreeSet::new();
        let pattern_bytes = pattern.chars.as_bytes();
        let word_bytes = word.as_bytes();
        let mut word_set: BTreeSet<u8> = BTreeSet::new();
        word_set.extend(word_bytes.iter());

        for i in 0..5 {
            if pattern.state[i] == GuessState::Correct {
                if word_bytes[i] != pattern_bytes[i] {
                    //println!("Correct mismatch");
                    return false;
                }
            } else if pattern.state[i] == GuessState::Wrong {
                if word_bytes[i] == pattern_bytes[i] {
                    return false;
                }
                nonexist.insert(pattern_bytes[i]);
            } else if pattern.state[i] == GuessState::Misplace {
                if word_bytes[i] == pattern_bytes[i] {
                    return false;
                }
                exist.insert(pattern_bytes[i]);
            }
        }
        for i in nonexist.iter() {
            if word_set.contains(i) {
                return false;
            }
        }
        for i in exist.iter() {
            if !word_set.contains(i) {
                return false;
            }
        }
        true
    }
    pub fn add_pattern(&mut self, word: String, one_match: Arc<Match>) {
        let boxed: Arc<String> = Arc::new(word);
        self.patterns.push(Pattern {
            chars: boxed.clone(),
            state: [
                one_match.states[0],
                one_match.states[1],
                one_match.states[2],
                one_match.states[3],
                one_match.states[4],
            ],
        });
    }
}
fn generate_pattern(word: Arc<String>) -> Vec<Pattern> {
    let mut result: Vec<Pattern> = Vec::new();
    let boxed = word;
    result.push(Pattern {
        chars: boxed.clone(),
        state: [
            GuessState::Wrong,
            GuessState::Wrong,
            GuessState::Wrong,
            GuessState::Wrong,
            GuessState::Wrong,
        ],
    });
    result = result
        .into_iter()
        .flat_map(|mut x| {
            let mut new_vec = Vec::new();
            new_vec.push(x.clone());
            x.state[0] = GuessState::Misplace;
            new_vec.push(x.clone());
            x.state[0] = GuessState::Correct;
            new_vec.push(x);
            new_vec
        })
        .flat_map(|mut x| {
            let mut new_vec = Vec::new();
            new_vec.push(x.clone());
            x.state[1] = GuessState::Misplace;
            new_vec.push(x.clone());
            x.state[1] = GuessState::Correct;
            new_vec.push(x);
            new_vec
        })
        .flat_map(|mut x| {
            let mut new_vec = Vec::new();
            new_vec.push(x.clone());
            x.state[2] = GuessState::Misplace;
            new_vec.push(x.clone());
            x.state[2] = GuessState::Correct;
            new_vec.push(x);
            new_vec
        })
        .flat_map(|mut x| {
            let mut new_vec = Vec::new();
            new_vec.push(x.clone());
            x.state[3] = GuessState::Misplace;
            new_vec.push(x.clone());
            x.state[3] = GuessState::Correct;
            new_vec.push(x);
            new_vec
        })
        .flat_map(|mut x| {
            let mut new_vec = Vec::new();
            new_vec.push(x.clone());
            x.state[4] = GuessState::Misplace;
            new_vec.push(x.clone());
            x.state[4] = GuessState::Correct;
            new_vec.push(x);
            new_vec
        })
        .filter(|x| {
            let mut guessstate_dict = HashMap::<_, GuessState>::new();
            for i in 0..5 {
                let temp = x.chars.chars().nth(i).unwrap();
                if guessstate_dict.contains_key(&temp) {
                    if *guessstate_dict.get(&temp).unwrap() != x.state[i] {
                        if guessstate_dict.get(&temp).unwrap() == &GuessState::Wrong
                            || x.state[i] == GuessState::Wrong
                        {
                            return false;
                        }
                    }
                } else {
                    guessstate_dict.insert(temp.clone(), x.state[i]);
                }
            }
            true
        })
        .collect();
    return result;
}
