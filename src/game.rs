use std::error::Error;
use std::fmt;
use std::sync::{Arc, OnceLock};

const ANSWER_DATA: &str = include_str!("../data/answer");
const CANDIDATE_DATA: &str = include_str!("../data/candidate");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameError {
    InvalidWord,
    InvalidAnswerIndex(usize),
    EmptyAnswerList,
}

impl fmt::Display for GameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidWord => {
                f.write_str("a word must contain exactly five lowercase ASCII letters")
            }
            Self::InvalidAnswerIndex(index) => write!(f, "answer index {index} is out of bounds"),
            Self::EmptyAnswerList => f.write_str("the answer list must not be empty"),
        }
    }
}

impl Error for GameError {}

/// A validated five-letter word. Dictionary membership is checked separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Word([u8; 5]);

impl TryFrom<[u8; 5]> for Word {
    type Error = GameError;

    fn try_from(bytes: [u8; 5]) -> Result<Self, Self::Error> {
        if bytes.iter().all(u8::is_ascii_lowercase) {
            Ok(Self(bytes))
        } else {
            Err(GameError::InvalidWord)
        }
    }
}

impl TryFrom<&str> for Word {
    type Error = GameError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        let bytes: [u8; 5] = text
            .as_bytes()
            .try_into()
            .map_err(|_| GameError::InvalidWord)?;
        Self::try_from(bytes)
    }
}

impl Word {
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).expect("Word contains only ASCII letters")
    }

    /// Grade a guess against this answer, consuming exact matches first.
    #[inline]
    pub(crate) fn grade(self, guess: Self) -> Match {
        let mut result = Match::new();
        let mut counts = [0u8; 26];
        for &letter in &self.0 {
            counts[(letter - b'a') as usize] += 1;
        }
        for ((state, &letter), &answer) in result.states.iter_mut().zip(&guess.0).zip(&self.0) {
            if letter == answer {
                *state = GuessState::Correct;
                counts[(letter - b'a') as usize] -= 1;
            }
        }
        for (state, &letter) in result.states.iter_mut().zip(&guess.0) {
            let count = &mut counts[(letter - b'a') as usize];
            if *state != GuessState::Correct && *count > 0 {
                *state = GuessState::Misplace;
                *count -= 1;
            }
        }
        result
    }
}

#[derive(Debug)]
pub(crate) struct WordList {
    pub(crate) answers: Vec<String>,
    pub(crate) candidates: Vec<String>,
    #[cfg(all(feature = "solver", not(target_arch = "wasm32")))]
    candidate_words: OnceLock<Box<[Word]>>,
}

impl WordList {
    fn load() -> Result<WordList, Box<dyn Error>> {
        let answers: Vec<String> = serde_json::from_str(ANSWER_DATA)?;
        let mut candidates: Vec<String> = serde_json::from_str(CANDIDATE_DATA)?;
        candidates.extend(answers.iter().cloned());
        if answers.is_empty() {
            return Err(GameError::EmptyAnswerList.into());
        }
        for word in &candidates {
            Word::try_from(word.as_str())?;
        }

        Ok(WordList {
            answers,
            candidates,
            #[cfg(all(feature = "solver", not(target_arch = "wasm32")))]
            candidate_words: OnceLock::new(),
        })
    }

    #[cfg(all(feature = "solver", not(target_arch = "wasm32")))]
    pub(crate) fn candidate_words(&self) -> &[Word] {
        self.candidate_words.get_or_init(|| {
            self.candidates
                .iter()
                .map(|word| {
                    Word::try_from(word.as_str()).expect("word list was validated at load time")
                })
                .collect()
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
    answer: Word,
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

        let answer = Word::try_from(words.answers[index].as_str()).expect("validated answer");
        Game {
            words,
            answer,
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
    #[cfg(all(feature = "solver", not(target_arch = "wasm32")))]
    pub(crate) fn word_list(&self) -> Arc<WordList> {
        Arc::clone(&self.words)
    }
    pub fn set_game_with_answer_index(&mut self, index: usize) -> Result<(), GameError> {
        let answer = self
            .answers()
            .get(index)
            .ok_or(GameError::InvalidAnswerIndex(index))?;
        self.answer = Word::try_from(answer.as_str())?;
        self.round = 0;
        self.state = GameState::On;
        Ok(())
    }
    pub fn set_game_with_answer(&mut self, answer: impl AsRef<str>) -> Result<(), GameError> {
        self.answer = Word::try_from(answer.as_ref())?;
        self.round = 0;
        self.state = GameState::On;
        Ok(())
    }
    pub fn grade_guess(&self, word: &str) -> Result<Match, GameError> {
        Ok(self.answer.grade(Word::try_from(word)?))
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
        self.answer.as_str()
    }
    pub fn reset(&mut self) {
        let mut index: usize = rand::random();
        index %= self.answers().len();

        self.set_game_with_answer_index(index)
            .expect("random index is within the answer list");
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
        self.states.iter().all(|&s| s == GuessState::Correct)
    }
    pub fn new() -> Match {
        Match {
            states: [GuessState::Wrong; 5],
        }
    }
}
#[derive(PartialEq, Debug)]
pub enum GameState {
    On,
    Correct,
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_words_are_rejected_without_changing_the_game() {
        let mut game = Game::new();
        game.set_game_with_answer("apple").unwrap();
        game.progress_game(&Match::new());
        for word in ["", "a", "applejunk", "APPLE", "app1e", "éabc", "abc🦀"] {
            assert_eq!(game.grade_guess(word), Err(GameError::InvalidWord));
            assert_eq!(game.set_game_with_answer(word), Err(GameError::InvalidWord));
            assert_eq!(game.answer(), "apple");
            assert_eq!(game.round(), 1);
        }
        assert_eq!(
            game.set_game_with_answer_index(usize::MAX),
            Err(GameError::InvalidAnswerIndex(usize::MAX))
        );
        assert_eq!(game.answer(), "apple");
        assert_eq!(game.round(), 1);
    }

    #[test]
    fn grading_matches_a_reference_for_all_three_letter_alphabet_pairs() {
        let words: Vec<_> = (0..243)
            .map(|mut i| {
                let mut bytes = [b'a'; 5];
                for byte in &mut bytes {
                    *byte += (i % 3) as u8;
                    i /= 3;
                }
                Word::try_from(bytes).unwrap()
            })
            .collect();
        for &answer in &words {
            for &guess in &words {
                let mut remaining = answer.0.map(Some);
                let mut expected = Match::new();
                for (i, &letter) in guess.0.iter().enumerate() {
                    if remaining[i] == Some(letter) {
                        expected.states[i] = GuessState::Correct;
                        remaining[i] = None;
                    }
                }
                for (state, &letter) in expected.states.iter_mut().zip(&guess.0) {
                    if *state == GuessState::Correct {
                        continue;
                    }
                    if let Some(slot) = remaining.iter_mut().find(|slot| **slot == Some(letter)) {
                        *state = GuessState::Misplace;
                        *slot = None;
                    }
                }
                assert_eq!(answer.grade(guess), expected, "{answer:?} / {guess:?}");
            }
        }
    }

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
    fn word_list_loads_answers_and_candidates_from_embedded_data() {
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
        game.set_game_with_answer("cigar").unwrap();

        let one_match = game.grade_guess("cigar").unwrap();

        assert_eq!(one_match.states, [GuessState::Correct; 5]);
    }

    #[test]
    fn grade_guess_marks_partial_and_mixed_matches_with_current_rules() {
        let mut game = Game::new();
        game.set_game_with_answer("cigar").unwrap();

        let partial = game.grade_guess("argon").unwrap();
        let mixed = game.grade_guess("cairn").unwrap();

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
        game.set_game_with_answer("cigar").unwrap();

        let one_match = game.grade_guess("blush").unwrap();

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

        game.set_game_with_answer("zonal").unwrap();
        assert_eq!(game.answer(), "zonal");

        game.set_game_with_answer_index(zonal_index).unwrap();
        assert_eq!(game.answer(), "zonal");
        assert_eq!(game.round(), 0);
        assert!(matches!(game.state, GameState::On));
    }

    #[test]
    fn set_game_with_answer_accepts_custom_answers() {
        let mut game = Game::new();

        game.set_game_with_answer("abcde").unwrap();

        assert_eq!(game.answer(), "abcde");
    }

    #[test]
    fn match_is_correct_only_when_every_slot_is_correct() {
        assert!(correct_match().is_correct());
        assert!(!Match::new().is_correct());
    }

    #[test]
    fn grade_guess_handles_duplicate_letters_in_guess() {
        let mut game = Game::new();
        game.set_game_with_answer("apple").unwrap();

        let one_match = game.grade_guess("eerie").unwrap();

        assert_eq!(
            one_match.states,
            [
                GuessState::Wrong,
                GuessState::Wrong,
                GuessState::Wrong,
                GuessState::Wrong,
                GuessState::Correct,
            ]
        );
    }

    #[test]
    fn grade_guess_caps_misplace_by_answer_letter_count() {
        let mut game = Game::new();
        game.set_game_with_answer("apple").unwrap();

        // Position 0 is an exact 'a' match, consuming the answer's single 'a'.
        // The remaining four 'a's in the guess must all be Wrong — not
        // Misplace — because the answer has no more 'a's to account for them.
        let one_match = game.grade_guess("aaaaa").unwrap();

        assert_eq!(
            one_match.states,
            [
                GuessState::Correct,
                GuessState::Wrong,
                GuessState::Wrong,
                GuessState::Wrong,
                GuessState::Wrong,
            ]
        );
    }
}
