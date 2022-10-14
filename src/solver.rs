use std::collections::HashSet;

use super::game::*;

#[derive(Debug)]
pub struct Solver<'a> {
    game: &'a mut Game,
    history_guess: Vec<Guess>,
    history_match: Vec<Match>,
    nonexist: [HashSet<char>; 5],
    exist: HashSet<char>,
    count: usize,
    intent: [char; 5],
}

impl<'a> Solver<'a> {
    pub fn bind(game: &mut Game) -> Solver {
        return Solver {
            game,
            history_match: Vec::new(),
            history_guess: Vec::new(),
            nonexist: [
                HashSet::new(),
                HashSet::new(),
                HashSet::new(),
                HashSet::new(),
                HashSet::new(),
            ],
            exist: HashSet::new(),
            count: 0,
            intent: ['*'; 5],
        };
    }
    pub fn new_guess(&mut self) -> Guess {
        for i in 0..self.game.candidates.len() {
            let word = &self.game.candidates[i];
            if self.valid_word(word) {
                self.count = i + 1;
                return Guess {
                    state: word.clone(),
                };
            }
        }
        return Guess {
            state: "tares".to_string(),
        };
    }
    pub fn try_guess(&mut self, guess: &Guess) -> Option<Match> {
        if !self.game.check_valid_guess(guess) {
            return None;
        }
        let one_match = self.game.grade_guess(guess);
        self.history_guess.push(guess.clone());
        self.history_match.push(one_match.clone());
        self.game.progress_game(one_match.clone());
        for i in 0..5 {
            if one_match.states[i] == GuessState::Correct {
                self.intent[i] = guess.state.as_bytes()[i] as char;
                self.exist.insert(guess.state.as_bytes()[i] as char);
            } else if one_match.states[i] == GuessState::Misplace {
                self.exist.insert(guess.state.as_bytes()[i] as char);
                self.nonexist[i].insert(guess.state.as_bytes()[i] as char);
            } else {
                self.nonexist[i].insert(guess.state.as_bytes()[i] as char);
            }
        }

        return Some(one_match);
    }
    fn valid_word(&self, word: &String) -> bool {
        let slice = word.as_bytes();
        for index in 0..5 {
            if self.intent[index] != '*' && self.intent[index] != slice[index] as char {
                return false;
            }
        }
        for index in 0..5 {
            for i in self.nonexist[index].iter() {
                if slice[index] as char == *i {
                    return false;
                }
            }
        }
        for i in self.exist.iter() {
            if !word.contains(*i) {
                return false;
            }
        }
        return true;
    }
}
