use std::collections::HashSet;

use super::game::*;

#[derive(Debug)]
pub struct Solver<'a> {
    game: &'a mut Game,
    nonexist: [HashSet<char>; 5],
    exist: HashSet<char>,
    intent: [char; 5],
    valid_table: Vec<bool>,
}

impl<'a> Solver<'a> {
    pub fn bind(game: &mut Game) -> Solver {
        let table_size = game.candidates.len();
        return Solver {
            game,
            nonexist: [
                HashSet::new(),
                HashSet::new(),
                HashSet::new(),
                HashSet::new(),
                HashSet::new(),
            ],
            exist: HashSet::new(),
            intent: ['*'; 5],
            valid_table: vec![true; table_size],
        };
    }
    pub fn new_guess(&mut self) -> Guess {
        if self.game.round() == 0 {
            return Guess {
                state: "tares".to_string(),
            };
        }
        let mut score = -1.0;
        let mut index: String = String::new();
        for i in 0..self.game.candidates.len() {
            let temp = self.valid_word(i);
            if temp {
                let new_score = self.calculate_score(i);
                if new_score > score {
                    score = new_score;
                    index = self.game.candidates[i].clone();
                }
            }
        }
        return Guess { state: index };
    }
    pub fn try_guess(&mut self, guess: &Guess) -> Option<Match> {
        if !self.game.check_valid_guess(guess) {
            return None;
        }
        let one_match = self.game.grade_guess(guess);
        self.game.progress_game(one_match.clone());
        for i in 0..5 {
            if one_match.states[i] == GuessState::Correct {
                self.intent[i] = guess.state.as_bytes()[i] as char;
                self.exist.insert(guess.state.as_bytes()[i] as char);
            } else if one_match.states[i] == GuessState::Misplace {
                self.nonexist[i].insert(guess.state.as_bytes()[i] as char);
                self.exist.insert(guess.state.as_bytes()[i] as char);
            }
        }
        for i in 0..5 {
            if one_match.states[i] == GuessState::Wrong {
                if !self.exist.contains(&(guess.state.as_bytes()[i] as char)) {
                    for j in 0..5 {
                        self.nonexist[j].insert(guess.state.as_bytes()[i] as char);
                    }
                } else {
                    self.nonexist[i].insert(guess.state.as_bytes()[i] as char);
                }
            }
        }

        return Some(one_match);
    }
    fn valid_word(&mut self, table_index: usize) -> bool {
        if !self.valid_table[table_index] {
            return false;
        }
        let word = &self.game.candidates[table_index];
        let slice = word.as_bytes();

        for index in 0..5 {
            if self.intent[index] != '*' && self.intent[index] != slice[index] as char {
                self.valid_table[table_index] = false;
                return false;
            }
        }
        for index in 0..5 {
            for i in self.nonexist[index].iter() {
                if slice[index] as char == *i {
                    self.valid_table[table_index] = false;
                    return false;
                }
            }
        }
        for i in self.exist.iter() {
            if !word.contains(*i) {
                self.valid_table[table_index] = false;
                return false;
            }
        }
        return true;
    }
    fn calculate_score(&mut self, table_index: usize) -> f64 {
        let word = self.game.candidates[table_index].clone();
        let mut total = 0;
        let mut sum = [0; 5];
        for i in 0..self.game.candidates.len() {
            let can = &self.game.candidates[i].clone();
            if self.valid_word(i) {
                total += 1;
                for j in 0..5 {
                    if can.as_bytes()[j] == word.as_bytes()[j] {
                        sum[j] += 1;
                    }
                }
            }
        }
        let mut p = 0.0;
        for i in 0..5 {
            if sum[i] == total {
                p += 100.0;
            } else {
                p += f64::log2((total as f64) / ((total - sum[i]) as f64));
            }
        }

        return p;
    }
}
