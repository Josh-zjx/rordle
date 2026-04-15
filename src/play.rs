pub mod game;
pub mod solver;
use game::*;
use std::rc::Rc;
use std::sync::Mutex;
slint::include_modules!();

fn main() {
    use slint::Model;

    let game = Rc::new(Mutex::new(Game::new()));

    let main_window = MainWindow::new().expect("failed to create main window");
    let main_window_weak = main_window.as_weak();
    let new_data = vec![empty_charblock(); 30];
    let new_data = Rc::new(slint::VecModel::from(new_data));
    main_window.set_char_items(Rc::clone(&new_data).into());
    main_window.set_level(0);
    main_window.set_index(0);

    // Callback functions on handle keyboard input
    let char_items_handler = Rc::clone(&new_data);
    main_window.on_handle_keyboard(move |text| {
        let Some(window) = main_window_weak.upgrade() else {
            return;
        };

        if &text as &str == "\n" {
            let mut level = window.get_level() as usize;
            let success = window.get_success();
            if success {
                return;
            }
            let failed = window.get_failed();
            if failed {
                return;
            }

            let Some(first) = char_items_handler.row_data(level * 5) else {
                return;
            };
            let Some(second) = char_items_handler.row_data(level * 5 + 1) else {
                return;
            };
            let Some(third) = char_items_handler.row_data(level * 5 + 2) else {
                return;
            };
            let Some(fourth) = char_items_handler.row_data(level * 5 + 3) else {
                return;
            };
            let Some(fifth) = char_items_handler.row_data(level * 5 + 4) else {
                return;
            };
            let curr_word = format!(
                "{}{}{}{}{}",
                first.text, second.text, third.text, fourth.text, fifth.text,
            );

            println!("Trying to submit: {:?}", curr_word);

            let guess = curr_word.to_lowercase();
            let mut game = game.lock().expect("game mutex should not be poisoned");
            if game.check_valid_guess(&guess) {
                let res = game.grade_guess(&guess);

                #[cfg(debug_assertions)]
                println!("Match {:?}", res);

                for i in 0..5 {
                    let index = level * 5 + i;
                    let Some(mut new_state) = char_items_handler.row_data(index) else {
                        return;
                    };
                    new_state.trial = false;
                    new_state.nonexist = false;
                    new_state.correct = false;
                    new_state.misplaced = false;
                    match res.states[i] {
                        GuessState::Wrong => {
                            new_state.nonexist = true;
                            char_items_handler.set_row_data(index, new_state);
                        }
                        GuessState::Correct => {
                            new_state.correct = true;
                            char_items_handler.set_row_data(index, new_state);
                        }
                        GuessState::Misplace => {
                            new_state.misplaced = true;
                            char_items_handler.set_row_data(index, new_state);
                        }
                    }
                }
                game.progress_game(&res);
                if game.state == GameState::Correct {
                    window.set_success(true);
                    println!("Correct Guess!");
                }
                level += 1;
                if level == 6 {
                    window.set_failed(true);
                    println!("Game Over!");
                }
                window.set_level(level as i32);
                window.set_index(0);
            } else {
                println!("Invalid Guess");
                window.set_invalid(true);
            }
        } else if &text as &str == "\u{8}" {
            let level = window.get_level();
            let mut index = window.get_index();
            if index > 0 {
                char_items_handler
                    .set_row_data((level * 5 + index - 1) as usize, empty_charblock());
                index -= 1;
            }
            window.set_index(index);
            window.set_invalid(false);
        } else if text.chars().all(char::is_alphabetic) {
            let success = window.get_success();
            if success {
                return;
            }
            let failed = window.get_failed();
            if failed {
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

    // Callback function on reset games
    let char_items_handler = new_data;
    main_window.on_reset(move || {
        println!("reset");
        for i in 0..30 {
            char_items_handler.set_row_data(i, empty_charblock())
        }
        let Some(window) = main_window_weak.upgrade() else {
            return;
        };
        window.set_level(0);
        window.set_index(0);
    });

    main_window.run().expect("failed to run main window");
}

fn build_charblock(text: &str) -> CharItem {
    //println!("New charblock: {:?}", text);
    CharItem {
        text: text.into(),
        trial: true,
        correct: true,
        misplaced: true,
        nonexist: true,
    }
}
fn empty_charblock() -> CharItem {
    build_charblock("")
}
