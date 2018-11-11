#[macro_use]
extern crate assert_approx_eq;
extern crate clap;
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

use clap::{App, Arg, SubCommand};
use std::collections::HashMap;
use std::time::SystemTime;

use std::fs::OpenOptions;

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

        let card = card::Card {
            uuid: uuid::Uuid::new_v4().to_string(),
            created: SystemTime::now(),
            tags: tags,
            card_contents: card::CardContents::BasicCard(card::BasicCard {
                question: question,
                answer: answer,
            }),
        };

        println!("Card: {:?}", card);
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open("ledger.dat".to_string())?;
        append_to_ledger(ledger::LedgerEntry::NewCard(card), &mut file)?;
    } else if let Some(matches) = matches.subcommand_matches("delete") {
        let uuid = matches.value_of("uuid").unwrap();
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open("ledger.dat".to_string())?;
        append_to_ledger(ledger::LedgerEntry::DeleteCard(uuid.to_string()), &mut file)?;
    } else {
        // Data structures for Question/Answering
        let mut cards = HashMap::new();
        let mut attempts = HashMap::new();

        // TODO: Sort the attempts by timestamp?

        let mut file = OpenOptions::new().read(true).open("ledger.dat")?;
        read_ledger(&mut file, &mut cards, &mut attempts)?;

        loop {
            println!("Cards: {:?}", cards);
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
