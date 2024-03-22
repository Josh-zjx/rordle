use super::game::*;
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

#[derive(Debug, Clone)]
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

        assert_eq!(cache_lines.len(), 243);
        Solver {
            patterns: Vec::new(),
            valid_table: vec![true; table_size],
            second_cache: cache_lines,
            candidates: game.candidates.clone(),
            current_candidate: String::new(),
        }
    }
    pub fn new_guess(&self, round: u8) -> Guess {
        if round == 0 {
            return Guess {
                state: "tares".to_string(),
            };
        }
        if CACHE && round == 1 {
            let mut index = 0;
            for i in 0..5 {
                index *= 3;
                index += match self.patterns[0].state[i] {
                    GuessState::Wrong => 0,
                    GuessState::Misplace => 1,
                    GuessState::Correct => 2,
                }
            }
            return Guess {
                state: self.second_cache[index].to_string(),
            };
        }
        let mut score = -1.0;
        let mut index = 0;
        for i in 0..self.candidates.len() {
            let temp = self.valid_word(i);
            if temp {
                let new_score = self.calculate_score(i);
                if new_score > score {
                    score = new_score;
                    index = i;
                }
            }
        }
        Guess {
            state: self.candidates[index].clone(),
        }
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
        let mut pattern_matched = vec![0; 243];
        for j in 0..self.candidates.len() {
            if self.valid_word(j) {
                total += 1;
                for i in 0..243 {
                    if self.try_match(&self.candidates[j], &patterns[i]) {
                        pattern_matched[i] += 1;
                    }
                }
            }
        }
        for i in 0..243 {
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
        let mut nonexist: [u8; 5] = [0; 5];
        let pattern_bytes = pattern.chars.as_bytes();
        let word_bytes = word.as_bytes();
        for i in 0..5 {
            if pattern.state[i] == GuessState::Correct {
                if word_bytes[i] != pattern_bytes[i] {
                    return false;
                }
            } else if pattern.state[i] == GuessState::Wrong {
                nonexist[i] = pattern_bytes[i];
            }
        }
        for i in 0..5 {
            if pattern.state[i] == GuessState::Correct {
                continue;
            }
            for j in nonexist.iter() {
                if word_bytes[i] == *j {
                    return false;
                }
            }
            if pattern.state[i] == GuessState::Misplace && word_bytes[i] == pattern_bytes[i]
                || !word.contains(pattern_bytes[i] as char)
            {
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
        .collect();
    assert_eq!(result.len(), 243);
    return result;
}
