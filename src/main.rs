extern crate reqwest;

use clap::{AppSettings, Args, Parser, Subcommand};
use std::fs::File;
use std::io::{self, prelude::*, BufReader};
use std::path::Path;
use std::time::Instant;

const FILENAME: &str = "./words.txt";

/// stores hint information
#[derive(Debug)]
struct Hint {
    letter: char,
    position: usize,
    kind: char,
}

/// global args
/// TODO make non positional
#[derive(Args)]
#[clap(name = "wordle")]
#[clap(about = "wordle solver")]
struct Struct {
    /// number of words to source
    #[clap(short, long, default_value_t = 10000)]
    count: u64,
}

/// CLI struct
#[derive(Parser)]
#[clap(name = "wordle")]
#[clap(about = "wordle solver")]
struct Cli {
    #[clap(subcommand)]
    command: Commands,

    #[clap(flatten)]
    delegate: Struct,
}

/// CLI sub commands
#[derive(Subcommand)]
enum Commands {
    /// try and solve the target word in fewest number of turns
    #[clap(setting(AppSettings::ArgRequiredElseHelp))]
    Solve {
        /// target word to solve for
        #[clap()]
        target: String,
    },

    /// interactively play wordle
    #[clap()]
    Play {},
}

#[tokio::main]
async fn main() {
    if !Path::new(FILENAME).exists() {
        println!("words.txt not found, downloading...");
        let res = download_words().await;
        match res {
            Ok(v) => println!("done: {:?}", v),
            Err(e) => {
                println!("error: {:?}", e);
                return;
            }
        }
    }

    let args = Cli::parse();

    println!("parsing words c={:?}", args.delegate.count);
    let mut words: Vec<String> = Vec::new();
    let res = parse_words(&mut words, args.delegate.count);
    match res {
        Ok(v) => println!("done: {:?}", v),
        Err(e) => {
            println!("error: {:?}", e);
            return;
        }
    }

    match &args.command {
        Commands::Solve { target } => {
            if target.len() != 5 {
                println!("target must be 5 characters in length");
                return;
            }
            println!("attempting to solve with target {:?}", target);
            let start = Instant::now();
            solve(words, target.to_string());
            let end = start.elapsed();
            println!("took {:.2?}", end);
        }
        Commands::Play {} => {
            println!("playing wordle");
            play(words)
        }
    }
}

/// downloads a list of words ordered by how frequently they are used
async fn download_words() -> io::Result<()> {
    let resp = reqwest::get("https://norvig.com/ngrams/count_1w.txt")
        .await
        .expect("request failed");
    let body = resp.text().await.expect("body invalid");
    let mut out = File::create(FILENAME).expect("failed to create file");
    io::copy(&mut body.as_bytes(), &mut out).expect("failed to copy content");
    Ok(())
}

/// reads a word file and parses it into a vector
fn parse_words(words: &mut Vec<String>, count: u64) -> io::Result<()> {
    let file = File::open(FILENAME)?;
    let reader = BufReader::new(file);

    let mut c = count; // number of top words to grab for initial guess
    for line in reader.lines() {
        match line {
            Ok(l) => {
                let mut split: Vec<&str> = l.split('\t').collect();
                split.pop(); // remove freq we dont need
                let word = split.pop().unwrap(); // get actual word
                if word.chars().count() != 5 {
                    continue;
                }
                words.push(word.to_string());
                c = c - 1;
                if c <= 0 {
                    break;
                }
            }
            Err(e) => return Err(e),
        }
    }

    Ok(())
}

/// solves a wordle until it finds the word or gives up
fn solve(words: Vec<String>, target: String) {
    let mut turn = 0u32;
    let mut possible_words = words.clone();
    loop {
        turn += 1;
        println!("turn: {:?}", turn);
        let most_popular = possible_words.get(0).unwrap().to_string();
        println!("guess: {:?}", most_popular);
        let hints = get_hints(&most_popular, &target);
        if is_winner(&hints) {
            println!("word: {:?}, turn: {:?}", most_popular, turn);
            return;
        }
        if turn >= 6 {
            println!("could not find word after 6 turns");
            return;
        }
        possible_words = narrow_guesses(possible_words, hints);
        println!("possible words: {:?}", possible_words.len());
        if possible_words.len() <= 0 {
            println!("word not found, try sourcing more words with --count arg (see --help)");
            return;
        }
    }
}

/// interactively plays wordle with the user
fn play(words: Vec<String>) {
    let mut turn = 0u32;
    let mut possible_words = words.clone();
    println!("enter hints as string where green='g', yellow='y', and black='b' (example: ggybb)");
    loop {
        turn += 1;
        println!("turn: {:?}", turn);
        let guess = possible_words.get(0).unwrap().to_string();
        println!("try: {:?}", guess);
        let mut hint = String::new();
        println!("enter hint string:");
        std::io::stdin().read_line(&mut hint).unwrap();
        hint.pop();
        if hint.len() != 5 {
            println!("invalid hint string");
            turn -= 1;
            continue;
        }
        if hint == "ggggg" {
            println!("we did it!");
            break;
        }
        let mut hints: Vec<Hint> = Vec::new();
        let mut pos = 0;
        for h in hint.chars() {
            hints.push(Hint {
                kind: h,
                position: pos,
                letter: guess.chars().nth(pos).unwrap(),
            });
            pos += 1;
        }
        possible_words = narrow_guesses(possible_words, hints);
        println!("possible words: {:?}", possible_words.len());
        if possible_words.len() <= 0 {
            println!("word not found, try sourcing more words with --count arg (see --help)");
            return;
        }
    }
}

/// narrows down potential guesses based on provided hints
fn narrow_guesses(words: Vec<String>, hints: Vec<Hint>) -> Vec<String> {
    let mut guesses: Vec<String> = Vec::new();
    for word in words {
        let mut is_valid = true;
        for hint in &hints {
            if hint.kind == 'g' && word.chars().nth(hint.position).unwrap() != hint.letter {
                is_valid = false;
                break;
            }
            if hint.kind == 'y'
                && (word.chars().nth(hint.position).unwrap() == hint.letter
                    || !word.contains(hint.letter))
            {
                is_valid = false;
                break;
            }
            if hint.kind == 'b' && word.contains(hint.letter) {
                is_valid = false;
                break;
            }
        }
        if is_valid {
            guesses.push(word)
        }
    }
    return guesses;
}

/// gets a list of hints for the provided guess against the target word
fn get_hints(guess: &String, target: &String) -> Vec<Hint> {
    let mut pos: usize = 0;
    let mut hints: Vec<Hint> = Vec::new();
    for c in guess.chars() {
        let mut hint = 'b';

        if target.contains(c) {
            if target.chars().nth(pos).unwrap() == c {
                hint = 'g'
            } else {
                hint = 'y'
            }
        }

        if !target.contains(c) {
            hint = 'b'
        }

        hints.push(Hint {
            kind: hint,
            letter: c,
            position: pos,
        });
        pos = pos + 1;
    }
    return hints;
}

/// determines if all hints are green
fn is_winner(hints: &Vec<Hint>) -> bool {
    for hint in hints {
        if hint.kind != 'g' {
            return false;
        }
    }
    return true;
}
