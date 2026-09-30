use crate::game::*;
use crate::solver::Solver;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

slint::include_modules!();

fn build_charblock(text: &str) -> CharItem {
    CharItem {
        text: text.into(),
        trial: true,
        correct: false,
        misplaced: false,
    }
}

fn empty_charblock() -> CharItem {
    build_charblock("")
}

/// Search off the UI thread; a poor opening can make the search take seconds.
#[cfg(not(target_arch = "wasm32"))]
fn find_hint(solver: Solver, round: usize, deliver: impl FnOnce(String) + Send + 'static) {
    std::thread::spawn(move || deliver(solver.suggest(round)));
}

/// Browsers have no threads here, so search synchronously.
#[cfg(target_arch = "wasm32")]
fn find_hint(solver: Solver, round: usize, deliver: impl FnOnce(String) + Send + 'static) {
    deliver(solver.suggest(round));
}

/// Replace the current input row with `word`.
fn fill_current_row(window: &MainWindow, word: &str) {
    use slint::Model;

    let level = window.get_level() as usize;
    let char_items = window.get_char_items();
    for (i, letter) in word.chars().take(5).enumerate() {
        char_items.set_row_data(
            level * 5 + i,
            build_charblock(&letter.to_ascii_uppercase().to_string()),
        );
    }
    window.set_index(5);
    window.set_invalid(false);
}

pub fn run() -> Result<(), slint::PlatformError> {
    use slint::Model;

    let game = Rc::new(RefCell::new(Game::new()));
    let solver = Rc::new(RefCell::new(Solver::bind(&game.borrow())));
    // Bumped by every hint request, submitted guess, and reset, so a hint that
    // finishes after the board has moved on is discarded.
    let hint_request = Arc::new(AtomicU64::new(0));

    let main_window = MainWindow::new()?;
    let main_window_weak = main_window.as_weak();
    let new_data = Rc::new(slint::VecModel::from(vec![empty_charblock(); 30]));
    main_window.set_char_items(Rc::clone(&new_data).into());
    main_window.set_level(0);
    main_window.set_index(0);

    let char_items_handler = Rc::clone(&new_data);
    let game_handle = Rc::clone(&game);
    let solver_handle = Rc::clone(&solver);
    let hint_handle = Arc::clone(&hint_request);
    main_window.on_handle_keyboard(move |text| {
        let Some(window) = main_window_weak.upgrade() else {
            return;
        };

        if text.as_str() == "\n" {
            let mut level = window.get_level() as usize;
            if window.get_success() || window.get_failed() {
                return;
            }

            let mut curr_word = String::with_capacity(5);
            for i in 0..5 {
                let Some(cell) = char_items_handler.row_data(level * 5 + i) else {
                    return;
                };
                curr_word.push_str(&cell.text);
            }

            let guess = curr_word.to_lowercase();
            let mut game = game_handle.borrow_mut();
            if game.check_valid_guess(&guess) {
                let Ok(res) = game.grade_guess(&guess) else {
                    window.set_invalid(true);
                    return;
                };
                let Ok(chars) = guess.as_bytes().try_into() else {
                    window.set_invalid(true);
                    return;
                };
                if solver_handle.borrow_mut().add_pattern(chars, &res).is_err() {
                    window.set_invalid(true);
                    return;
                }
                hint_handle.fetch_add(1, Ordering::Relaxed);
                window.set_hint_pending(false);

                for i in 0..5 {
                    let index = level * 5 + i;
                    let Some(mut new_state) = char_items_handler.row_data(index) else {
                        return;
                    };
                    new_state.trial = false;
                    match res.states[i] {
                        GuessState::Wrong => {
                            new_state.correct = false;
                            new_state.misplaced = false;
                        }
                        GuessState::Correct => {
                            new_state.correct = true;
                            new_state.misplaced = false;
                        }
                        GuessState::Misplace => {
                            new_state.correct = false;
                            new_state.misplaced = true;
                        }
                    }
                    char_items_handler.set_row_data(index, new_state);
                }
                game.progress_game(&res);
                if game.state == GameState::Correct {
                    window.set_success(true);
                } else if level == 5 {
                    window.set_failed(true);
                    window.set_answer_text(game.answer().to_uppercase().into());
                }
                level += 1;
                window.set_level(level as i32);
                window.set_index(0);
            } else {
                window.set_invalid(true);
            }
        } else if text.as_str() == "\u{8}" {
            let level = window.get_level();
            let mut index = window.get_index();
            if index > 0 {
                char_items_handler
                    .set_row_data((level * 5 + index - 1) as usize, empty_charblock());
                index -= 1;
            }
            window.set_index(index);
            window.set_invalid(false);
        } else if text.len() == 1 && text.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
            if window.get_success() || window.get_failed() {
                return;
            }
            let level = window.get_level();
            let mut index = window.get_index();
            if index < 5 {
                char_items_handler.set_row_data(
                    (level * 5 + index) as usize,
                    build_charblock(&text.to_string().to_uppercase()),
                );
                index += 1;
                window.set_invalid(false);
            }
            window.set_index(index);
        }
    });

    let main_window_weak = main_window.as_weak();
    let game_handle = Rc::clone(&game);
    let solver_handle = Rc::clone(&solver);
    let hint_handle = Arc::clone(&hint_request);
    main_window.on_hint(move || {
        let Some(window) = main_window_weak.upgrade() else {
            return;
        };
        if window.get_success() || window.get_failed() || window.get_hint_pending() {
            return;
        }

        let request = hint_handle.fetch_add(1, Ordering::Relaxed) + 1;
        let latest_request = Arc::clone(&hint_handle);
        let window_weak = window.as_weak();
        window.set_hint_pending(true);
        find_hint(
            solver_handle.borrow().clone(),
            game_handle.borrow().round(),
            move |word| {
                let _ = window_weak.upgrade_in_event_loop(move |window| {
                    if latest_request.load(Ordering::Relaxed) != request {
                        return;
                    }
                    window.set_hint_pending(false);
                    fill_current_row(&window, &word);
                });
            },
        );
    });

    let main_window_weak = main_window.as_weak();
    let char_items_handler = new_data;
    let game_handle = game;
    main_window.on_reset(move || {
        for i in 0..30 {
            char_items_handler.set_row_data(i, empty_charblock());
        }
        let Some(window) = main_window_weak.upgrade() else {
            return;
        };
        window.set_level(0);
        window.set_index(0);
        window.set_success(false);
        window.set_failed(false);
        window.set_invalid(false);
        window.set_answer_text("".into());
        window.set_hint_pending(false);
        hint_request.fetch_add(1, Ordering::Relaxed);
        game_handle.borrow_mut().reset();
        solver.borrow_mut().reset();
    });

    main_window.run()
}
