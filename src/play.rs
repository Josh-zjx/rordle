pub mod game;
pub mod solver;
use game::*;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
slint::include_modules!();

fn main() {
    use slint::Model;

    let game = Rc::new(Mutex::new(Game::new()));

    let main_window = MainWindow::new().unwrap();
    let main_window_weak = main_window.as_weak().clone();
    let new_data = vec![empty_charblock(); 30];
    let new_data = Rc::new(slint::VecModel::from(new_data));
    main_window_weak
        .unwrap()
        .set_char_items(new_data.clone().into());
    main_window_weak.unwrap().set_level(0);
    main_window_weak.unwrap().set_index(0);

    // Callback functions on handle keyboard input
    let char_items_handler = new_data.clone();
    main_window.on_handle_keyboard(move |text| {
        if &text as &str == "\n" {
            let mut level = main_window_weak.unwrap().get_level() as usize;
            let success = main_window_weak.unwrap().get_success();
            if success {
                return;
            }
            let failed = main_window_weak.unwrap().get_failed();
            if failed {
                return;
            }

            let curr_word = format!(
                "{}{}{}{}{}",
                char_items_handler.row_data(level * 5).unwrap().text,
                char_items_handler.row_data(level * 5 + 1).unwrap().text,
                char_items_handler.row_data(level * 5 + 2).unwrap().text,
                char_items_handler.row_data(level * 5 + 3).unwrap().text,
                char_items_handler.row_data(level * 5 + 4).unwrap().text,
            );
            println!("Trying to submit: {:?}", curr_word);
            let guess = Guess {
                state: curr_word.to_lowercase(),
            };
            if (game.lock().unwrap()).check_valid_guess(&guess) {
                let res = game.lock().unwrap().grade_guess(&guess);
                println!("Match {:?}", res);
                for i in 0..5 {
                    let index = level * 5 + i;
                    let mut new_state = char_items_handler.row_data(index).unwrap();
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
                game.lock().unwrap().progress_game(Arc::new(res));
                if game.lock().unwrap().state == GameState::Correct {
                    main_window_weak.unwrap().set_success(true);
                    println!("Correct Guess!");
                }
                level += 1;
                if level == 6 {
                    main_window_weak.unwrap().set_failed(true);
                    println!("Game Over!");
                }
                main_window_weak.unwrap().set_level(level as i32);
                main_window_weak.unwrap().set_index(0);
            } else {
                println!("Invalid Guess");
                main_window_weak.unwrap().set_invalid(true);
            }
        } else if &text as &str == "\u{8}" {
            let level = main_window_weak.unwrap().get_level();
            let mut index = main_window_weak.unwrap().get_index();
            if index > 0 {
                char_items_handler
                    .set_row_data((level * 5 + index - 1) as usize, empty_charblock());
                index -= 1;
            }
            main_window_weak.unwrap().set_index(index);
            main_window_weak.unwrap().set_invalid(false);
        } else if text.chars().all(char::is_alphabetic) {
            let success = main_window_weak.unwrap().get_success();
            if success {
                return;
            }
            let failed = main_window_weak.unwrap().get_failed();
            if failed {
                return;
            }
            let level = main_window_weak.unwrap().get_level();
            let mut index = main_window_weak.unwrap().get_index();
            if index < 5 {
                char_items_handler.set_row_data(
                    (level * 5 + index) as usize,
                    build_charblock(&text.to_string().to_uppercase()),
                );
                index += 1;
                main_window_weak.unwrap().set_invalid(false);
            }
            main_window_weak.unwrap().set_index(index);
        }
    });
    let main_window_weak = main_window.as_weak().clone();

    // Callback function on reset games
    let char_items_handler = new_data.clone();
    main_window.on_reset(move || {
        println!("reset");
        for i in 0..30 {
            char_items_handler.set_row_data(i, empty_charblock())
        }
        main_window_weak.unwrap().set_level(0);
        main_window_weak.unwrap().set_index(0);
    });

    main_window.run().unwrap();
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
