use rand;
use std;
use std::io::prelude::*;
use std::io::stdin;

pub struct Game {
    answer: String,
    candidate: Vec<String>,
    tried: Vec<String>,
    nonexist: Vec<char>,
}

#[derive(PartialEq, Debug)]
enum State {
    Wrong,
    Misplace,
    Correct,
}
#[derive(Debug)]
pub struct Match {
    states: [State; 5],
}
impl Match {
    pub fn is_correct(&self) -> bool {
        if self.states[0] == State::Correct
            && self.states[1] == State::Correct
            && self.states[2] == State::Correct
            && self.states[3] == State::Correct
            && self.states[4] == State::Correct
        {
            true
        } else {
            false
        }
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
                State::Wrong,
                State::Wrong,
                State::Wrong,
                State::Wrong,
                State::Wrong,
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
                one_match.states[i] = State::Correct;
            }
        }
        // Wrong pass
        for i in 0..5 {
            if word.as_bytes()[i] != self.answer.as_bytes()[i] {
                for j in 0..5 {
                    if word.as_bytes()[i] == self.answer.as_bytes()[j]
                        && one_match.states[j] != State::Correct
                    {
                        one_match.states[i] = State::Misplace;
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

fn main() -> () {
    println!("Hello, world!");
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
