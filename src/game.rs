use bevy::prelude::Component;
use std::io::prelude::*;
use std::rc::Rc;

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum GuessState {
    Wrong,
    Misplace,
    Correct,
}

#[derive(Debug, Component)]
pub struct Game {
    answer: String,
    pub answers: Vec<String>,
    pub candidates: Vec<String>,
    round: usize,
    pub state: GameState,
}
impl Game {
    pub fn new() -> Game {
        let mut answer_strings = String::new();
        {
            let mut answer_file = std::fs::File::open("./data/answer").unwrap();
            answer_file.read_to_string(&mut answer_strings).unwrap();
        }
        let answers: Vec<String> = serde_json::from_str(&answer_strings).unwrap();

        let mut index: usize = rand::random();
        index = index % answers.len();

        let mut candidate_strings = String::new();
        {
            let mut candidate_file = std::fs::File::open("./data/candidate").unwrap();
            candidate_file
                .read_to_string(&mut candidate_strings)
                .unwrap();
        }

        let mut candidate_vec: Vec<String> = serde_json::from_str(&candidate_strings).unwrap();
        let answer = answers[index].clone();
        candidate_vec.append(&mut (answers.clone()));
        let new_game = Game {
            answer,
            answers,
            candidates: candidate_vec,
            round: 0,
            state: GameState::On,
        };
        return new_game;
    }
    pub fn grade_guess(&self, guess: &Guess) -> Match {
        let mut one_match = Match::new();
        // Correct pass
        let word = &guess.state;
        for i in 0..5 {
            if (*word).as_bytes()[i] == self.answer.as_bytes()[i] {
                one_match.states[i] = GuessState::Correct;
            }
        }
        // Wrong pass
        for i in 0..5 {
            if word.as_bytes()[i] != self.answer.as_bytes()[i] {
                for j in 0..5 {
                    if one_match.states[j] == GuessState::Correct {
                        continue;
                    }
                    if word.as_bytes()[i] == self.answer.as_bytes()[j] {
                        one_match.states[i] = GuessState::Misplace;
                        break;
                    }
                }
            }
        }
        return one_match;
    }
    pub fn check_valid_guess(&self, guess: &Guess) -> bool {
        let word = &guess.state;
        if self.candidates.iter().any(|i| *i == *word) {
            return true;
        } else {
            return false;
        }
    }
    pub fn progress_game(&mut self, one_match: Rc<Match>) {
        if one_match.is_correct() {
            self.state = GameState::Correct;
            return;
        } else {
            self.state = GameState::On;
        }
        // Update round statistics
        self.inc_round();
    }
    pub fn round(&self) -> usize {
        return self.round;
    }
    pub fn inc_round(&mut self) {
        self.round += 1;
    }
    pub fn answer(&self) -> String {
        return self.answer.clone();
    }
    pub fn reset(&mut self) {
        let mut index: usize = rand::random();
        index = index % self.answers.len();

        self.answer = self.answers[index].clone();
    }
}

#[derive(Debug, Clone)]
pub struct Match {
    pub states: [GuessState; 5],
}
impl Match {
    pub fn is_correct(&self) -> bool {
        if self.states[0] == GuessState::Correct
            && self.states[1] == GuessState::Correct
            && self.states[2] == GuessState::Correct
            && self.states[3] == GuessState::Correct
            && self.states[4] == GuessState::Correct
        {
            true
        } else {
            false
        }
    }
    pub fn new() -> Match {
        return Match {
            states: [
                GuessState::Wrong,
                GuessState::Wrong,
                GuessState::Wrong,
                GuessState::Wrong,
                GuessState::Wrong,
            ],
        };
    }
}
#[derive(PartialEq, Debug)]
pub enum GameState {
    On,
    ReadyForCheck,
    Correct,
    Over,
}
#[derive(Component, Clone, Debug)]
pub struct Guess {
    pub state: String,
}
