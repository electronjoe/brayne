#[macro_use]
extern crate assert_approx_eq;
extern crate clap;
extern crate priority_queue;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate termion;
extern crate uuid;

mod card;
mod ledger;
mod supermemo;

use ledger::{append_to_ledger, read_ledger};

use card::{Card, CardContents, UuidString};
use clap::{App, Arg, SubCommand};
use std::collections::HashMap;
use std::time::SystemTime;

use std::fs::OpenOptions;

fn attempt_card(
    uuid: &str,
    cards: &HashMap<UuidString, Card>,
) -> Result<card::AttemptRecord, String> {
    let card = cards
        .get(uuid)
        .ok_or_else(|| "Unable to find card with uuid provided".to_owned())?;
    match card.card_contents {
        CardContents::BasicCard(ref basic_card) => {
            println!(
                "{}Question:{}{}",
                termion::color::Fg(termion::color::Blue),
                termion::color::Fg(termion::color::Reset),
                basic_card.question,
            );
        }
    }
    println!("Hit Enter for Answer");
    let mut recall = "".to_string();
    std::io::stdin()
        .read_line(&mut recall)
        .map_err(|err| err.to_string())?;
    match card.card_contents {
        CardContents::BasicCard(ref basic_card) => {
            println!(
                "{}Answer:{}{}",
                termion::color::Fg(termion::color::Blue),
                termion::color::Fg(termion::color::Reset),
                basic_card.answer,
            );
        }
    }
    println!("Blackout [0] ... Perfect [5]");
    recall.clear();
    std::io::stdin()
        .read_line(&mut recall)
        .map_err(|err| err.to_string())?;
    let recall = recall.trim_right();

    let attempt_quality = match recall {
        "5" => card::AttemptQuality::Perfect,
        "4" => card::AttemptQuality::CorrectAfterHesitation,
        "3" => card::AttemptQuality::CorrectSeriousDifficulty,
        "2" => card::AttemptQuality::IncorrectButEasyRecall,
        "1" => card::AttemptQuality::IncorrectButRemembered,
        "0" => card::AttemptQuality::Blackout,
        _ => {
            return Err("attempt quality must be 0-5".to_owned());
        }
    };

    Ok(card::AttemptRecord {
        uuid: uuid.to_string(),
        time: SystemTime::now(),
        quality: attempt_quality,
    })
}

fn main() -> Result<(), String> {
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

        let card = card::Card {
            uuid: uuid::Uuid::new_v4().to_string(),
            created: SystemTime::now(),
            tags,
            card_contents: card::CardContents::BasicCard(card::BasicCard { question, answer }),
        };

        println!("Card: {:?}", card);
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open("ledger.dat".to_string())
            .map_err(|err| err.to_string())?;
        append_to_ledger(&ledger::LedgerEntry::NewCard(card), &mut file)
            .map_err(|err| err.to_string())?;
    } else if let Some(matches) = matches.subcommand_matches("delete") {
        let uuid = matches.value_of("uuid").unwrap();
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open("ledger.dat".to_string())
            .map_err(|err| err.to_string())?;
        append_to_ledger(
            &ledger::LedgerEntry::DeleteCard(uuid.to_string()),
            &mut file,
        ).map_err(|err| err.to_string())?;
    } else {
        // Data structures for Question/Answering
        let mut cards = HashMap::new();
        let mut supermemo_deck = supermemo::SuperMemoDeck::new();

        let mut file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open("ledger.dat")
            .map_err(|err| err.to_string())?;
        read_ledger(&file, &mut cards, &mut supermemo_deck).map_err(|err| err.to_string())?;

        loop {
            let mut command = "".to_string();
            std::io::stdin()
                .read_line(&mut command)
                .map_err(|err| err.to_string())?;
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
                "r" => continue,
                "c" => {
                    let uuid = match supermemo_deck.draw_card(SystemTime::now()) {
                        None => {
                            println!("There are no cards in the Deck!");
                            continue;
                        }
                        Some(uuid) => uuid,
                    };
                    let attempt_record = attempt_card(&uuid, &cards)?;
                    let ledger_attempt = ledger::LedgerEntry::Attempt(attempt_record.clone());
                    append_to_ledger(&ledger_attempt.clone(), &mut file)
                        .map_err(|err| err.to_string())?;
                    supermemo_deck.insert_attempt(&attempt_record)?;
                }
                _ => (),
            };
        }
    }
    Ok(())
}
