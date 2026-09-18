use rayon::prelude::*;
use rordle::game::Game;
use rordle::solver::Solver;
use std::process::ExitCode;
use std::time::Instant;

const MAX_ATTEMPTS: usize = 21;
// Regression limits for the full answer list using proper Wordle feedback.
const MAX_AVERAGE: f64 = 4.2;
const MAX_FAILURES: usize = 0;

#[derive(Debug, Default)]
struct Stats {
    games: usize,
    attempts: usize,
    failures: usize,
    unsolved: usize,
}

impl Stats {
    fn record(&mut self, attempts: usize, solved: bool) {
        self.games += 1;
        self.attempts += attempts;
        self.failures += usize::from(!solved || attempts > 6);
        self.unsolved += usize::from(!solved);
    }

    fn merge(mut self, other: Self) -> Self {
        self.games += other.games;
        self.attempts += other.attempts;
        self.failures += other.failures;
        self.unsolved += other.unsolved;
        self
    }

    fn average(&self) -> Option<f64> {
        (self.games > 0).then(|| self.attempts as f64 / self.games as f64)
    }

    fn passes_regression(&self) -> bool {
        self.unsolved == 0
            && self.failures == MAX_FAILURES
            && self.average().is_some_and(|average| average <= MAX_AVERAGE)
    }
}

fn run_game(game: &mut Game, solver: &mut Solver, verbose: bool) -> Stats {
    solver.reset();
    let mut stats = Stats::default();
    for round in 0..MAX_ATTEMPTS {
        let (guess, score) = solver.new_guess(round);
        if verbose {
            println!(
                "{} {} {}",
                round + 1,
                guess.as_str(game.candidates()),
                score
            );
        }
        let result = solver
            .try_guess(guess, game)
            .expect("solver must choose a dictionary word");
        if result.is_correct() {
            stats.record(round + 1, true);
            return stats;
        }
    }
    stats.record(MAX_ATTEMPTS, false);
    stats
}

fn solve_all() -> Stats {
    let total = Game::new().answers().len();
    (0..total)
        .into_par_iter()
        .fold(
            || {
                let game = Game::new();
                let solver = Solver::bind(&game);
                (game, solver, Stats::default())
            },
            |(mut game, mut solver, stats), index| {
                game.set_game_with_answer_index(index)
                    .expect("enumerated answer index");
                let result = run_game(&mut game, &mut solver, false);
                (game, solver, stats.merge(result))
            },
        )
        .map(|(_, _, stats)| stats)
        .reduce(Stats::default, Stats::merge)
}

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let start = Instant::now();
    let (stats, check) = match args.as_slice() {
        [] => (solve_all(), false),
        [flag] if flag == "--check" => (solve_all(), true),
        [flag, answer] if flag == "--word" => {
            let mut game = Game::new();
            if let Err(error) = game.set_game_with_answer(answer) {
                eprintln!("{error}");
                return ExitCode::FAILURE;
            }
            let mut solver = Solver::bind(&game);
            (run_game(&mut game, &mut solver, true), false)
        }
        _ => {
            eprintln!("usage: solver [--check | --word WORD]");
            return ExitCode::FAILURE;
        }
    };
    println!("Total games: {}", stats.games);
    println!(
        "Total guesses (including unsolved games): {}",
        stats.attempts
    );
    println!(
        "Total failures (over six guesses or unsolved): {}",
        stats.failures
    );
    println!("Total unsolved: {}", stats.unsolved);
    if let Some(average) = stats.average() {
        println!("Average trials (unsolved capped at {MAX_ATTEMPTS}): {average}");
    }
    println!("Elapsed: {:?}", start.elapsed());
    if check && !stats.passes_regression() {
        eprintln!("solver regression: expected no unsolved games, at most {MAX_FAILURES} failures, and average <= {MAX_AVERAGE}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statistics_include_unsolved_attempts_and_failures() {
        let mut stats = Stats::default();
        stats.record(4, true);
        let mut other = Stats::default();
        other.record(MAX_ATTEMPTS, false);
        let stats = stats.merge(other);
        assert_eq!(stats.games, 2);
        assert_eq!(stats.attempts, 25);
        assert_eq!(stats.failures, 1);
        assert_eq!(stats.unsolved, 1);
        assert_eq!(stats.average(), Some(12.5));
        assert!(!stats.passes_regression());
    }

    #[test]
    fn regression_check_rejects_empty_and_degraded_runs() {
        assert!(!Stats::default().passes_regression());
        let mut good = Stats::default();
        good.record(4, true);
        assert!(good.passes_regression());
        let mut slow = Stats::default();
        slow.record(5, true);
        assert!(!slow.passes_regression());
    }
}
