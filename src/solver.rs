use super::game::*;
use rayon::prelude::*;
use std::cmp::Ordering;
use std::error::Error;
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
    valid_table: Vec<bool>,
    valid_indices: Vec<u32>,
    words: Arc<WordList>,
    survive: usize,
}

#[inline]
fn bitvec_of(word: &[u8; 5]) -> u32 {
    let mut v = 0u32;
    v |= char_to_bitvec(word[0]);
    v |= char_to_bitvec(word[1]);
    v |= char_to_bitvec(word[2]);
    v |= char_to_bitvec(word[3]);
    v |= char_to_bitvec(word[4]);
    v
}

#[inline]
fn grade_pair_bytes(word: &[u8; 5], wordvec: u32, pattern: &[u8; 5]) -> usize {
    let mut idx = 0usize;
    for (&w, &p) in word.iter().zip(pattern.iter()) {
        idx *= 3;
        if w == p {
            idx += 2;
        } else if wordvec & char_to_bitvec(p) != 0 {
            idx += 1;
        }
    }
    idx
}

#[cfg(test)]
fn grade_pair(word: &str, candidate: &str) -> usize {
    let word_bytes: &[u8; 5] = word
        .as_bytes()
        .try_into()
        .expect("grade_pair word must be 5 bytes");
    let cand_bytes: &[u8; 5] = candidate
        .as_bytes()
        .try_into()
        .expect("grade_pair candidate must be 5 bytes");
    grade_pair_bytes(word_bytes, bitvec_of(word_bytes), cand_bytes)
}

#[inline]
fn char_to_bitvec(c: u8) -> u32 {
    1u32 << (c - 97)
}

/// Solver-internal grading. Matches the bitset-based semantics that
/// `try_match_bytes` expects (a letter is "in" the answer if present
/// anywhere, regardless of duplicate counts). The game's public
/// `grade_guess` uses correct Wordle duplicate handling for the UI;
/// the solver must stay on this older semantics so that
/// `filter_valid_word` does not reject the true answer. See
/// SOLVER_PERF.md for the quirk this preserves.
fn solver_grade(answer: &str, guess: &str) -> Match {
    let answer_bytes = answer.as_bytes();
    let guess_bytes = guess.as_bytes();
    let mut answer_mask = 0u32;
    for &b in answer_bytes {
        answer_mask |= char_to_bitvec(b);
    }
    let mut states = [GuessState::Wrong; 5];
    for i in 0..5 {
        if guess_bytes[i] == answer_bytes[i] {
            states[i] = GuessState::Correct;
        } else if answer_mask & char_to_bitvec(guess_bytes[i]) != 0 {
            states[i] = GuessState::Misplace;
        }
    }
    Match { states }
}

/// Check whether `word`, treated as a hypothetical answer, is consistent with
/// the guess + observed grading stored in `pattern`. Uses the grade_guess
/// direction (guess letter probed against answer's letter set).
#[inline]
fn try_match_bytes(word: &[u8; 5], pattern: &Pattern) -> bool {
    let wordvec = bitvec_of(word);
    for ((&w, &pc), &ps) in word
        .iter()
        .zip(pattern.chars.iter())
        .zip(pattern.state.iter())
    {
        if w == pc {
            if ps != GuessState::Correct {
                return false;
            }
        } else if wordvec & char_to_bitvec(pc) != 0 {
            if ps != GuessState::Misplace {
                return false;
            }
        } else if ps != GuessState::Wrong {
            return false;
        }
    }
    true
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pattern {
    pub chars: [u8; 5],
    pub state: [GuessState; 5],
}

fn decode_pattern(mut idx: usize) -> [GuessState; 5] {
    let mut out = [GuessState::Wrong; 5];
    for slot in (0..5).rev() {
        out[slot] = match idx % 3 {
            0 => GuessState::Wrong,
            1 => GuessState::Misplace,
            _ => GuessState::Correct,
        };
        idx /= 3;
    }
    out
}

fn encode_pattern(state: &[GuessState; 5]) -> usize {
    let mut idx = 0usize;
    for &s in state.iter() {
        idx *= 3;
        idx += match s {
            GuessState::Wrong => 0,
            GuessState::Misplace => 1,
            GuessState::Correct => 2,
        };
    }
    idx
}

fn score_over(
    cand_bytes: &[[u8; 5]],
    cand_bitvecs: &[u32],
    valid: &[u32],
    guess_idx: usize,
) -> f64 {
    let guess_bytes = &cand_bytes[guess_idx];
    let mut pattern_matched = [0u32; PATTERN_SIZE];
    let total = valid.len();
    for &idx in valid {
        let c = &cand_bytes[idx as usize];
        let cv = cand_bitvecs[idx as usize];
        let p = grade_pair_bytes(c, cv, guess_bytes);
        pattern_matched[p] += 1;
    }
    let mut score = 0.0f64;
    let total_f = total as f64;
    for &count in pattern_matched.iter() {
        if count != 0 {
            let p = count as f64 / total_f;
            score -= p * p.log2();
        }
    }
    score
}

fn best_over(cand_bytes: &[[u8; 5]], cand_bitvecs: &[u32], valid: &[u32]) -> (usize, f64) {
    (0..cand_bytes.len())
        .into_par_iter()
        .map(|g| (g, score_over(cand_bytes, cand_bitvecs, valid, g)))
        .reduce(
            || (cand_bytes.len(), f64::NEG_INFINITY),
            |a, b| match b.1.total_cmp(&a.1) {
                Ordering::Greater => b,
                Ordering::Equal if b.0 < a.0 => b,
                _ => a,
            },
        )
}

/// Precomputed best round-1 guess (and its score) keyed by the 243 possible
/// gradings of the opening word "tares". `None` entries correspond to
/// unreachable patterns (no surviving candidate).
fn precomputed_second_guess(words: &WordList) -> &'static [Option<(u16, u64)>; PATTERN_SIZE] {
    static TABLE: OnceLock<[Option<(u16, u64)>; PATTERN_SIZE]> = OnceLock::new();
    TABLE.get_or_init(|| {
        let cand_bytes = &words.candidate_bytes;
        let cand_bitvecs = &words.candidate_bitvecs;
        let mut out = [None; PATTERN_SIZE];
        for (pat_idx, slot) in out.iter_mut().enumerate() {
            let state = decode_pattern(pat_idx);
            let pattern = Pattern {
                chars: TARES_BYTES,
                state,
            };
            let mut valid: Vec<u32> = Vec::new();
            for (i, c) in cand_bytes.iter().enumerate() {
                if try_match_bytes(c, &pattern) {
                    valid.push(i as u32);
                }
            }
            if valid.is_empty() {
                continue;
            }
            if valid.len() == 1 {
                *slot = Some((valid[0] as u16, 0.0f64.to_bits()));
                continue;
            }
            let (idx, score) = best_over(cand_bytes, cand_bitvecs, &valid);
            *slot = Some((idx as u16, score.to_bits()));
        }
        out
    })
}

impl Solver {
    fn candidates(&self) -> &[String] {
        &self.words.candidates
    }

    pub fn bind(game: &Game) -> Result<Solver, Box<dyn Error>> {
        let n = game.candidates().len();

        Ok(Solver {
            patterns: Vec::new(),
            valid_table: vec![true; n],
            valid_indices: (0..n as u32).collect(),
            words: game.word_list(),
            survive: n,
        })
    }
    pub fn new_guess(&self, round: u8) -> (Guess, f64) {
        let candidates = self.candidates();

        if round == 0 {
            return (Guess::OpeningWord, 0.0);
        }
        if self.survive == 1 {
            for i in 0..candidates.len() {
                if self.valid_word(i) {
                    return (Guess::Candidate(i), 0.0);
                }
            }
        }

        if round == 1 && self.patterns.len() == 1 && self.patterns[0].chars == TARES_BYTES {
            let pat_idx = encode_pattern(&self.patterns[0].state);
            if let Some((idx, score_bits)) = precomputed_second_guess(&self.words)[pat_idx] {
                return (Guess::Candidate(idx as usize), f64::from_bits(score_bits));
            }
        }

        let (index, score) = (0..candidates.len())
            .into_par_iter()
            .map(|i| (i, self.calculate_score(i)))
            .reduce(
                || (candidates.len(), f64::NEG_INFINITY),
                |a, b| match b.1.total_cmp(&a.1) {
                    Ordering::Greater => b,
                    Ordering::Equal if b.0 < a.0 => b,
                    _ => a,
                },
            );

        #[cfg(debug_assertions)]
        {
            let mut rank: Vec<(f64, &str)> = candidates
                .iter()
                .enumerate()
                .map(|(i, w)| (-self.calculate_score(i), w.as_str()))
                .collect();
            rank.sort_by(|a, b| {
                a.partial_cmp(b)
                    .expect("solver scores should always be comparable")
            });
            for (x, y) in rank.into_iter().take(100) {
                println!("{}: {}", y, -x);
            }
        }

        (Guess::Candidate(index), score)
    }
    pub fn try_guess(&mut self, guess: Guess, game: &mut Game) -> Option<Match> {
        let guess_word = guess.as_str(self.candidates());

        if !game.check_valid_guess(guess_word) {
            return None;
        }
        let one_match = solver_grade(game.answer(), guess_word);
        let guess_chars = guess_word
            .as_bytes()
            .try_into()
            .expect("guess word must be exactly 5 bytes");
        game.progress_game(&one_match);
        self.add_pattern(guess_chars, &one_match);
        #[cfg(debug_assertions)]
        println!("{:?}", one_match);
        self.filter_valid_word();
        Some(one_match)
    }
    fn valid_word(&self, table_index: usize) -> bool {
        self.valid_table[table_index]
    }
    pub fn reset(&mut self) {
        let n = self.candidates().len();
        self.valid_table = vec![true; n];
        self.valid_indices = (0..n as u32).collect();
        self.patterns = Vec::new();
        self.survive = n;
    }

    fn filter_valid_word(&mut self) {
        let old_indices = std::mem::take(&mut self.valid_indices);
        let words = Arc::clone(&self.words);
        let cand_bytes = &words.candidate_bytes;
        let mut new_valid = Vec::with_capacity(old_indices.len());
        for idx in old_indices {
            let c = &cand_bytes[idx as usize];
            let keep = self.patterns.iter().all(|p| try_match_bytes(c, p));
            if keep {
                new_valid.push(idx);
            } else {
                self.valid_table[idx as usize] = false;
            }
        }
        self.survive = new_valid.len();
        self.valid_indices = new_valid;
    }

    fn calculate_score(&self, guess_idx: usize) -> f64 {
        let cand_bytes = &self.words.candidate_bytes;
        let cand_bitvecs = &self.words.candidate_bitvecs;
        let guess_bytes = &cand_bytes[guess_idx];

        let mut pattern_matched = [0u32; PATTERN_SIZE];
        let total = self.valid_indices.len();
        for &idx in &self.valid_indices {
            let c = &cand_bytes[idx as usize];
            let cv = cand_bitvecs[idx as usize];
            // grade_pair direction from the original code:
            // word_arg = candidate (so wordvec = candidate's bits),
            // pattern_arg = guess. Probes guess letter against candidate's bits.
            let p = grade_pair_bytes(c, cv, guess_bytes);
            pattern_matched[p] += 1;
        }

        let mut score = 0.0f64;
        let total_f = total as f64;
        for &count in pattern_matched.iter() {
            if count != 0 {
                let p = count as f64 / total_f;
                score -= p * p.log2();
            }
        }
        score
    }

    pub fn add_pattern(&mut self, word: [u8; 5], one_match: &Match) {
        self.patterns = vec![Pattern {
            chars: word,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bind_initializes_candidate_tracking_from_game() {
        let game = Game::new();
        let solver = Solver::bind(&game).expect("failed to bind solver to game data");

        assert_eq!(solver.valid_table.len(), game.candidates().len());
        assert_eq!(solver.candidates().len(), game.candidates().len());
        assert_eq!(solver.survive, game.candidates().len());
        assert!(Arc::ptr_eq(&solver.words, &game.word_list()));
        assert!(solver.patterns.is_empty());
    }

    #[test]
    fn bind_does_not_change_existing_game_state() {
        let mut game = Game::new();
        game.set_game_with_answer("zonal");
        let round = game.round();
        let answer = game.answer().to_string();

        let _solver = Solver::bind(&game).expect("failed to bind solver to game data");

        assert_eq!(game.answer(), answer);
        assert_eq!(game.round(), round);
        assert!(matches!(game.state, GameState::On));
    }

    #[test]
    fn new_guess_returns_tares_for_opening_round() {
        let game = Game::new();
        let solver = Solver::bind(&game).expect("failed to bind solver to game data");

        let (guess, score) = solver.new_guess(0);

        assert_eq!(guess.as_str(game.candidates()), "tares");
        assert_eq!(score, 0.0);
    }

    #[test]
    fn try_guess_returns_match_for_valid_guess_and_none_for_invalid_guess() {
        let mut game = Game::new();
        game.set_game_with_answer("cigar");
        let mut solver = Solver::bind(&game).expect("failed to bind solver to game data");

        let valid_guess = Guess::Word("cigar".into());
        let invalid_guess = Guess::Word("xxxxx".into());

        let valid_match = solver.try_guess(valid_guess, &mut game);
        let invalid_match = solver.try_guess(invalid_guess, &mut game);

        assert!(valid_match.is_some());
        assert!(valid_match
            .expect("valid guess should produce a match")
            .is_correct());
        assert!(invalid_match.is_none());
    }

    #[test]
    fn reset_restores_solver_state() {
        let mut game = Game::new();
        game.set_game_with_answer("cigar");
        let mut solver = Solver::bind(&game).expect("failed to bind solver to game data");

        let _ = solver.try_guess(Guess::Word("argon".into()), &mut game);

        solver.reset();

        assert!(solver.patterns.is_empty());
        assert!(solver.valid_table.iter().all(|is_valid| *is_valid));
        assert_eq!(solver.survive, solver.candidates().len());
    }

    #[test]
    fn calculate_score_is_non_negative() {
        let game = Game::new();
        let solver = Solver::bind(&game).expect("failed to bind solver to game data");

        assert!(solver.calculate_score(0) >= 0.0);
    }

    fn run_solver_sequence(answer: &str) -> Vec<String> {
        let mut game = Game::new();
        let mut solver = Solver::bind(&game).expect("failed to bind solver to game data");
        game.set_game_with_answer(answer);
        solver.reset();
        let mut seq: Vec<String> = Vec::new();
        loop {
            let (guess, _score) = solver.new_guess(game.round() as u8);
            let word = guess.as_str(game.candidates()).to_string();
            seq.push(word);
            let one_match = solver.try_guess(guess, &mut game);
            if one_match.as_ref().is_some_and(|m| m.is_correct()) {
                break;
            }
            if seq.len() > 20 {
                break;
            }
        }
        seq
    }

    #[test]
    fn golden_guess_sequences() {
        let goldens: &[(&str, &[&str])] = &[
            ("zonal", &["tares", "colin", "panax", "abamp", "zonal"]),
            ("cigar", &["tares", "broil", "micra", "cigar"]),
            ("rouge", &["tares", "dogie", "rouge"]),
            ("proxy", &["tares", "bound", "crool", "apery", "proxy"]),
            ("slate", &["tares", "stalk", "slate"]),
            ("aback", &["tares", "colin", "bunya", "aback"]),
            ("whack", &["tares", "colin", "bunya", "whack"]),
            ("robot", &["tares", "fruit", "abamp", "robot"]),
            ("crane", &["tares", "beard", "campi", "kanzu", "crane"]),
            ("abbey", &["tares", "blind", "abaca", "abbey"]),
        ];
        for (answer, expected) in goldens {
            let seq = run_solver_sequence(answer);
            let actual: Vec<&str> = seq.iter().map(String::as_str).collect();
            assert_eq!(
                actual.as_slice(),
                *expected,
                "guess sequence regression for answer {answer}"
            );
        }
    }

    #[test]
    fn grade_pair_quirk_pinned() {
        // grade_pair intentionally uses reversed bitset semantics compared to
        // game::grade_guess. This quirk is preserved on purpose; do not "fix"
        // without re-baselining golden_guess_sequences.
        assert_eq!(grade_pair("abbed", "beads"), 120);
        assert_eq!(grade_pair("tares", "zonal"), 3);
        assert_eq!(grade_pair("cigar", "cigar"), 242);
        assert_eq!(grade_pair("blush", "cigar"), 0);
    }

    #[test]
    fn calculate_score_bitwise_stable() {
        // Pins bitwise-identical entropy for guess_idx=0 on a fresh solver.
        // Catches floating-point reordering introduced by refactors.
        let game = Game::new();
        let solver = Solver::bind(&game).expect("failed to bind solver to game data");
        let score = solver.calculate_score(0);
        assert_eq!(score.to_bits(), 0x40112a919150b453u64);
    }

    #[test]
    fn second_guess_table_matches_fresh_search() {
        // For a sampling of reachable tares-grading patterns, the precomputed
        // table must agree with a fresh search done by the normal `new_guess`
        // path (so the lookup can never drift from the search).
        let game = Game::new();
        let words = game.word_list();
        let table = precomputed_second_guess(&words);

        for pat_idx in [0usize, 1, 3, 5, 9, 27, 81, 100, 121, 200, 242] {
            let state = decode_pattern(pat_idx);
            // Seed a solver as if "tares" had been guessed with this grade,
            // without depending on grade_guess (which would constrain the
            // answer shape).
            let mut solver = Solver::bind(&game).expect("bind");
            solver.patterns = vec![Pattern {
                chars: TARES_BYTES,
                state,
            }];
            solver.filter_valid_word();
            let expected = table[pat_idx];

            if solver.survive == 0 {
                assert!(expected.is_none(), "expected None for pat_idx {pat_idx}");
                continue;
            }

            // Drive the normal round-1 search, but suppress the table lookup
            // by temporarily switching the recorded chars. This forces a
            // fresh parallel reduction to compute ground truth.
            solver.patterns[0].chars = *b"xxxxx";
            let (fresh_guess, fresh_score) = solver.new_guess(1);
            let fresh_idx = match fresh_guess {
                Guess::Candidate(i) => i,
                _ => panic!("expected Candidate guess"),
            };

            let (tbl_idx, tbl_score_bits) =
                expected.expect("table entry should exist when survivors > 0");

            // survive==1 short-circuit returns (first_valid, 0.0), while the
            // table records the same first_valid index with score 0.0.
            if solver.survive == 1 {
                assert_eq!(tbl_idx as usize, fresh_idx, "pat_idx {pat_idx}");
                continue;
            }

            assert_eq!(tbl_idx as usize, fresh_idx, "pat_idx {pat_idx}");
            assert_eq!(
                tbl_score_bits,
                fresh_score.to_bits(),
                "pat_idx {pat_idx} score drift"
            );
        }
    }

    fn serial_best_guess(solver: &Solver) -> (usize, f64) {
        let candidates = solver.candidates();
        let mut best_index = candidates.len();
        let mut best_score = f64::NEG_INFINITY;
        for i in 0..candidates.len() {
            let s = solver.calculate_score(i);
            if s > best_score {
                best_score = s;
                best_index = i;
            }
        }
        (best_index, best_score)
    }

    #[test]
    fn parallel_equals_serial() {
        // After a real guess, the parallel reduction in `new_guess` must
        // produce the same winning (index, score) as a strict-greater serial
        // scan. Ties break toward the lowest index in both, so they must agree.
        let mut game = Game::new();
        game.set_game_with_answer("cigar");
        let mut solver = Solver::bind(&game).expect("failed to bind solver to game data");
        let _ = solver.try_guess(Guess::Word("tares".into()), &mut game);

        let (serial_idx, serial_score) = serial_best_guess(&solver);
        let (parallel_guess, parallel_score) = solver.new_guess(game.round() as u8);
        let parallel_word = parallel_guess.as_str(solver.candidates());

        assert_eq!(parallel_word, solver.candidates()[serial_idx].as_str());
        assert_eq!(parallel_score.to_bits(), serial_score.to_bits());
    }

    #[test]
    fn solver_can_still_solve_zonal_with_current_behavior() {
        let mut game = Game::new();
        let mut solver = Solver::bind(&game).expect("failed to bind solver to game data");
        game.set_game_with_answer("zonal");
        solver.reset();

        let mut attempts = 0;

        loop {
            attempts += 1;
            let (guess, _score) = solver.new_guess(game.round() as u8);
            let one_match = solver.try_guess(guess, &mut game);

            if one_match
                .as_ref()
                .is_some_and(|one_match| one_match.is_correct())
            {
                break;
            }

            assert!(
                attempts < 20,
                "solver failed to solve zonal within 20 guesses"
            );
        }

        assert!(attempts <= 6, "solver regressed to {attempts} guesses");
    }
}
