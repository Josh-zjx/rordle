use bevy::prelude::*;
use bevy::text::Text2dBounds;
use rand;
use std;
use std::io::prelude::*;
use std::io::stdin;

// Define the size of Box
const BOX_SIZE: Vec2 = Vec2::new(40.0, 40.0);
// Define the color used by box
const GRAY: Color = Color::rgb(0.5, 0.5, 0.5);
const GREEN: Color = Color::rgb(0.25, 0.75, 0.25);
const YELLOW: Color = Color::rgb(0.75, 0.75, 0.25);

pub struct Game {
    answer: String,
    candidate: Vec<String>,
    tried: Vec<String>,
    nonexist: Vec<char>,
}

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
#[derive(Debug)]
pub enum GameError {
    Invalid,
}

impl Game {
    pub fn new_game() -> Game {
        let mut contents = String::new();
        {
            let mut answer_file = std::fs::File::open("./data/answer").unwrap();
            answer_file.read_to_string(&mut contents).unwrap();
        }
        let mut answers: Vec<String> = serde_json::from_str(&contents).unwrap();
        let mut index: usize = rand::random();
        index = index % answers.len();

        let mut condidates = String::new();
        {
            let mut candidate_file = std::fs::File::open("./data/candidate").unwrap();
            candidate_file.read_to_string(&mut condidates).unwrap();
        }
        let mut candidates: Vec<String> = serde_json::from_str(&condidates).unwrap();
        let answer = answers[index].to_string();
        candidates.append(&mut answers);

        return Game {
            answer,
            candidate: candidates,
            tried: Vec::new(),
            nonexist: Vec::new(),
        };
    }

    pub fn guess(&mut self, word: &String) -> Result<Match, GameError> {
        // Check whether input is valid

        let mut one_match = Match {
            states: [
                GuessState::Wrong,
                GuessState::Wrong,
                GuessState::Wrong,
                GuessState::Wrong,
                GuessState::Wrong,
            ],
        };
        if self.candidate.iter().any(|i| i == word) {
            println!("Valid input");
        } else {
            return Err(GameError::Invalid);
        }
        // Correct pass
        for i in 0..5 {
            if word.as_bytes()[i] == self.answer.as_bytes()[i] {
                one_match.states[i] = GuessState::Correct;
            }
        }
        // Wrong pass
        for i in 0..5 {
            if word.as_bytes()[i] != self.answer.as_bytes()[i] {
                for j in 0..5 {
                    if word.as_bytes()[i] == self.answer.as_bytes()[j]
                        && one_match.states[j] != GuessState::Correct
                    {
                        one_match.states[i] = GuessState::Misplace;
                        break;
                    }
                }
            }
        }
        self.tried.push(String::from(word));
        return Ok(one_match);
    }
    pub fn review(&self) -> &String {
        return &self.answer;
    }
}
fn check_input(
    mut submit_query: Query<&mut Submit, With<Submit>>,
    mut guess_query: Query<&mut Guess, With<Guess>>,
    mut board_query: Query<(&mut Sprite, &Row, &Col), (With<Sprite>, With<Row>, With<Col>)>,
    answer: Res<Answer>,
) {
    let mut submit = submit_query.single_mut();
    if submit.state == GameState::ReadyForCheck {
        submit.state = GameState::On;
        let mut guess = guess_query.single_mut();
        let word = &guess.state;
        let mut one_match = Match::new();
        //        if self.candidate.iter().any(|i| i == word) {
        //            println!("Valid input");
        //        } else {
        //            return Err(GameError::Invalid);
        //        }
        // Correct pass
        for i in 0..5 {
            if word.as_bytes()[i] == answer.state.as_bytes()[i] {
                one_match.states[i] = GuessState::Correct;
            }
        }
        // Wrong pass
        for i in 0..5 {
            if word.as_bytes()[i] != answer.state.as_bytes()[i] {
                for j in 0..5 {
                    if word.as_bytes()[i] == answer.state.as_bytes()[j]
                        && one_match.states[j] != GuessState::Correct
                    {
                        one_match.states[i] = GuessState::Misplace;
                        break;
                    }
                }
            }
        }
        for (mut a, b, c) in &mut board_query {
            if b.index == submit.round {
                a.color = match one_match.states[c.index] {
                    GuessState::Wrong => GRAY,
                    GuessState::Correct => GREEN,
                    GuessState::Misplace => YELLOW,
                }
            }
        }
        guess.correctness = one_match;
        submit.round += 1;
        guess.state = String::new();
        if guess.correctness.is_correct() {
            submit.state = GameState::Correct;
            return;
        } else {
            submit.state = GameState::On;
        }
    }
}
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
                a.sections[0].value = (string.as_bytes()[c.index] as char).to_string();
            } else {
                a.sections[0].value = String::from("~");
            }
        }
    }
}
fn check_game_over(
    mut submit_query: Query<&mut Submit, With<Submit>>,
    mut toast_query: Query<&mut Text, (With<Toast>, With<Text>)>,
) {
    let mut submit = submit_query.single_mut();
    if submit.round >= 6 || submit.state == GameState::Correct {
        let mut text = toast_query.single_mut();
        text.sections[0].value = String::from("Game is Over");
        submit.state = GameState::Over;
        return;
    }
}
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

fn _play() -> () {
    let mut game = Game::new_game();
    println!("{:?}", game.review());
    loop {
        let mut user_input = String::new();
        stdin().read_line(&mut user_input).expect("Wrong Format");
        user_input.pop();
        let result = game.guess(&user_input);
        match result {
            Ok(one_match) => {
                println!("{:?}", one_match);
                if one_match.is_correct() {
                    println!("{}", "You guessed the correct answer");
                    return;
                }
                println!("{:} tried", game.tried.len());
                if game.tried.len() > 5 {
                    println!("You used all the turns");
                    println!("{:?}", game.review());
                    return;
                }
            }
            Err(_e) => {
                println!("Invalid Input! Enter again");
            }
        }
    }
}
fn main() -> () {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(Answer {
            state: "tares".to_string(),
        })
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

#[derive(Component)]
struct Submit {
    state: GameState,
    round: usize,
}
#[derive(Component)]
struct Guess {
    state: String,
    correctness: Match,
}
struct Answer {
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

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Camera
    commands.spawn_bundle(Camera2dBundle::default());
    // Initialize game state
    commands.spawn().insert(Submit {
        state: GameState::On,
        round: 0,
    });
    // Initialize Guess State
    commands.spawn().insert(Guess {
        state: String::new(),
        correctness: Match::new(),
    });

    // Set Text Style
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let text_style = TextStyle {
        font,
        font_size: 30.0,
        color: Color::BLACK,
    };

    // Create 6 x 5 wordle array
    for i in 0..6 {
        for j in 0..5 {
            let text_style = text_style.clone();
            let box_position = Vec2::new(60.0 * j as f32, 60.0 * i as f32);
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
                .insert(Row { index: 5 - i })
                .insert(Col { index: j });
            commands
                .spawn_bundle(Text2dBundle {
                    text: Text::from_section("~", text_style),
                    text_2d_bounds: Text2dBounds { size: BOX_SIZE },
                    transform: Transform::from_xyz(
                        box_position.x - BOX_SIZE.x / 2.0,
                        box_position.y + BOX_SIZE.y / 2.0,
                        1.0,
                    ),
                    ..default()
                })
                .insert(Row { index: 5 - i })
                .insert(Col { index: j });
        }
    }

    // Add the game status toast
    commands
        .spawn_bundle(Text2dBundle {
            text: Text::from_section("", text_style.clone()),
            text_2d_bounds: Text2dBounds { size: BOX_SIZE },
            transform: Transform::from_xyz(-50.0, 50.0, 1.0),
            ..default()
        })
        .insert(Toast);
}

#[derive(Component)]
struct Toast;
