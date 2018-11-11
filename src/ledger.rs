use card::{AttemptRecord, Card, UuidString};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
pub enum LedgerEntry {
    NewCard(Card),
    UpdateTags(UuidString, Vec<String>),
    DeleteCard(UuidString),
    Attempt(AttemptRecord),
}

pub fn append_to_ledger(update: LedgerEntry, ledger_path: String) -> Result<(), std::io::Error> {
    let serialized = serde_json::to_string(&update).unwrap();
    println!("Serialized: {}", serialized);

    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(ledger_path)?;
    file.write(serialized.as_bytes())?;
    file.write(b"\n")?;
    file.flush()?;
    Ok(())
}

pub fn update_from_ledger(
    ledger_entry: LedgerEntry,
    cards: &mut HashMap<UuidString, Card>,
    attempts: &mut HashMap<UuidString, Vec<AttemptRecord>>,
) -> Result<(), String> {
    match ledger_entry {
        LedgerEntry::NewCard(new_card) => {
            cards.insert(new_card.uuid.clone(), new_card);
            Ok(())
        }
        LedgerEntry::DeleteCard(uuid) => {
            // TODO throw a warning if uuid doesn't exist?
            cards.remove(&uuid);
            Ok(())
        }
        LedgerEntry::Attempt(attempt) => {
            if attempts.contains_key(&attempt.uuid) {
                attempts.get_mut(&attempt.uuid).unwrap().push(attempt);
            } else {
                attempts.insert(attempt.uuid.clone(), vec![attempt]);
            }
            Ok(())
        }
        _ => Ok(()),
    }
}
