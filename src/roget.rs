use rordle::game::*;
use rordle::solver::*;
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    //solve_one();
    solve_all();
}

#[allow(dead_code)]
fn solve_one() {
    let mut game = Game::new();
    let mut solver = Solver::bind(&game).expect("failed to bind solver to game data");
    game.set_game_with_answer("zonal");
    println!("answer is {:}", game.answer());
    solver.reset();
    let mut count = 0;

    loop {
        count += 1;
        let (guess, score) = solver.new_guess(game.round() as u8);

        println!("{} {} {}", count, guess.as_str(game.candidates()), score);

        let one_match = solver.try_guess(guess, &mut game);

        if one_match.as_ref().is_some_and(Match::is_correct) {
            break;
        }
        if count > 20 {
            break;
        }
    }
}

#[allow(dead_code)]
fn solve_all() {
    let sum = Arc::new(Mutex::new(0));
    let fail = Arc::new(Mutex::new(0));
    let unsolve = Arc::new(Mutex::new(0));
    let count = Arc::new(Mutex::new(0));
    let total_thread = 8;
    let mut handlers = Vec::new();
    for t in 0..total_thread {
        let sum = Arc::clone(&sum);
        let fail = Arc::clone(&fail);
        let unsolve = Arc::clone(&unsolve);
        let g_count = Arc::clone(&count);
        let handler = thread::spawn(move || {
            let mut game = Game::new();
            let mut solver = Solver::bind(&game).expect("failed to bind solver to game data");
            let total_run = game.answers().len();
            //let total_run = 1;
            let offset = t;
            for i in 0..total_run {
                if i % total_thread == offset {
                    game.set_game_with_answer_index(i);
                    solver.reset();
                    let mut count = 0;

                    loop {
                        count += 1;
                        let (guess, _score) = solver.new_guess(game.round() as u8);

                        let one_match = solver.try_guess(guess, &mut game);

                        if one_match.as_ref().is_some_and(Match::is_correct) {
                            *g_count
                                .lock()
                                .expect("attempt counter mutex should not be poisoned") += count;
                            if count > 6 {
                                let mut fail_handler = fail
                                    .lock()
                                    .expect("failure counter mutex should not be poisoned");
                                *fail_handler += 1;
                            }
                            break;
                        }
                        if count > 20 {
                            {
                                let mut unsolve_handler = unsolve
                                    .lock()
                                    .expect("unsolved counter mutex should not be poisoned");
                                *unsolve_handler += 1;
                            }
                            break;
                        }
                    }
                    let mut sum_handler = sum
                        .lock()
                        .expect("completed-game counter mutex should not be poisoned");
                    *sum_handler += 1;
                }
            }
        });
        handlers.push(handler);
    }
    for h in handlers.into_iter() {
        h.join().expect("solver thread should not panic");
    }
    println!(
        "Total attempts: {:}",
        *sum.lock()
            .expect("completed-game counter mutex should not be poisoned")
    );
    println!(
        "Total failures: {:}",
        *fail
            .lock()
            .expect("failure counter mutex should not be poisoned")
    );
    println!(
        "Total unsolved: {:}",
        *unsolve
            .lock()
            .expect("unsolved counter mutex should not be poisoned")
    );
    println!(
        "Average Trial: {:}",
        *count
            .lock()
            .expect("attempt counter mutex should not be poisoned") as f64
            / *sum
                .lock()
                .expect("completed-game counter mutex should not be poisoned") as f64
    );
}
