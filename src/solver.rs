use super::game::*;
use std::io::prelude::*;
use std::sync::Arc;

const PATTERN_SIZE: usize = 243;

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
    words: Arc<WordList>,
    pub current_candidate: String,
    survive: usize,
}

fn grade_pair(word: &str, candidate: &str) -> usize {
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
    pub chars: [u8; 5],
    pub state: [GuessState; 5],
}

impl Solver {
    fn candidates(&self) -> &[String] {
        &self.words.candidates
    }

    pub fn bind(game: &Game) -> Solver {
        let table_size = game.candidates().len();
        let mut cache_strings = String::new();
        {
            let mut cache_file = std::fs::File::open("./data/cache").unwrap();
            cache_file.read_to_string(&mut cache_strings).unwrap();
        }

        Solver {
            patterns: Vec::new(),
            valid_table: vec![true; table_size],
            words: game.word_list(),
            current_candidate: String::new(),
            survive: table_size,
        }
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

        let mut score = -1.0;
        let mut index = candidates.len();

        #[cfg(debug_assertions)]
        let mut rank: Vec<(f64, &str)> = vec![];

        for (i, _) in candidates.iter().enumerate() {
            let new_score = self.calculate_score(i);

            #[cfg(debug_assertions)]
            rank.push((-new_score, candidates[i].as_str()));

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

        (Guess::Candidate(index), score)
    }
    pub fn try_guess(&mut self, guess: Guess, game: &mut Game) -> Option<Match> {
        let guess_word = guess.as_str(self.candidates());

        if !game.check_valid_guess(guess_word) {
            return None;
        }
        let one_match = game.grade_guess(guess_word);
        let guess_chars = guess_word.as_bytes().try_into().unwrap();
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
        self.valid_table = vec![true; self.candidates().len()];
        self.patterns = Vec::new();
        self.current_candidate = String::new();
    }

    fn filter_valid_word(&mut self) {
        let candidates = &self.words.candidates;
        let mut survive = candidates.len();
        for (table_index, candidate) in candidates.iter().enumerate() {
            if !self.valid_word(table_index) {
                survive -= 1;
                continue;
            }
            let word = candidate.as_str();
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
        let candidates = &self.words.candidates;
        let word = candidates[table_index].as_str();

        let mut score: f64 = 0.0;
        let mut total = 0;
        let mut pattern_matched = vec![0; PATTERN_SIZE];
        for (j, candidate) in candidates.iter().enumerate() {
            if self.valid_word(j) {
                total += 1;
                let pattern_index = grade_pair(candidate.as_str(), word);
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
    fn try_match(&self, word: &str, pattern: &Pattern) -> bool {
        let pattern_bytes = &pattern.chars;
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
        let solver = Solver::bind(&game);

        assert_eq!(solver.valid_table.len(), game.candidates().len());
        assert_eq!(solver.candidates().len(), game.candidates().len());
        assert_eq!(solver.survive, game.candidates().len());
        assert!(Arc::ptr_eq(&solver.words, &game.word_list()));
        assert!(solver.patterns.is_empty());
    }

    #[test]
    fn new_guess_returns_tares_for_opening_round() {
        let game = Game::new();
        let solver = Solver::bind(&game);

        let (guess, score) = solver.new_guess(0);

        assert_eq!(guess.as_str(game.candidates()), "tares");
        assert_eq!(score, 0.0);
    }

    #[test]
    fn try_guess_returns_match_for_valid_guess_and_none_for_invalid_guess() {
        let mut game = Game::new();
        game.set_game_with_answer("cigar");
        let mut solver = Solver::bind(&game);

        let valid_guess = Guess::Word("cigar".into());
        let invalid_guess = Guess::Word("xxxxx".into());

        let valid_match = solver.try_guess(valid_guess, &mut game);
        let invalid_match = solver.try_guess(invalid_guess, &mut game);

        assert!(valid_match.is_some());
        assert!(valid_match.unwrap().is_correct());
        assert!(invalid_match.is_none());
    }

    #[test]
    fn reset_restores_solver_state() {
        let mut game = Game::new();
        game.set_game_with_answer("cigar");
        let mut solver = Solver::bind(&game);

        let _ = solver.try_guess(
            Guess::Word("argon".into()),
            &mut game,
        );

        solver.reset();

        assert!(solver.patterns.is_empty());
        assert!(solver.valid_table.iter().all(|is_valid| *is_valid));
        assert_eq!(solver.current_candidate, "");
    }

    #[test]
    fn calculate_score_is_non_negative() {
        let game = Game::new();
        let solver = Solver::bind(&game);

        assert!(solver.calculate_score(0) >= 0.0);
    }

    #[test]
    fn solver_can_still_solve_zonal_with_current_behavior() {
        let mut game = Game::new();
        let mut solver = Solver::bind(&game);
        game.set_game_with_answer("zonal");
        solver.reset();

        let mut attempts = 0;

        loop {
            attempts += 1;
            let (guess, _score) = solver.new_guess(game.round() as u8);
            let one_match = solver.try_guess(guess, &mut game);

            if one_match.as_ref().is_some_and(|one_match| one_match.is_correct()) {
                break;
            }

            assert!(attempts < 20, "solver failed to solve zonal within 20 guesses");
        }

        assert!(attempts <= 6, "solver regressed to {attempts} guesses");
    }
}
