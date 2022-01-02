extern crate reqwest;

use std::path::Path;
use std::fs::File;
use std::io::{self, prelude::*, BufReader};

const FILENAME: &str = "./words.txt";

#[tokio::main]
async fn main() {
    if !Path::new(FILENAME).exists() {
        println!("words.txt not found, downloading...");
        let res = download_words().await;
        match res {
            Ok(v) => println!("done: {:?}", v),
            Err(e) => {
                println!("error: {:?}", e);
                return
            },
        }
    }

    println!("parsing words");
    let mut words: Vec<String> = Vec::new();
    let res = parse_words(&mut words);
    match res {
        Ok(v) => println!("done: {:?}", v),
        Err(e) => {
            println!("error: {:?}", e);
            return
        },
    }
    println!("{:?}", words)
}

/// downloads a list of words ordered by how frequently they are used
async fn download_words() -> io::Result<()> {
    let resp = reqwest::get("https://norvig.com/ngrams/count_1w.txt").await.expect("request failed");
    let body = resp.text().await.expect("body invalid");
    let mut out = File::create(FILENAME).expect("failed to create file");
    io::copy(&mut body.as_bytes(), &mut out).expect("failed to copy content");
    Ok(())
}

fn parse_words(words: &mut Vec<String>) -> io::Result<()> {
    let file = File::open(FILENAME)?;
    let reader = BufReader::new(file);

    let mut c = 1000; // number of top words to grab for initial guess
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
            },
            Err(e) => {
                return Err(e)
            },
        }
    }

    Ok(())
}