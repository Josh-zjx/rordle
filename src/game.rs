use std::collections::BTreeSet;
use std::io::prelude::*;
use std::sync::Arc;

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
pub enum GuessState {
    Wrong,
    Misplace,
    Correct,
}

#[derive(Debug)]
pub struct Game {
    answer: String,
    answer_index: usize,
    pub answers: Vec<String>,
    pub candidates: Vec<String>,
    round: usize,
    pub state: GameState,
}
impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
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
        index %= answers.len();

        let mut candidate_strings = String::new();
        {
            let mut candidate_file = std::fs::File::open("./data/candidate").unwrap();
            candidate_file
                .read_to_string(&mut candidate_strings)
                .unwrap();
        }

        let mut candidate_vec: Vec<String> = serde_json::from_str(&candidate_strings).unwrap();
        let answer = answers[index].clone();

        #[cfg(debug_assertions)]
        println!("The answer is {}", answer);
        candidate_vec.append(&mut (answers.clone()));
        Game {
            answer,
            answer_index: index,
            answers,
            candidates: candidate_vec,
            round: 0,
            state: GameState::On,
        }
    }
    pub fn set_game_with_answer_index(&mut self, index: usize) {
        assert!(index < self.answers.len());
        self.answer_index = index;
        self.answer = self.answers[self.answer_index].clone();
        self.round = 0;
        self.state = GameState::On;
    }
    pub fn grade_guess(&self, guess: &Guess) -> Match {
        let mut one_match = Match::new();
        // Correct pass
        let word = &guess.state;
        let mut char_set: BTreeSet<u8> = BTreeSet::new();
        let answer_bytes = self.answer.as_bytes();
        let word_bytes = word.as_bytes();
        char_set.extend(answer_bytes.iter());

        for i in 0..5 {
            if word_bytes[i] == answer_bytes[i] {
                one_match.states[i] = GuessState::Correct;
            } else if char_set.contains(&word_bytes[i]) {
                one_match.states[i] = GuessState::Misplace;
            } else {
                one_match.states[i] = GuessState::Wrong;
            }
        }
        one_match
    }
    pub fn check_valid_guess(&self, guess: &Guess) -> bool {
        let word = &guess.state;
        self.candidates.iter().any(|i| *i == *word)
    }
    pub fn progress_game(&mut self, one_match: Arc<Match>) {
        if one_match.is_correct() {
            self.state = GameState::Correct;
        } else {
            self.state = GameState::On;
            self.inc_round();
        }
    }
    pub fn round(&self) -> usize {
        self.round
    }
    pub fn inc_round(&mut self) {
        self.round += 1;
    }
    pub fn answer(&self) -> String {
        self.answer.clone()
    }
    pub fn reset(&mut self) {
        let mut index: usize = rand::random();
        index %= self.answers.len();

        self.answer = self.answers[index].clone();
    }
}

#[derive(Debug, Clone)]
pub struct Match {
    pub states: [GuessState; 5],
}
impl Default for Match {
    fn default() -> Self {
        Self::new()
    }
}
impl Match {
    pub fn is_correct(&self) -> bool {
        self.states[0] == GuessState::Correct
            && self.states[1] == GuessState::Correct
            && self.states[2] == GuessState::Correct
            && self.states[3] == GuessState::Correct
            && self.states[4] == GuessState::Correct
    }
    pub fn new() -> Match {
        Match {
            states: [
                GuessState::Wrong,
                GuessState::Wrong,
                GuessState::Wrong,
                GuessState::Wrong,
                GuessState::Wrong,
            ],
        }
    }
}
#[derive(PartialEq, Debug)]
pub enum GameState {
    On,
    ReadyForCheck,
    Correct,
    Over,
}
#[derive(Clone, Debug)]
pub struct Guess {
    pub state: String,
}
