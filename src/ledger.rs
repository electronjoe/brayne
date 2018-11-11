use card::{AttemptRecord, Card, UuidString};
use priority_queue::PriorityQueue;
use std::cmp::Reverse;
use std::collections::HashMap;
use std::io::prelude::*;
use std::io::BufReader;
use std::time::SystemTime;
use supermemo::CardState;

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
    card_states: &mut HashMap<UuidString, CardState>,
    schedule: &mut PriorityQueue<String, Reverse<SystemTime>>,
) -> Result<(), String> {
    match ledger_entry {
        LedgerEntry::NewCard(new_card) => {
            cards.insert(new_card.uuid.clone(), new_card.clone());
            card_states.insert(new_card.uuid.clone(), CardState::new(new_card.created));
            schedule.push(
                new_card.uuid.clone(),
                Reverse(card_states.get(&new_card.uuid).unwrap().next_attempt()),
            );
            Ok(())
        }
        LedgerEntry::DeleteCard(uuid) => {
            // TODO throw a warning if uuid doesn't exist?
            cards.remove(&uuid);
            Ok(())
        }
        LedgerEntry::Attempt(attempt) => {
            card_states
                .get_mut(&attempt.uuid)
                .expect("Missing CardState, no prior NewCard?")
                .update(&attempt);
            if None == schedule.change_priority(
                &attempt.uuid,
                Reverse(card_states.get(&attempt.uuid).unwrap().next_attempt()),
            ) {
                schedule.push(
                    attempt.uuid.clone(),
                    Reverse(card_states.get(&attempt.uuid).unwrap().next_attempt()),
                );
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

pub fn read_ledger(
    file: &std::fs::File,
    cards: &mut HashMap<UuidString, Card>,
    card_states: &mut HashMap<UuidString, CardState>,
    schedule: &mut PriorityQueue<String, Reverse<SystemTime>>,
) -> Result<(), std::io::Error> {
    for (num, line) in BufReader::new(file).lines().enumerate() {
        let l = line?;
        let update: LedgerEntry = serde_json::from_str(&l)?;
        println!("LedgerEntry {}: {:?}", num, update);
        update_from_ledger(update, cards, card_states, schedule).unwrap();
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
        let mut card_states = HashMap::new();
        let mut schedule = PriorityQueue::new();
        read_ledger(&file, &mut cards, &mut card_states, &mut schedule)
            .expect("Should be able to read_ledger");
        assert_eq!(cards.contains_key("banana-farm"), true);
        assert_eq!(*cards.get("banana-farm").unwrap(), new_card);
    }
}
