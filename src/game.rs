use std::collections::BTreeSet;
use std::error::Error;
use std::io::prelude::*;
use std::sync::{Arc, OnceLock};

#[derive(Debug)]
pub(crate) struct WordList {
    pub(crate) answers: Vec<String>,
    pub(crate) candidates: Vec<String>,
    pub(crate) candidate_bytes: Box<[[u8; 5]]>,
    pub(crate) candidate_bitvecs: Box<[u32]>,
}

impl WordList {
    fn load() -> Result<WordList, Box<dyn Error>> {
        let mut answer_strings = String::new();
        {
            let mut answer_file = std::fs::File::open("./data/answer")?;
            answer_file.read_to_string(&mut answer_strings)?;
        }
        let answers: Vec<String> = serde_json::from_str(&answer_strings)?;

        let mut candidate_strings = String::new();
        {
            let mut candidate_file = std::fs::File::open("./data/candidate")?;
            candidate_file.read_to_string(&mut candidate_strings)?;
        }

        let mut candidates: Vec<String> = serde_json::from_str(&candidate_strings)?;
        candidates.extend(answers.iter().cloned());

        let candidate_bytes: Box<[[u8; 5]]> = candidates
            .iter()
            .map(|w| {
                let mut buf = [0u8; 5];
                buf.copy_from_slice(w.as_bytes());
                buf
            })
            .collect();

        let candidate_bitvecs: Box<[u32]> = candidate_bytes
            .iter()
            .map(|w| {
                let mut v = 0u32;
                for b in w.iter() {
                    v |= 1u32 << (*b - 97);
                }
                v
            })
            .collect();

        Ok(WordList {
            answers,
            candidates,
            candidate_bytes,
            candidate_bitvecs,
        })
    }
}

fn shared_word_list() -> Arc<WordList> {
    static WORD_LIST: OnceLock<Arc<WordList>> = OnceLock::new();

    Arc::clone(
        WORD_LIST.get_or_init(|| {
            Arc::new(WordList::load().expect("failed to load word list data files"))
        }),
    )
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
pub enum GuessState {
    Wrong,
    Misplace,
    Correct,
}

#[derive(Debug)]
pub struct Game {
    words: Arc<WordList>,
    answer_index: usize,
    custom_answer: Option<String>,
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
        let words = shared_word_list();

        let mut index: usize = rand::random();
        index %= words.answers.len();

        Game {
            words,
            answer_index: index,
            custom_answer: None,
            round: 0,
            state: GameState::On,
        }
    }
    pub fn answers(&self) -> &[String] {
        &self.words.answers
    }
    pub fn candidates(&self) -> &[String] {
        &self.words.candidates
    }
    pub(crate) fn word_list(&self) -> Arc<WordList> {
        Arc::clone(&self.words)
    }
    pub fn set_game_with_answer_index(&mut self, index: usize) {
        assert!(index < self.answers().len());
        self.answer_index = index;
        self.custom_answer = None;
        self.round = 0;
        self.state = GameState::On;
    }
    pub fn set_game_with_answer(&mut self, answer: impl Into<String>) {
        self.answer_index = 0;
        self.custom_answer = Some(answer.into());
        self.round = 0;
        self.state = GameState::On;
    }
    fn current_answer(&self) -> &str {
        self.custom_answer
            .as_deref()
            .unwrap_or(&self.words.answers[self.answer_index])
    }
    pub fn grade_guess(&self, word: &str) -> Match {
        let mut one_match = Match::new();
        // Correct pass
        let mut char_set: BTreeSet<u8> = BTreeSet::new();
        let answer_bytes = self.current_answer().as_bytes();
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
    pub fn check_valid_guess(&self, word: &str) -> bool {
        self.candidates().iter().any(|candidate| candidate == word)
    }
    pub fn progress_game(&mut self, one_match: &Match) {
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
    pub fn answer(&self) -> &str {
        self.current_answer()
    }
    pub fn reset(&mut self) {
        let mut index: usize = rand::random();
        index %= self.answers().len();

        self.answer_index = index;
        self.custom_answer = None;
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
#[cfg(test)]
mod tests {
    use super::*;

    fn correct_match() -> Match {
        Match {
            states: [GuessState::Correct; 5],
        }
    }

    #[test]
    fn new_game_starts_with_loaded_words_and_initial_state() {
        let game = Game::new();

        assert!(!game.answers().is_empty());
        assert!(!game.candidates().is_empty());
        assert_eq!(game.round(), 0);
        assert!(matches!(game.state, GameState::On));
        assert!(game.candidates().contains(&game.answer().to_string()));
    }

    #[test]
    fn word_list_loads_answers_and_candidates_from_disk() {
        let words = WordList::load().expect("failed to load word list test data");

        assert!(!words.answers.is_empty());
        assert!(!words.candidates.is_empty());
        assert!(words
            .answers
            .iter()
            .all(|answer| words.candidates.contains(answer)));
    }

    #[test]
    fn grade_guess_marks_exact_match_as_all_correct() {
        let mut game = Game::new();
        game.set_game_with_answer("cigar".to_string());

        let one_match = game.grade_guess("cigar");

        assert_eq!(one_match.states, [GuessState::Correct; 5]);
    }

    #[test]
    fn grade_guess_marks_partial_and_mixed_matches_with_current_rules() {
        let mut game = Game::new();
        game.set_game_with_answer("cigar".to_string());

        let partial = game.grade_guess("argon");
        let mixed = game.grade_guess("cairn");

        assert_eq!(
            partial.states,
            [
                GuessState::Misplace,
                GuessState::Misplace,
                GuessState::Correct,
                GuessState::Wrong,
                GuessState::Wrong,
            ]
        );
        assert_eq!(
            mixed.states,
            [
                GuessState::Correct,
                GuessState::Misplace,
                GuessState::Misplace,
                GuessState::Misplace,
                GuessState::Wrong,
            ]
        );
    }

    #[test]
    fn grade_guess_marks_absent_letters_as_wrong() {
        let mut game = Game::new();
        game.set_game_with_answer("cigar".to_string());

        let one_match = game.grade_guess("blush");

        assert_eq!(one_match.states, [GuessState::Wrong; 5]);
    }

    #[test]
    fn check_valid_guess_distinguishes_known_and_unknown_words() {
        let game = Game::new();

        assert!(game.check_valid_guess("zonal"));
        assert!(!game.check_valid_guess("xxxxx"));
    }

    #[test]
    fn progress_game_sets_correct_or_advances_round() {
        let mut game = Game::new();

        game.progress_game(&Match::new());
        assert!(matches!(game.state, GameState::On));
        assert_eq!(game.round(), 1);

        game.progress_game(&correct_match());
        assert!(matches!(game.state, GameState::Correct));
        assert_eq!(game.round(), 1);
    }

    #[test]
    fn set_game_with_answer_and_index_update_answer() {
        let mut game = Game::new();
        let zonal_index = game
            .answers()
            .iter()
            .position(|answer| answer == "zonal")
            .expect("zonal should exist in the answer list");

        game.set_game_with_answer("zonal");
        assert_eq!(game.answer(), "zonal");

        game.set_game_with_answer_index(zonal_index);
        assert_eq!(game.answer(), "zonal");
        assert_eq!(game.round(), 0);
        assert!(matches!(game.state, GameState::On));
    }

    #[test]
    fn set_game_with_answer_accepts_custom_answers() {
        let mut game = Game::new();

        game.set_game_with_answer("abcde");

        assert_eq!(game.answer(), "abcde");
    }

    #[test]
    fn match_is_correct_only_when_every_slot_is_correct() {
        assert!(correct_match().is_correct());
        assert!(!Match::new().is_correct());
    }
}
