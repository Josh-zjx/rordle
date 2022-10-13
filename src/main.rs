use bevy::text::Text2dBounds;
use bevy::{prelude::*, window::PresentMode};
use rand;
use std;
use std::collections::hash_map;
use std::io::prelude::*;

// Define the size of Box
const BOX_SIZE: Vec2 = Vec2::new(40.0, 40.0);
// Define the color used by box
const GRAY: Color = Color::rgb(0.5, 0.5, 0.5);
const GREEN: Color = Color::rgb(0.25, 0.75, 0.25);
const YELLOW: Color = Color::rgb(0.75, 0.75, 0.25);
//const RED: Color = Color::rgb(0.75, 0.25, 0.25);

#[derive(PartialEq, Debug)]
enum GuessState {
    Wrong,
    Misplace,
    Correct,
}
#[derive(PartialEq, Debug)]
enum GameState {
    On,
    ReadyForCheck,
    Correct,
    Over,
}
#[derive(Debug)]
pub struct Match {
    states: [GuessState; 5],
}
impl Match {
    pub fn is_correct(&self) -> bool {
        if self.states[0] == GuessState::Correct
            && self.states[1] == GuessState::Correct
            && self.states[2] == GuessState::Correct
            && self.states[3] == GuessState::Correct
            && self.states[4] == GuessState::Correct
        {
            true
        } else {
            false
        }
    }
    pub fn new() -> Match {
        return Match {
            states: [
                GuessState::Wrong,
                GuessState::Wrong,
                GuessState::Wrong,
                GuessState::Wrong,
                GuessState::Wrong,
            ],
        };
    }
}

fn check_input(
    mut submit_query: Query<&mut Submit, With<Submit>>,
    mut guess_query: Query<&mut Guess, With<Guess>>,
    mut board_query: Query<(&mut Sprite, &Row, &Col), (With<Sprite>, With<Row>, With<Col>)>,
    answer: Res<Answer>,
    candidates: Res<Candidates>,
    mut toast: Query<&mut Text, (With<Toast>, With<Text>)>,
    mut status: Query<&mut Text, (Without<Toast>, With<StatusBoard>)>,
) {
    let mut submit = submit_query.single_mut();

    // Only execute when user submit a full guess
    if submit.state == GameState::ReadyForCheck {
        submit.state = GameState::On;
        let mut guess = guess_query.single_mut();
        let word = &guess.state;

        // Filter and Reject non-valid guess
        if candidates.state.iter().any(|i| i == word) {
            toast.single_mut().sections[0].value = String::from("Valid Try!");
        } else {
            toast.single_mut().sections[0].value = String::from("Not in Dict");
            return;
        }

        let mut one_match = Match::new();
        // Correct pass
        for i in 0..5 {
            if word.as_bytes()[i] == answer.state.as_bytes()[i] {
                one_match.states[i] = GuessState::Correct;
            }
        }
        // Wrong pass
        for i in 0..5 {
            if word.as_bytes()[i] != answer.state.as_bytes()[i] {
                let mut exist = false;
                for j in 0..5 {
                    if word.as_bytes()[i] == answer.state.as_bytes()[j] {
                        exist = true;
                        if one_match.states[j] != GuessState::Correct {
                            one_match.states[i] = GuessState::Misplace;
                            break;
                        }
                    }
                }
                if !exist && !submit.wrong.contains_key(&(*&word.as_bytes()[i] as char)) {
                    submit
                        .wrong
                        .insert(word.as_bytes()[i] as char, GuessState::Wrong);
                }
            }
        }

        // Color the guess as output
        for (mut a, b, c) in &mut board_query {
            if b.index == submit.round {
                a.color = match one_match.states[c.index] {
                    GuessState::Wrong => GRAY,
                    GuessState::Correct => GREEN,
                    GuessState::Misplace => YELLOW,
                }
            }
        }

        // Update Finalized word
        let mut current_status = status.single_mut();
        let old_status = current_status.sections[0].value.clone();
        let mut new_string = String::new();
        for i in 0..5 {
            if old_status.as_bytes()[i] == '*' as u8 {
                if one_match.states[i] == GuessState::Correct {
                    new_string.push(word.as_bytes()[i] as char);
                } else {
                    new_string.push('*');
                }
            } else {
                new_string.push(old_status.as_bytes()[i] as char);
            }
        }
        current_status.sections[0].value = new_string;
        // Check correctness
        if one_match.is_correct() {
            submit.state = GameState::Correct;
            return;
        } else {
            submit.state = GameState::On;
        }
        // Update round statistics
        submit.round += 1;

        // Clean guess
        guess.state = String::new();
    }
}
// Update status and input to UI
fn update_board(
    query: Query<&Guess, With<Guess>>,
    submit_query: Query<&Submit, With<Submit>>,
    mut board_query: Query<(&mut Text, &Row, &Col), (With<Text>, With<Row>, With<Col>)>,
) {
    let submit = submit_query.single();
    let string = &query.single().state;
    for (mut a, b, c) in &mut board_query {
        if b.index == submit.round {
            if c.index < string.chars().count() {
                let letter = string.as_bytes()[c.index].clone() as char;
                let display_text = match submit.wrong.get(&letter) {
                    Some(state) => {
                        if *state == GuessState::Wrong {
                            let mut temp = String::from("( ");
                            temp.push(letter);
                            temp.push_str(" )");
                            temp
                        } else {
                            letter.to_string()
                        }
                    }
                    None => letter.to_string(),
                };
                a.sections[0].value = display_text;
            } else {
                a.sections[0].value = String::from("  ");
            }
        }
    }
}
// Check whether the game is over and input should be freezed
fn check_game_over(
    mut submit_query: Query<&mut Submit, With<Submit>>,
    mut toast_query: Query<&mut Text, (With<Toast>, With<Text>)>,
    mut status: Query<&mut Text, (Without<Toast>, With<StatusBoard>)>,
    answer: Res<Answer>,
) {
    let mut submit = submit_query.single_mut();
    if submit.round >= 6 || submit.state == GameState::Correct {
        let mut text = toast_query.single_mut();
        text.sections[0].value = String::from("Game is Over");
        status.single_mut().sections[0].value = answer.state.clone();
        submit.state = GameState::Over;
        return;
    }
}
// Capture Keyboard Input
// letters are captured as keystroke because
//      - Wordle is a game with mechanism based on individual A-Z letters
//      - only characters (regardless of capitalization) are valid input
fn key_input(
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<&mut Guess, With<Guess>>,
    mut submit_query: Query<&mut Submit, With<Submit>>,
) {
    let mut submit = submit_query.single_mut();
    if submit.state == GameState::Over {
        return;
    }
    let mut guess_content = query.single_mut();
    let current_length = guess_content.state.chars().count();

    // Only accept full guess
    if keyboard_input.just_pressed(KeyCode::Return) {
        if current_length == 5 {
            submit.state = GameState::ReadyForCheck;
            return;
        }
    } else if keyboard_input.just_pressed(KeyCode::Back) {
        if current_length > 0 {
            guess_content.state.remove(current_length - 1);
            return;
        }
    } else {
        if current_length == 5 {
            return;
        } else if keyboard_input.just_pressed(KeyCode::A) {
            guess_content.state.push('a');
            return;
        } else if keyboard_input.just_pressed(KeyCode::B) {
            guess_content.state.push('b');
            return;
        } else if keyboard_input.just_pressed(KeyCode::C) {
            guess_content.state.push('c');
            return;
        } else if keyboard_input.just_pressed(KeyCode::D) {
            guess_content.state.push('d');
            return;
        } else if keyboard_input.just_pressed(KeyCode::E) {
            guess_content.state.push('e');
            return;
        } else if keyboard_input.just_pressed(KeyCode::F) {
            guess_content.state.push('f');
            return;
        } else if keyboard_input.just_pressed(KeyCode::G) {
            guess_content.state.push('g');
            return;
        } else if keyboard_input.just_pressed(KeyCode::H) {
            guess_content.state.push('h');
            return;
        } else if keyboard_input.just_pressed(KeyCode::I) {
            guess_content.state.push('i');
            return;
        } else if keyboard_input.just_pressed(KeyCode::J) {
            guess_content.state.push('j');
            return;
        } else if keyboard_input.just_pressed(KeyCode::K) {
            guess_content.state.push('k');
            return;
        } else if keyboard_input.just_pressed(KeyCode::L) {
            guess_content.state.push('l');
            return;
        } else if keyboard_input.just_pressed(KeyCode::M) {
            guess_content.state.push('m');
            return;
        } else if keyboard_input.just_pressed(KeyCode::N) {
            guess_content.state.push('n');
            return;
        } else if keyboard_input.just_pressed(KeyCode::O) {
            guess_content.state.push('o');
            return;
        } else if keyboard_input.just_pressed(KeyCode::P) {
            guess_content.state.push('p');
            return;
        } else if keyboard_input.just_pressed(KeyCode::Q) {
            guess_content.state.push('q');
            return;
        } else if keyboard_input.just_pressed(KeyCode::R) {
            guess_content.state.push('r');
            return;
        } else if keyboard_input.just_pressed(KeyCode::S) {
            guess_content.state.push('s');
            return;
        } else if keyboard_input.just_pressed(KeyCode::T) {
            guess_content.state.push('t');
            return;
        } else if keyboard_input.just_pressed(KeyCode::U) {
            guess_content.state.push('u');
            return;
        } else if keyboard_input.just_pressed(KeyCode::V) {
            guess_content.state.push('v');
            return;
        } else if keyboard_input.just_pressed(KeyCode::W) {
            guess_content.state.push('w');
            return;
        } else if keyboard_input.just_pressed(KeyCode::X) {
            guess_content.state.push('x');
            return;
        } else if keyboard_input.just_pressed(KeyCode::Y) {
            guess_content.state.push('y');
            return;
        } else if keyboard_input.just_pressed(KeyCode::Z) {
            guess_content.state.push('z');
            return;
        } else {
        }
    }
}

/// Main Entry
fn main() -> () {
    App::new()
        .insert_resource(WindowDescriptor {
            title: "Rordle".to_string(),
            width: 600.,
            height: 600.,
            present_mode: PresentMode::AutoVsync,
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .insert_resource(Answer {
            state: String::new(),
        })
        .insert_resource(Candidates { state: Vec::new() })
        .add_startup_system(setup)
        .add_system_set(
            SystemSet::new()
                .with_system(key_input)
                .with_system(check_input.after(key_input))
                .with_system(update_board.after(key_input))
                .with_system(check_game_over.after(check_input)),
        )
        .run();
}

/// Initialize Game State, UI components and Resources
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut answer: ResMut<Answer>,
    mut candidates: ResMut<Candidates>,
) {
    // Read data to build answer set and candidate set
    let mut answer_strings = String::new();
    {
        let mut answer_file = std::fs::File::open("./data/answer").unwrap();
        answer_file.read_to_string(&mut answer_strings).unwrap();
    }
    let mut answers: Vec<String> = serde_json::from_str(&answer_strings).unwrap();

    let mut index: usize = rand::random();
    index = index % answers.len();

    let mut candidate_strings = String::new();
    {
        let mut candidate_file = std::fs::File::open("./data/candidate").unwrap();
        candidate_file
            .read_to_string(&mut candidate_strings)
            .unwrap();
    }

    let mut candidate_vec: Vec<String> = serde_json::from_str(&candidate_strings).unwrap();
    answer.state = answers[index].to_string();
    candidate_vec.append(&mut answers);
    candidates.state = candidate_vec;

    // Camera
    commands.spawn_bundle(Camera2dBundle::default());
    // Initialize game state
    commands.spawn().insert(Submit {
        state: GameState::On,
        round: 0,
        wrong: hash_map::HashMap::new(),
    });
    // Initialize Guess State
    commands.spawn().insert(Guess {
        state: String::new(),
    });

    // Set Text Style
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let text_style = TextStyle {
        font,
        font_size: 30.0,
        color: Color::BLACK,
    };

    let bias = -150.0;
    // Create 6 x 5 wordle array
    for i in 0..6 {
        for j in 0..5 {
            let text_style = text_style.clone();
            let box_position = Vec2::new(60.0 * j as f32, 60.0 * i as f32 + bias);
            commands
                .spawn_bundle(SpriteBundle {
                    sprite: Sprite {
                        color: GRAY,
                        custom_size: Some(BOX_SIZE),
                        ..default()
                    },
                    transform: Transform::from_translation(box_position.extend(0.0)),
                    ..default()
                })
                // Adding row and col member for access
                .insert(Row { index: 5 - i })
                .insert(Col { index: j });
            commands
                .spawn_bundle(Text2dBundle {
                    text: Text::from_section("  ", text_style),
                    text_2d_bounds: Text2dBounds { size: BOX_SIZE },
                    transform: Transform::from_xyz(
                        box_position.x - BOX_SIZE.x / 2.0,
                        box_position.y + BOX_SIZE.y / 2.0,
                        1.0,
                    ),
                    ..default()
                })
                // Adding row and col member for access
                .insert(Row { index: 5 - i })
                .insert(Col { index: j });
        }
    }

    // Add the game status toast
    commands
        .spawn_bundle(Text2dBundle {
            text: Text::from_section("", text_style.clone()),
            text_2d_bounds: Text2dBounds {
                size: BOX_SIZE * 2.0,
            },
            transform: Transform::from_xyz(-150.0, 50.0, 1.0),
            ..default()
        })
        .insert(Toast);

    // Add the game state board
    commands
        .spawn_bundle(Text2dBundle {
            text: Text::from_section("*****", text_style.clone()),
            text_2d_bounds: Text2dBounds {
                size: BOX_SIZE * 2.0,
            },
            transform: Transform::from_xyz(-150.0, 150.0, 1.0),
            ..default()
        })
        .insert(StatusBoard);
}

#[derive(Component)]
struct Toast;

#[derive(Component)]
struct StatusBoard;

#[derive(Component)]
struct Submit {
    state: GameState,
    round: usize,
    wrong: hash_map::HashMap<char, GuessState>,
}

#[derive(Component)]
struct Guess {
    state: String,
}

#[derive(Component)]
struct Col {
    index: usize,
}

#[derive(Component)]
struct Row {
    index: usize,
}

struct Answer {
    state: String,
}

struct Candidates {
    state: Vec<String>,
}
