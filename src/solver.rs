use crate::game::{Game, GameError, GuessState, Match, Word, WordList};
use rayon::prelude::*;
use std::cmp::Ordering;
use std::sync::{Arc, OnceLock};

const PATTERN_SIZE: usize = 243;
const TARES_BYTES: [u8; 5] = *b"tares";

#[derive(Debug)]
pub enum Guess {
    OpeningWord,
    Candidate(usize),
    Word(Box<str>),
}

impl Guess {
    pub fn as_str<'a>(&'a self, candidates: &'a [String]) -> &'a str {
        match self {
            Guess::OpeningWord => "tares",
            Guess::Candidate(index) => candidates[*index].as_str(),
            Guess::Word(word) => word,
        }
    }
}

#[derive(Debug)]
pub struct Solver {
    patterns: Vec<Pattern>,
    valid_indices: Vec<usize>,
    words: Arc<WordList>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pattern {
    pub chars: [u8; 5],
    pub state: [GuessState; 5],
}

fn encode_pattern(state: &[GuessState; 5]) -> usize {
    state.iter().fold(0, |idx, s| {
        idx * 3
            + match s {
                GuessState::Wrong => 0,
                GuessState::Misplace => 1,
                GuessState::Correct => 2,
            }
    })
}

fn score_over(words: &[Word], valid: &[usize], guess_idx: usize) -> f64 {
    let guess = words[guess_idx];
    let mut pattern_matched = [0u32; PATTERN_SIZE];
    for &idx in valid {
        pattern_matched[encode_pattern(&words[idx].grade(guess).states)] += 1;
    }
    let mut score = 0.0;
    for count in pattern_matched {
        if count != 0 {
            let p = count as f64 / valid.len() as f64;
            score -= p * p.log2();
        }
    }
    score
}

fn better_guess(a: (usize, f64), b: (usize, f64)) -> (usize, f64) {
    match b.1.total_cmp(&a.1) {
        Ordering::Greater => b,
        Ordering::Equal if b.0 < a.0 => b,
        _ => a,
    }
}

fn best_over(words: &[Word], valid: &[usize]) -> (usize, f64) {
    // Batch callers already distribute games across Rayon workers. Score each
    // such game serially, avoiding nested scheduling and repeated cache misses.
    if rayon::current_thread_index().is_some() {
        return (0..words.len())
            .map(|g| (g, score_over(words, valid, g)))
            .fold((words.len(), f64::NEG_INFINITY), better_guess);
    }
    (0..words.len())
        .into_par_iter()
        .map(|g| (g, score_over(words, valid, g)))
        .reduce(|| (words.len(), f64::NEG_INFINITY), better_guess)
}

struct SecondGuessCache {
    entries: [OnceLock<(usize, f64)>; PATTERN_SIZE],
}

impl SecondGuessCache {
    const fn new() -> Self {
        Self {
            entries: [const { OnceLock::new() }; PATTERN_SIZE],
        }
    }

    fn get_or_compute(
        &self,
        pattern: usize,
        compute: impl FnOnce() -> (usize, f64),
    ) -> (usize, f64) {
        let entry = &self.entries[pattern];
        if let Some(&result) = entry.get() {
            return result;
        }
        // Do not hold an initialization lock across Rayon work: another task on
        // the same pool may request this entry while the search is running.
        // Concurrent misses may duplicate work, but publish the same result.
        let result = compute();
        let _ = entry.set(result);
        result
    }
}

static SECOND_GUESSES: SecondGuessCache = SecondGuessCache::new();

impl Solver {
    pub fn bind(game: &Game) -> Self {
        Self {
            patterns: Vec::with_capacity(6),
            valid_indices: (0..game.candidates().len()).collect(),
            words: game.word_list(),
        }
    }

    pub fn new_guess(&self, round: usize) -> (Guess, f64) {
        if round == 0 {
            return (Guess::OpeningWord, 0.0);
        }
        if let [index] = self.valid_indices.as_slice() {
            return (Guess::Candidate(*index), 0.0);
        }

        let search = || best_over(self.words.candidate_words(), &self.valid_indices);
        let (index, score) =
            if round == 1 && self.patterns.len() == 1 && self.patterns[0].chars == TARES_BYTES {
                SECOND_GUESSES.get_or_compute(encode_pattern(&self.patterns[0].state), search)
            } else {
                search()
            };
        (Guess::Candidate(index), score)
    }

    pub fn try_guess(&mut self, guess: Guess, game: &mut Game) -> Option<Match> {
        let guess_word = match &guess {
            Guess::Candidate(index) => self.words.candidates.get(*index)?.as_str(),
            _ => guess.as_str(&self.words.candidates),
        };
        if !game.check_valid_guess(guess_word) {
            return None;
        }
        let result = game.grade_guess(guess_word).ok()?;
        let chars = guess_word.as_bytes().try_into().ok()?;
        self.add_pattern(chars, &result).ok()?;
        game.progress_game(&result);
        Some(result)
    }

    pub fn reset(&mut self) {
        self.valid_indices.clear();
        self.valid_indices.extend(0..self.words.candidates.len());
        self.patterns.clear();
    }

    /// Apply observed feedback immediately, retaining every earlier constraint.
    /// Malformed words are rejected without changing the solver.
    pub fn add_pattern(&mut self, word: [u8; 5], result: &Match) -> Result<(), GameError> {
        let guess = Word::try_from(word)?;
        let words = self.words.candidate_words();
        self.valid_indices
            .retain(|&idx| words[idx].grade(guess) == *result);
        self.patterns.push(Pattern {
            chars: word,
            state: result.states,
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bind_preserves_game_and_includes_all_allowed_guesses() {
        let mut game = Game::new();
        game.set_game_with_answer("zonal").unwrap();
        let solver = Solver::bind(&game);
        assert_eq!(solver.valid_indices.len(), game.candidates().len());
        assert!(solver.valid_indices.len() > game.answers().len());
        assert!(Arc::ptr_eq(&solver.words, &game.word_list()));
        assert_eq!(game.answer(), "zonal");
        assert_eq!(game.round(), 0);
        assert!(solver.patterns.is_empty());
        assert_eq!(solver.new_guess(0).0.as_str(game.candidates()), "tares");
    }

    #[test]
    fn simulated_and_external_feedback_agree_for_duplicate_letters() {
        for (answer, guess) in [("apple", "eerie"), ("abbey", "babes"), ("cigar", "civic")] {
            let mut game = Game::new();
            game.set_game_with_answer(answer).unwrap();
            let mut simulated = Solver::bind(&game);
            let mut external = Solver::bind(&game);
            let result = game.grade_guess(guess).unwrap();
            external
                .add_pattern(guess.as_bytes().try_into().unwrap(), &result)
                .unwrap();
            assert_eq!(
                simulated.try_guess(Guess::Word(guess.into()), &mut game),
                Some(result)
            );
            assert_eq!(simulated.valid_indices, external.valid_indices);
            assert!(external
                .valid_indices
                .iter()
                .any(|&i| game.candidates()[i] == answer));
        }
    }

    #[test]
    fn external_feedback_applies_immediately_and_accumulates() {
        let mut game = Game::new();
        game.set_game_with_answer("cigar").unwrap();
        let mut solver = Solver::bind(&game);
        for guess in ["tares", "cairn"] {
            solver
                .add_pattern(
                    guess.as_bytes().try_into().unwrap(),
                    &game.grade_guess(guess).unwrap(),
                )
                .unwrap();
        }
        let expected: Vec<_> = game
            .candidates()
            .iter()
            .enumerate()
            .filter_map(|(i, word)| {
                let mut hypothetical = Game::new();
                hypothetical.set_game_with_answer(word).unwrap();
                ["tares", "cairn"]
                    .iter()
                    .all(|guess| hypothetical.grade_guess(guess) == game.grade_guess(guess))
                    .then_some(i)
            })
            .collect();
        assert_eq!(solver.valid_indices, expected);
        solver
            .add_pattern(*b"cigar", &game.grade_guess("cigar").unwrap())
            .unwrap();
        assert_eq!(solver.new_guess(2).0.as_str(game.candidates()), "cigar");
    }

    #[test]
    fn invalid_guesses_and_patterns_leave_state_unchanged() {
        let mut game = Game::new();
        let mut solver = Solver::bind(&game);
        let original = solver.valid_indices.clone();
        assert!(solver
            .try_guess(Guess::Word("xxxxx".into()), &mut game)
            .is_none());
        assert!(solver
            .try_guess(Guess::Candidate(usize::MAX), &mut game)
            .is_none());
        assert_eq!(
            solver.add_pattern(*b"APPLE", &Match::new()),
            Err(GameError::InvalidWord)
        );
        assert_eq!(solver.valid_indices, original);
        assert!(solver.patterns.is_empty());
        assert_eq!(game.round(), 0);
    }

    #[test]
    fn reset_restores_candidates_and_reuses_storage() {
        let mut game = Game::new();
        game.set_game_with_answer("cigar").unwrap();
        let mut solver = Solver::bind(&game);
        let allocation = solver.valid_indices.as_ptr();
        solver.try_guess(Guess::OpeningWord, &mut game).unwrap();
        solver.reset();
        assert_eq!(
            solver.valid_indices,
            (0..game.candidates().len()).collect::<Vec<_>>()
        );
        assert_eq!(solver.valid_indices.as_ptr(), allocation);
        assert!(solver.patterns.is_empty());
    }

    #[test]
    fn empty_candidate_behavior_is_preserved() {
        let game = Game::new();
        let mut solver = Solver::bind(&game);
        solver
            .add_pattern(
                *b"zzzzz",
                &Match {
                    states: [GuessState::Correct; 5],
                },
            )
            .unwrap();
        assert!(solver.valid_indices.is_empty());
        let (guess, score) = solver.new_guess(2);
        assert_eq!(guess.as_str(game.candidates()), game.candidates()[0]);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn second_guess_cache_is_lazy_and_matches_fresh_search() {
        let cache = SecondGuessCache::new();
        let mut game = Game::new();
        game.set_game_with_answer("cigar").unwrap();
        let mut solver = Solver::bind(&game);
        let result = solver.try_guess(Guess::OpeningWord, &mut game).unwrap();
        let pattern = encode_pattern(&result.states);
        let fresh = best_over(solver.words.candidate_words(), &solver.valid_indices);
        assert_eq!(cache.get_or_compute(pattern, || fresh), fresh);
        assert_eq!(
            cache.get_or_compute(pattern, || panic!("cached entry recomputed")),
            fresh
        );
        assert_eq!(
            cache
                .entries
                .iter()
                .filter(|entry| entry.get().is_some())
                .count(),
            1
        );
        let (guess, score) = solver.new_guess(1);
        assert_eq!(guess.as_str(game.candidates()), game.candidates()[fresh.0]);
        assert_eq!(score.to_bits(), fresh.1.to_bits());
    }

    #[test]
    fn parallel_search_matches_serial_with_deterministic_ties() {
        let mut game = Game::new();
        game.set_game_with_answer("cigar").unwrap();
        let mut solver = Solver::bind(&game);
        solver.try_guess(Guess::OpeningWord, &mut game).unwrap();
        let words = solver.words.candidate_words();
        let expected = (0..words.len())
            .map(|i| (i, score_over(words, &solver.valid_indices, i)))
            .fold((words.len(), f64::NEG_INFINITY), better_guess);
        let (guess, score) = solver.new_guess(2);
        assert_eq!(
            guess.as_str(game.candidates()),
            game.candidates()[expected.0]
        );
        assert_eq!(score.to_bits(), expected.1.to_bits());
    }

    #[test]
    fn golden_guess_sequences_with_wordle_feedback() {
        let goldens: &[(&str, &[&str])] = &[
            ("zonal", &["tares", "colin", "panda", "abamp", "zonal"]),
            ("cigar", &["tares", "broil", "micra", "cigar"]),
            ("rouge", &["tares", "deice", "bourg", "rouge"]),
            ("proxy", &["tares", "bound", "crool", "gawps", "proxy"]),
            ("slate", &["tares", "stalk", "slate"]),
            ("aback", &["tares", "colin", "bunya", "aback"]),
            ("whack", &["tares", "colin", "bunya", "whack"]),
            ("robot", &["tares", "fruit", "abamp", "robot"]),
            ("crane", &["tares", "beard", "campi", "kanzu", "crane"]),
            ("abbey", &["tares", "blind", "abaca", "abbey"]),
        ];
        for &(answer, expected) in goldens {
            let mut game = Game::new();
            game.set_game_with_answer(answer).unwrap();
            let mut solver = Solver::bind(&game);
            let mut actual = Vec::new();
            for round in 0..6 {
                let (guess, _) = solver.new_guess(round);
                actual.push(guess.as_str(game.candidates()).to_owned());
                if solver.try_guess(guess, &mut game).unwrap().is_correct() {
                    break;
                }
            }
            assert_eq!(actual, expected, "guess sequence for {answer}");
        }
    }

    #[test]
    fn representative_answers_solve_using_real_game_feedback() {
        for answer in [
            "zonal", "cigar", "rouge", "proxy", "slate", "aback", "whack", "robot", "crane",
            "abbey", "apple", "eerie",
        ] {
            let mut game = Game::new();
            game.set_game_with_answer(answer).unwrap();
            let mut solver = Solver::bind(&game);
            let solved = (0..6).any(|round| {
                let (guess, _) = solver.new_guess(round);
                let word = guess.as_str(game.candidates());
                assert!(game.check_valid_guess(word));
                let result = game.grade_guess(word).unwrap();
                solver
                    .add_pattern(word.as_bytes().try_into().unwrap(), &result)
                    .unwrap();
                result.is_correct()
            });
            assert!(solved, "failed to solve {answer} in six guesses");
        }
    }
}
