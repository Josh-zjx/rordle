pub mod game;
pub mod solver;
use game::*;
use solver::*;
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let sum = Arc::new(Mutex::new(0));
    let fail = Arc::new(Mutex::new(0));
    let count = Arc::new(Mutex::new(0));
    let total_thread = 1;
    let mut handlers = Vec::new();
    for t in 0..total_thread {
        let sum = sum.clone();
        let fail = fail.clone();
        let g_count = count.clone();
        let handler = thread::spawn(move || {
            let mut game = Game::new();
            let mut solver = Solver::bind(&game);
            let total_run = game.answers.len() / 100;
            let offset = t;
            for i in 0..total_run {
                if i % total_thread == offset {
                    game.set_game_with_answer_index(i);
                    solver.reset();
                    let mut count = 0;

                    loop {
                        count += 1;
                        let (guess, score) = solver.new_guess(game.round() as u8);

                        println!("{} {:?} {}", count, guess, score);
                        let one_match = solver.try_guess(guess, &mut game);

                        if one_match.is_some() && one_match.unwrap().is_correct() {
                            *g_count.lock().unwrap() += count;
                            if count > 6 {
                                let mut fail_handler = fail.lock().unwrap();
                                *fail_handler += 1;
                            }
                            break;
                        }
                        if count > 10 {
                            break;
                        }
                    }
                    let mut sum_handler = sum.lock().unwrap();
                    *sum_handler += 1;
                }
            }
        });
        handlers.push(handler);
    }
    for h in handlers.into_iter() {
        h.join().unwrap();
    }
    println!("Total attempts: {:}", *sum.lock().unwrap());
    println!("Total failures: {:}", *fail.lock().unwrap());
    println!(
        "Average Trial: {:}",
        *count.lock().unwrap() as f64 / *sum.lock().unwrap() as f64
    );
}
