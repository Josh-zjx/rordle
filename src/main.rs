pub mod game;
pub mod solver;
use game::*;
use solver::*;
use std::str::Chars;
use std::sync::{Arc, Mutex};
use std::thread;

fn main() -> () {
    use slint::Model;

    let mut game = std::rc::Rc::new(std::sync::Mutex::new(Game::new()));

    let main_window = MainWindow::new().unwrap();
    let main_window_weak = main_window.as_weak().clone();
    let new_data = vec![empty_charblock(); 30];
    let new_data = std::rc::Rc::new(slint::VecModel::from(new_data));
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

            let curr_word = format!(
                "{}{}{}{}{}",
                char_items_handler.row_data(level * 5).unwrap().text,
                char_items_handler.row_data(level * 5 + 1).unwrap().text,
                char_items_handler.row_data(level * 5 + 2).unwrap().text,
                char_items_handler.row_data(level * 5 + 3).unwrap().text,
                char_items_handler.row_data(level * 5 + 4).unwrap().text,
            );
            println!("Tring to submit: {:?}", curr_word);
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
                    println!("New state: {:?}", new_state);
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
                level += 1;
                main_window_weak.unwrap().set_level(level as i32);
                main_window_weak.unwrap().set_index(0);
            } else {
                println!("Invalid Guess");
                main_window_weak.unwrap().set_invalid(true);
            }

            println!("Key Event Enter");
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
            println!("Key Event Backspace");
        } else if text.chars().all(char::is_alphabetic) {
            println!("Key Event Input Got {:?}", text.to_string().to_uppercase());
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
        } else {
            println!("Non supported char");
        }
    });
    //let char_items: Vec<CharItem> = main_window.get_char_items().iter().collect();
    let main_window_weak = main_window.as_weak().clone();
    //
    //Initialize

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
    println!("New charblock: {:?}", text);
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

slint::slint! {
    struct CharItem {
        text: string,
        trial:bool,
        correct:bool,
        misplaced:bool,
        nonexist:bool,
    }
    component CharBlock inherits Rectangle{

        width: 60px;
        height: 60px;
        in property <string> show_char;
        in property <bool> trial;
        in property <bool> correct;
        in property <bool> misplaced;

        Rectangle{
            background: trial?#FFFFFF:(correct?#00FF00:(misplaced?#FFFF00:#808080));
        }
        Text {
            text: show_char;
            font-size: 35px;
        }

    }
export component MainWindow inherits Window {
        width: 360px;
        height: 500px;
        background: #000000;


        in property <int> level;
        in property <int> index;
        in property <bool> invalid:false;
        callback handle_keyboard(string);
        callback reset();
        FocusScope {
            key-pressed(event) => {
               // handle_keyboard(event)
                root.handle_keyboard(event.text);
                accept

            }
        }
        in property <int> curr_level:0;
        in property <[CharItem]> char_items:[
        ];


        Rectangle {
            x: 180px;
            y:450px;
            width:50px;
            height:50px;
            background:#ffffff;
            Text {
                text:root.invalid?"Invalid input":"";

            }
        }

        for tile[i] in char_items: CharBlock {
            x: mod(i,5) * 70px;
            y: floor(i / 5) * 70px;
            show_char:tile.text;
            trial:tile.trial;
            correct:tile.correct;
            misplaced:tile.misplaced;
        }


}}
/*
fn main() -> () {
    let sum = Arc::new(Mutex::new(0));
    let fail = Arc::new(Mutex::new(0));
    //let mut game = Game::new();
    //let mut solver = Solver::bind(&game);
    let total_thread = 10;
    let mut handlers = Vec::new();
    for t in 0..total_thread {
        let sum = Arc::clone(&sum);
        let fail = Arc::clone(&fail);
        let handler = thread::spawn(move || {
            let mut game = Game::new();
            let mut solver = Solver::bind(&game);
            let total_run = game.answers.len();
            let offset = t;
            for i in 0..total_run {
                if i % total_thread == offset {
                    game.set_game_with_answer_index(i);
                    solver.reset();
                    let mut count = 0;

                    loop {
                        count += 1;
                        let guess = solver.new_guess(game.round() as u8);
                        let one_match = solver.try_guess(guess, &mut game);
                        match one_match {
                            Some(one) => {
                                if one.is_correct() {
                                    if count > 6 {
                                        let mut fail_handler = fail.lock().unwrap();
                                        *fail_handler += 1;
                                    }
                                    break;
                                }
                            }
                            None => (),
                        }
                    }
                    //println!("{:}", count);
                    let mut sum_handler = sum.lock().unwrap();
                    *sum_handler += count;
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
    //println!("Average attempts: {:}", sum as f64 / total_run as f64);
}
*/
