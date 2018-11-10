extern crate clap;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate termion;
extern crate uuid;

use clap::{App, Arg, SubCommand};
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::io::BufReader;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Debug)]
enum LedgerEntry {
    NewCard(Card),
    UpdateCard(Card),
    DeleteCard(String),
    Attempt(AttemptRecord),
}

#[derive(Serialize, Deserialize, Debug)]
enum AttemptQuality {
    Perfect = 0,
    CorrectAfterHesitation = 1,
    CorrectSeriousDifficulty = 2,
    IncorrectButEasyRecall = 3,
    IncorrectButRemembered = 4,
    Blackout = 5,
}

#[derive(Serialize, Deserialize, Debug)]
struct AttemptRecord {
    uuid: String,
    // Timestamp of Attempt
    // Represents seconds of UTC time since Unix epoch
    attempt_seconds_utc: u64,
    attempt_quality: AttemptQuality,
}

#[derive(Serialize, Deserialize, Debug)]
enum CardContents {
    BasicCard(BasicCard),
}

#[derive(Serialize, Deserialize, Debug)]
struct BasicCard {
    question: String,
    answer: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Card {
    uuid: String,
    // Timestamp at time of card creation
    // Represents seconds of UTC time since Unix epoch
    creation_seconds_utc: u64,
    // Tags are arbirary user-supplied strings, e.g. "travel"
    tags: Vec<String>,

    card_contents: CardContents,
}

//Result<(), Box<std::error::Error + 'static>>
fn main() -> std::io::Result<()> {
    let matches = App::new("Brayne Local")
        .version("0.1")
        .author("Scott Moeller <electronjoe@gmail.com>")
        .about("Command line Brayne client storing data to local JSON ledger")
        .subcommand(
            SubCommand::with_name("create")
                .about("Creates a new card")
                .arg(
                    Arg::with_name("question")
                        .short("q")
                        .long("question")
                        .help("Card question")
                        .takes_value(true)
                        .required(true),
                ).arg(
                    Arg::with_name("answer")
                        .short("a")
                        .long("answer")
                        .help("Card answer")
                        .takes_value(true)
                        .required(true),
                ).arg(
                    Arg::with_name("tag")
                        .short("t")
                        .long("tag")
                        .help("Card tag")
                        .multiple(true)
                        .takes_value(true)
                        .required(false),
                ),
        ).subcommand(
            SubCommand::with_name("delete")
                .about("Deletes card with specified uuid")
                .arg(
                    Arg::with_name("uuid")
                        .short("u")
                        .long("uuid")
                        .help("Card uuid to delete")
                        .takes_value(true)
                        .required(true),
                ),
        ).get_matches();

    if let Some(matches) = matches.subcommand_matches("create") {
        let question = matches.value_of("question").unwrap().to_string();
        let answer = matches.value_of("answer").unwrap().to_string();
        let tags = if let Some(tags) = matches.values_of("tag") {
            tags.map(|i| i.to_string()).collect()
        } else {
            vec![]
        };

        let utc_seconds = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(utc) => utc,
            Err(err) => panic!("Failed to fetch system time: {}", err),
        };

        let card = Card {
            uuid: uuid::Uuid::new_v4().to_string(),
            creation_seconds_utc: utc_seconds.as_secs(),
            tags: tags,
            card_contents: CardContents::BasicCard(BasicCard {
                question: question,
                answer: answer,
            }),
        };

        println!("Card: {:?}", card);

        let serialized = serde_json::to_string(&card).unwrap();
        println!("Serialized: {}", serialized);

        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open("ledger.dat")?;
        file.write(serialized.as_bytes())?;
        file.write(b"\n")?;
        file.flush()?;
    } else {
        let mut file = OpenOptions::new().read(true).open("ledger.dat")?;
        for (num, line) in BufReader::new(file).lines().enumerate() {
            let l = line?;
            let card: Card = serde_json::from_str(&l)?;
            println!("Card {}: {:?}", num, card);
        }

        loop {
            let mut command = "".to_string();
            std::io::stdin().read_line(&mut command)?;
            command = command.to_lowercase();
            let command = command.trim_right();
            println!(
                "{}Got command: {}{}",
                termion::color::Fg(termion::color::Red),
                command,
                termion::color::Fg(termion::color::Reset)
            );
            match command {
                "q" => break,
                _ => (),
            };
        }
    }
    Ok(())
}
