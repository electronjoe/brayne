use card::{AttemptRecord, Card, UuidString};
use std::collections::HashMap;
use std::io::prelude::*;
use std::io::BufReader;
use supermemo::SuperMemoDeck;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum LedgerEntry {
    NewCard(Card),
    UpdateTags(UuidString, Vec<String>),
    DeleteCard(UuidString),
    Attempt(AttemptRecord),
}

pub fn append_to_ledger(
    update: &LedgerEntry,
    file: &mut std::fs::File,
) -> Result<(), std::io::Error> {
    let serialized = serde_json::to_string(update).unwrap();
    println!("Serialized: {}", serialized);

    file.write_all(serialized.as_bytes())?;
    file.write_all(b"\n")?;
    file.flush()?;
    Ok(())
}

pub fn update_from_ledger(
    ledger_entry: LedgerEntry,
    cards: &mut HashMap<UuidString, Card>,
    supermemo_deck: &mut SuperMemoDeck,
) -> Result<(), String> {
    match ledger_entry {
        LedgerEntry::NewCard(new_card) => {
            cards.insert(new_card.uuid.clone(), new_card.clone());
            supermemo_deck.new_card(new_card.uuid.clone(), new_card.created.clone());
            Ok(())
        }
        LedgerEntry::DeleteCard(uuid) => {
            // TODO throw a warning if uuid doesn't exist?
            cards.remove(&uuid);
            supermemo_deck.delete_card(&uuid);
            Ok(())
        }
        LedgerEntry::Attempt(attempt) => {
            supermemo_deck.insert_attempt(&attempt)?;
            Ok(())
        }
        _ => Ok(()),
    }
}

pub fn read_ledger(
    file: &std::fs::File,
    cards: &mut HashMap<UuidString, Card>,
    supermemo_deck: &mut SuperMemoDeck,
) -> Result<(), std::io::Error> {
    for (num, line) in BufReader::new(file).lines().enumerate() {
        let l = line?;
        let update: LedgerEntry = serde_json::from_str(&l)?;
        println!("LedgerEntry {}: {:?}", num, update);
        update_from_ledger(update, cards, supermemo_deck).unwrap();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    extern crate tempfile;

    use self::tempfile::tempfile;
    use super::*;
    use card::{BasicCard, CardContents};
    use std::time::SystemTime;

    #[test]
    fn test_ledger_write_read() {
        let new_card = Card {
            uuid: "banana-farm".to_string(),
            created: SystemTime::now(),
            tags: vec!["hippo".to_string(), "family".to_string()],
            card_contents: CardContents::BasicCard(BasicCard {
                question: "What do you call it when Batman skips church?".to_string(),
                answer: "Christian Bale".to_string(),
            }),
        };

        let mut file = tempfile().expect("Could not create tempfile");
        append_to_ledger(&LedgerEntry::NewCard(new_card.clone()), &mut file)
            .expect("Should have written to ledger successfully");

        file.seek(std::io::SeekFrom::Start(0))
            .expect("Should be able to seek to beginning of file");

        let mut cards = HashMap::new();
        let mut supermemo_deck = SuperMemoDeck::new();
        read_ledger(&file, &mut cards, &mut supermemo_deck).expect("Should be able to read_ledger");
        assert_eq!(cards.contains_key("banana-farm"), true);
        assert_eq!(*cards.get("banana-farm").unwrap(), new_card);
    }
}
