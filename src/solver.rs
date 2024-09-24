use super::game::*;
use rayon::prelude::*;
use std::io::prelude::*;
use std::sync::Arc;

const PATTERN_SIZE: usize = 243;
#[derive(Debug)]
pub struct Solver {
    patterns: Vec<Pattern>,
    valid_table: Vec<bool>,
    candidates: Vec<String>,
    pub current_candidate: String,
    survive: usize,
}

fn grade_pair(word: &String, candidate: &String) -> usize {
    let pattern_bytes = candidate.as_bytes();
    let word_bytes = word.as_bytes();
    let mut wordvec = 0u32;
    let mut pattern_index = 0;

    for byte in word_bytes.iter() {
        wordvec |= char_to_bitvec(*byte);
    }
    for i in 0..5 {
        pattern_index *= 3;
        if word_bytes[i] == pattern_bytes[i] {
            pattern_index += 2;
        } else if wordvec & char_to_bitvec(pattern_bytes[i]) != 0 {
            pattern_index += 1;
        }
    }
    pattern_index
}

fn char_to_bitvec(c: u8) -> u32 {
    1u32 << (c - 97)
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

        Solver {
            patterns: Vec::new(),
            valid_table: vec![true; table_size],
            candidates: game.candidates.clone(),
            current_candidate: String::new(),
            survive: game.candidates.len(),
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
        if self.survive == 1 {
            for i in 0..self.candidates.len() {
                if self.valid_word(i) {
                    return (
                        Guess {
                            state: self.candidates[i].clone(),
                        },
                        0.0,
                    );
                }
            }
        }

        let mut score = -1.0;
        let mut index = self.candidates.len();

        #[cfg(debug_assertions)]
        let mut rank: Vec<(f64, &str)> = vec![];

        for i in 0..self.candidates.len() {
            let new_score = self.calculate_score(i);

            #[cfg(debug_assertions)]
            rank.push((-new_score, self.candidates[i].as_str()));

            if new_score > score {
                score = new_score;
                index = i;
            }
        }

        #[cfg(debug_assertions)]
        {
            rank.sort_by(|a, b| a.partial_cmp(b).unwrap());

            for (x, y) in rank.into_iter().take(100) {
                println!("{}: {}", y, -x);
            }
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
        #[cfg(debug_assertions)]
        println!("{:?}", shared_match);
        self.filter_valid_word();
        Some(shared_match)
    }
    fn valid_word(&self, table_index: usize) -> bool {
        self.valid_table[table_index]
    }
    pub fn reset(&mut self) {
        self.valid_table = vec![true; self.candidates.len()];
        self.patterns = Vec::new();
        self.current_candidate = String::new();
    }

    fn filter_valid_word(&mut self) {
        let mut survive = self.candidates.len();
        for table_index in 0..self.candidates.len() {
            if !self.valid_word(table_index) {
                survive -= 1;
                continue;
            }
            let word = &self.candidates[table_index];
            for i in self.patterns.iter() {
                if !self.try_match(word, i) {
                    self.valid_table[table_index] = false;
                    survive -= 1;
                    break;
                }
            }
        }
        self.survive = survive;
    }
    fn calculate_score(&self, table_index: usize) -> f64 {
        let word = Arc::new(self.candidates[table_index].clone());

        let mut score: f64 = 0.0;
        let mut total = 0;
        let mut pattern_matched = vec![0; PATTERN_SIZE];
        for j in 0..self.candidates.len() {
            if self.valid_word(j) {
                total += 1;
                let pattern_index = grade_pair(&self.candidates[j], &word);
                pattern_matched[pattern_index] += 1;
            }
        }
        for i in pattern_matched.iter() {
            if *i != 0 {
                let p = *i as f64 / total as f64;
                score -= p * p.log2();
            }
        }

        score
    }

    /// check whether the guess word is compatible with a match pattern
    ///
    /// # [A B C D E]
    /// # [C W M M M]
    ///
    /// # [X X X X X]
    ///
    ///
    ///
    fn try_match(&self, word: &String, pattern: &Pattern) -> bool {
        let pattern_bytes = pattern.chars.as_bytes();
        let word_bytes = word.as_bytes();
        let mut wordvec = 0u32;

        for byte in word_bytes.iter() {
            wordvec |= char_to_bitvec(*byte);
        }
        for i in 0..5 {
            if word_bytes[i] == pattern_bytes[i] {
                if pattern.state[i] != GuessState::Correct {
                    return false;
                }
            } else if wordvec & char_to_bitvec(pattern_bytes[i]) != 0 {
                if pattern.state[i] != GuessState::Misplace {
                    return false;
                }
            } else if pattern.state[i] != GuessState::Wrong {
                return false;
            }
        }
        true
    }

    pub fn add_pattern(&mut self, word: String, one_match: Arc<Match>) {
        let boxed: Arc<String> = Arc::new(word);
        self.patterns = vec![Pattern {
            chars: boxed.clone(),
            state: [
                one_match.states[0],
                one_match.states[1],
                one_match.states[2],
                one_match.states[3],
                one_match.states[4],
            ],
        }];
    }
}
