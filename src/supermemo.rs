use card::{AttemptQuality, AttemptRecord, UuidString};
use priority_queue::PriorityQueue;
use std::cmp::Reverse;
use std::collections::{HashMap, VecDeque};
use std::ops::Add;
use std::time::{Duration, SystemTime};

#[derive(Debug)]
pub struct SuperMemoDeck {
    // The card_states store state informatino necesary for tracking card difficulty and next
    // attempt time.
    card_states: HashMap<UuidString, CardState>,
    // The sorted_deck is a PriorityQueue sorted by card next_attempt time (from card_states).
    // When attempts are made, if scoring CorrectAfterHesitation or better, the next_attempt
    // time is updated for the associated card in sorted_deck.  If scoring worse than
    // CorrectAfterHesitation, the card is removed from the sorted_deck and placed in repeat_deck.
    sorted_deck: PriorityQueue<UuidString, Reverse<SystemTime>>,
    // The repeat deck holds cards that should be repeated this session until
    // at least scoring CorrectAfterHesitation (and can then be moved back into sorted_deck).
    // In the event the card from the repeat Deck is more than 6 hours old, it gets moved
    // back to the sorted_deck for normal use. While in the repeat deck, attempts on the card
    // do not update difficulty level contained in card_states.
    repeat_deck: VecDeque<(UuidString, SystemTime)>,
}

impl SuperMemoDeck {
    pub fn new() -> SuperMemoDeck {
        SuperMemoDeck {
            card_states: HashMap::new(),
            sorted_deck: PriorityQueue::new(),
            repeat_deck: VecDeque::new(),
        }
    }

    pub fn new_card(&mut self, uuid: UuidString, created: SystemTime) {
        self.card_states
            .insert(uuid.clone(), CardState::new(created.clone()));
        self.sorted_deck.push(uuid, Reverse(created));
    }

    pub fn delete_card(&mut self, uuid: &UuidString) -> bool {
        // Write author to suggest .retain() member function?
        // self.sorted_deck.retain()
        self.sorted_deck
            .change_priority(uuid, Reverse(SystemTime::now()));
        self.sorted_deck.pop();
        self.repeat_deck.retain(|(u, _t)| u != uuid);
        self.card_states.remove(uuid).is_some()
    }

    pub fn insert_attempt(&mut self, attempt: &AttemptRecord) -> Result<(), String> {
        let from_sorted_deck = if let Some((front_uuid, _)) = self.sorted_deck.peek() {
            *front_uuid == attempt.uuid
        } else {
            false
        };
        let from_repeat_deck = if let Some((front_uuid, _)) = self.repeat_deck.front() {
            *front_uuid == attempt.uuid
        } else {
            false
        };

        if from_sorted_deck {
            self.card_states
                .get_mut(&attempt.uuid)
                .expect("should exist")
                .update(attempt);
            self.sorted_deck.change_priority(
                &attempt.uuid,
                Reverse(
                    self.card_states
                        .get(&attempt.uuid)
                        .expect("should exist")
                        .next_attempt,
                ),
            );
            match attempt.quality {
                AttemptQuality::CorrectSeriousDifficulty
                | AttemptQuality::IncorrectButEasyRecall
                | AttemptQuality::IncorrectButRemembered
                | AttemptQuality::Blackout => {
                    self.sorted_pop_to_repeat(attempt.uuid.to_string(), attempt.time)?;
                }
                _ => {
                    ();
                }
            }
            return Ok(());
        }

        // Repeat deck attempts do not update CardState, but can transition card out of repeat_deck
        if from_repeat_deck {
            match attempt.quality {
                AttemptQuality::Perfect | AttemptQuality::CorrectAfterHesitation => {
                    self.repeat_front_to_sorted(attempt.uuid.clone()).unwrap();
                    return Ok(());
                }
                _ => return Ok(()),
            }
        }

        return Err(
            "card not contained at front of sorted or repeat deck, unexpected attempt".to_string(),
        );
    }

    pub fn draw_card(&mut self, now: SystemTime) -> Option<UuidString> {
        // If a sorted_deck card is due for attempt, return it (but do not pop)
        // The card will have it's priority updated or will be popped upon insert_attempt
        match self.sorted_deck.peek() {
            Some((uuid, Reverse(next_challenge_time))) => {
                if *next_challenge_time <= now {
                    return Some(uuid.to_string());
                }
            }
            None => (),
        };

        // No sorted_deck card is due for attempt, check for repeat_deck freshness
        while !self.repeat_deck.is_empty() {
            let (uuid, repeat_insert_time) = self
                .repeat_deck
                .front()
                .expect("Validated !empty, but was empty")
                .clone();
            if repeat_insert_time.add(Duration::new(6 * 60 * 60, 0)) > now {
                return Some(uuid.to_string());
            } else {
                self.repeat_front_to_sorted(uuid).unwrap();
            }
        }
        None
    }

    // TODO On failures should we roll back transaction (re-insert to repeat_deck?)
    fn repeat_front_to_sorted(&mut self, uuid: UuidString) -> Result<(), String> {
        let _should_pop = self
            .repeat_deck
            .front()
            .ok_or("repeat_deck was empty".to_string())
            .map(|(front_uuid, _)| {
                if *front_uuid == uuid {
                    Ok(true)
                } else {
                    Err("uuid specified is not at front of repeat_deck".to_string())
                }
            })?;
        self.repeat_deck
            .pop_front()
            .ok_or("failed to pop_front on repeat_deck")?;
        let card_state = self
            .card_states
            .get(&uuid)
            .ok_or("uuid specified is not within card_states")?;
        let _pushed_ok = self
            .sorted_deck
            .push(uuid, Reverse(card_state.next_attempt))
            .map_or(Ok(()), |_| {
                Err("uuid specifed already contained in sorted_deck".to_string())
            })?;
        Ok(())
    }

    fn sorted_pop_to_repeat(&mut self, uuid: UuidString, now: SystemTime) -> Result<(), String> {
        let _should_pop = self
            .sorted_deck
            .peek()
            .ok_or("sorted_deck was empty".to_string())
            .map(|(front_uuid, _)| {
                if *front_uuid == uuid {
                    Ok(true)
                } else {
                    Err("uuid specified is not at top of sorted_deck")
                }
            })?;
        let _popped_ok = self
            .sorted_deck
            .pop()
            .ok_or("Could not pop from sorted_deck")?;
        let _pushed_ok = self.repeat_deck.push_back((uuid, now));
        Ok(())
    }
    // For Display
    // println!("Cards: {:?}", cards);
    // for (ref item, ref next_challenge_time) in schedule.clone().into_sorted_iter() {
    //     println!("\t{:?} @ {:?}", item, next_challenge_time);
    // }
}

#[derive(Debug)]
struct CardState {
    // Count of consecutive AttemptRecords scoring >= CorrectSeriousDifficulty
    recall_count: i32,
    // Effort factor up through last AttemptRecord
    effort_factor: f32,
    // Time of the last Attempt
    last_attempt: Option<std::time::SystemTime>,
    // Time of the next Attempt
    next_attempt: std::time::SystemTime,
}

impl CardState {
    pub fn new(created: std::time::SystemTime) -> CardState {
        CardState {
            recall_count: 0,
            effort_factor: 2.5,
            last_attempt: None,
            next_attempt: created,
        }
    }

    pub fn update(&mut self, attempt_record: &AttemptRecord) {
        const DAY_IN_SECONDS: u64 = 24 * 60 * 60;
        self.recall_count = match attempt_record.quality {
            AttemptQuality::Perfect
            | AttemptQuality::CorrectAfterHesitation
            | AttemptQuality::CorrectSeriousDifficulty => self.recall_count + 1,
            _ => 1,
        };
        self.effort_factor = match attempt_record.quality {
            AttemptQuality::Perfect
            | AttemptQuality::CorrectAfterHesitation
            | AttemptQuality::CorrectSeriousDifficulty => {
                update_effort_factor(self.effort_factor, attempt_record.quality)
            }
            _ => self.effort_factor,
        };
        self.next_attempt = match attempt_record.quality {
            AttemptQuality::Perfect
            | AttemptQuality::CorrectAfterHesitation
            | AttemptQuality::CorrectSeriousDifficulty => {
                if self.recall_count == 1 {
                    println!("recall_count == 1 for {:?}", attempt_record.uuid);
                    attempt_record.time.add(Duration::new(DAY_IN_SECONDS, 0))
                } else if self.recall_count == 2 {
                    attempt_record
                        .time
                        .add(Duration::new(6 * DAY_IN_SECONDS, 0))
                } else {
                    let prior_duration = self
                        .next_attempt
                        .duration_since(
                            self.last_attempt
                                .expect("Missing last_attempt for card with recall_count non-zero"),
                        ).expect("next_attempt should be later than last_attempt");
                    let next_duration_in_seconds =
                        prior_duration.as_secs() as f32 * self.effort_factor;
                    attempt_record
                        .time
                        .add(Duration::new(next_duration_in_seconds as u64, 0))
                }
            }
            _ => {
                // Forgot the card, will be placed in repeat deck, next attempt resets to one day
                attempt_record.time.add(Duration::new(DAY_IN_SECONDS, 0))
            }
        };

        self.last_attempt = Some(attempt_record.time);
    }
}

fn update_effort_factor(effort_factor: f32, quality: AttemptQuality) -> f32 {
    let quality_as_float = f32::from(quality as i8);
    let new_effort_factor =
        effort_factor + 0.1 - (5.0 - quality_as_float) * (0.08 + (5.0 - quality_as_float) * 0.02);

    // Clamp below by 1.3
    if new_effort_factor < 1.3 {
        return 1.3;
    }
    new_effort_factor
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ops::{Add, Mul};
    use std::time::{Duration, SystemTime};

    #[derive(Debug)]
    struct EventAtTime {
        time: SystemTime,
        event: Event,
    }

    #[derive(Debug)]
    enum Event {
        NewCard(UuidString),
        Attempt(AttemptRecord),
        Draw(Option<UuidString>),
    }

    fn execute_data_driven_test(deck: &mut SuperMemoDeck, events: &Vec<EventAtTime>) {
        for (i, event_time) in events.iter().enumerate() {
            let now = event_time.time;
            println!("row {}, now {:?}, deck: {:?}", i, now, deck);
            match event_time.event {
                Event::NewCard(ref uuid) => {
                    deck.new_card(uuid.clone(), now);
                }
                Event::Attempt(ref attempt_record) => {
                    assert_eq!(attempt_record.time, now, "row {} test driven table inconsistent, attempt record should have same time as test event", i);
                    deck.insert_attempt(&attempt_record)
                        .expect("insert_attempt should succeed");
                }
                Event::Draw(ref maybe_uuid) => {
                    assert_eq!(deck.draw_card(now), *maybe_uuid, "Row {} of test data", i);
                }
            }
        }
    }

    #[test]
    fn test_new_immediate_failure() {
        let now = SystemTime::now();
        let day = Duration::new(24 * 60 * 60, 0);
        let mut deck = SuperMemoDeck::new();

        let test_data = vec![
            EventAtTime {
                time: now,
                event: Event::NewCard("banana-uuid".to_string()),
            },
            EventAtTime {
                time: now.add(day),
                event: Event::Draw(Some("banana-uuid".to_string())),
            },
            EventAtTime {
                time: now.add(day),
                event: Event::Attempt(AttemptRecord {
                    uuid: "banana-uuid".to_string(),
                    time: now.add(day),
                    quality: AttemptQuality::IncorrectButEasyRecall,
                }),
            },
            EventAtTime {
                time: now.add(day),
                event: Event::Draw(Some("banana-uuid".to_string())),
            },
            EventAtTime {
                time: now.add(day),
                event: Event::Attempt(AttemptRecord {
                    uuid: "banana-uuid".to_string(),
                    time: now.add(day),
                    quality: AttemptQuality::Perfect,
                }),
            },
            EventAtTime {
                time: now.add(day),
                event: Event::Draw(None),
            },
            EventAtTime {
                time: now.add(day.mul(2)),
                event: Event::Draw(Some("banana-uuid".to_string())),
            },
        ];

        execute_data_driven_test(&mut deck, &test_data);
    }

    #[test]
    fn test_new_perfect_recall() {
        let now = SystemTime::now();
        let day = Duration::new(24 * 60 * 60, 0);
        let mut deck = SuperMemoDeck::new();

        let test_data = vec![
            EventAtTime {
                time: now,
                event: Event::NewCard("banana-uuid".to_string()),
            },
            EventAtTime {
                time: now,
                event: Event::Draw(Some("banana-uuid".to_string())),
            },
            EventAtTime {
                time: now,
                event: Event::Attempt(AttemptRecord {
                    uuid: "banana-uuid".to_string(),
                    time: now,
                    quality: AttemptQuality::Perfect,
                }),
            },
            EventAtTime {
                time: now,
                event: Event::Draw(None),
            },
            EventAtTime {
                time: now.add(day),
                event: Event::Draw(Some("banana-uuid".to_string())),
            },
            EventAtTime {
                time: now.add(day),
                event: Event::Attempt(AttemptRecord {
                    uuid: "banana-uuid".to_string(),
                    time: now.add(day),
                    quality: AttemptQuality::Perfect,
                }),
            },
            EventAtTime {
                time: now.add(day.mul(6)),
                event: Event::Draw(None),
            },
            EventAtTime {
                time: now.add(day.mul(7)),
                event: Event::Draw(Some("banana-uuid".to_string())),
            },
            EventAtTime {
                time: now.add(day.mul(7)),
                event: Event::Attempt(AttemptRecord {
                    uuid: "banana-uuid".to_string(),
                    time: now.add(day.mul(7)),
                    quality: AttemptQuality::Perfect,
                }),
            },
            EventAtTime {
                time: now.add(day.mul(23)),
                event: Event::Draw(None),
            },
            EventAtTime {
                time: now.add(day.mul(24)),
                event: Event::Draw(Some("banana-uuid".to_string())),
            },
        ];

        execute_data_driven_test(&mut deck, &test_data);
    }

    // Test repeat on Remembered but difficult
    #[test]
    fn test_correct_serious_difficult_repeat_deck() {
        let now = SystemTime::now();
        let day = Duration::new(24 * 60 * 60, 0);
        let mut deck = SuperMemoDeck::new();

        let test_data = vec![
            EventAtTime {
                // 0
                time: now,
                event: Event::NewCard("banana-uuid".to_string()),
            },
            EventAtTime {
                // 1
                time: now,
                event: Event::Attempt(AttemptRecord {
                    uuid: "banana-uuid".to_string(),
                    time: now,
                    quality: AttemptQuality::Perfect,
                }),
            },
            EventAtTime {
                // 2
                time: now.add(day),
                event: Event::Attempt(AttemptRecord {
                    uuid: "banana-uuid".to_string(),
                    time: now.add(day),
                    quality: AttemptQuality::CorrectSeriousDifficulty,
                }),
            },
            EventAtTime {
                // 3
                // Confirm card is now in repeat deck
                time: now.add(day),
                event: Event::Draw(Some("banana-uuid".to_string())),
            },
            EventAtTime {
                // 4
                // This will NOT update EF, but removes card from repeat deck
                time: now.add(day),
                event: Event::Attempt(AttemptRecord {
                    uuid: "banana-uuid".to_string(),
                    time: now.add(day),
                    quality: AttemptQuality::Perfect,
                }),
            },
            EventAtTime {
                // 5
                // Confirm card is no longer in repeat deck
                time: now.add(day),
                event: Event::Draw(None),
            },
            EventAtTime {
                // 6
                time: now.add(day.mul(7)),
                event: Event::Draw(Some("banana-uuid".to_string())),
            },
            EventAtTime {
                // 7
                time: now.add(day.mul(7)),
                event: Event::Attempt(AttemptRecord {
                    uuid: "banana-uuid".to_string(),
                    time: now.add(day.mul(7)),
                    quality: AttemptQuality::Perfect,
                }),
            },
            EventAtTime {
                // 8
                time: now.add(day.mul(22)),
                event: Event::Draw(None),
            },
            EventAtTime {
                // 9
                time: now.add(day.mul(23)),
                event: Event::Draw(Some("banana-uuid".to_string())),
            },
        ];

        execute_data_driven_test(&mut deck, &test_data);
    }

    #[test]
    fn test_interval_reset() {
        let now = SystemTime::now();
        let day = Duration::new(24 * 60 * 60, 0);
        let mut deck = SuperMemoDeck::new();

        let test_data = vec![
            EventAtTime {
                time: now,
                event: Event::NewCard("banana-uuid".to_string()),
            },
            EventAtTime {
                time: now,
                event: Event::Attempt(AttemptRecord {
                    uuid: "banana-uuid".to_string(),
                    time: now,
                    quality: AttemptQuality::Perfect, // EF': 2.6
                }),
            },
            EventAtTime {
                // This will NOT update EF, but will trigger interval reset
                time: now.add(day),
                event: Event::Attempt(AttemptRecord {
                    uuid: "banana-uuid".to_string(),
                    time: now.add(day),
                    quality: AttemptQuality::IncorrectButEasyRecall,
                }),
            },
            EventAtTime {
                // Confirm card is now in repeat deck
                time: now.add(day),
                event: Event::Draw(Some("banana-uuid".to_string())),
            },
            EventAtTime {
                // This will NOT update EF, but removes card from repeat deck
                time: now.add(day),
                event: Event::Attempt(AttemptRecord {
                    uuid: "banana-uuid".to_string(),
                    time: now.add(day),
                    quality: AttemptQuality::CorrectAfterHesitation,
                }),
            },
            EventAtTime {
                // Confirm card is no longer in repeat deck
                time: now.add(day),
                event: Event::Draw(None),
            },
            EventAtTime {
                // Confirm card is available after a day
                time: now.add(day.mul(2)),
                event: Event::Draw(Some("banana-uuid".to_string())),
            },
            // TODO Continue to confirm EF was not updated
        ];

        execute_data_driven_test(&mut deck, &test_data);
    }

    #[test]
    fn test_two_cards_with_repeat_deck() {
        let now = SystemTime::now();
        let day = Duration::new(24 * 60 * 60, 0);
        let mut deck = SuperMemoDeck::new();

        let test_data = vec![
            EventAtTime {  // 0
                time: now,
                event: Event::NewCard("banana-uuid".to_string()),
            },
            EventAtTime {  // 1
                time: now.add(day),
                event: Event::NewCard("coconut-uuid".to_string()),
            },
            EventAtTime {  // 2
                time: now.add(day),
                event: Event::Draw(Some("banana-uuid".to_string())),
            },
            EventAtTime {  // 3 - Bad recall moves to repeat_deck
                time: now.add(day),
                event: Event::Attempt(AttemptRecord {
                    uuid: "banana-uuid".to_string(),
                    time: now.add(day),
                    quality: AttemptQuality::IncorrectButEasyRecall,
                }),
            },
            EventAtTime {  // 4 - Should pull from sorted_deck over repeat_deck
                time: now.add(day),
                event: Event::Draw(Some("coconut-uuid".to_string())),
            },
            EventAtTime {  // 5 - Perfect recall pushes coconut-uuid to later repeat date (+1d)
                time: now.add(day),
                event: Event::Attempt(AttemptRecord {
                    uuid: "coconut-uuid".to_string(),
                    time: now.add(day),
                    quality: AttemptQuality::Perfect,
                }),
            },
            EventAtTime {  // 6 - Should now pull from repeat_deck
                time: now.add(day),
                event: Event::Draw(Some("banana-uuid".to_string())),
            },
      ];

        execute_data_driven_test(&mut deck, &test_data);
    }

    #[test]
    fn test_two_cards_successes () {
        let now = SystemTime::now();
        let day = Duration::new(24 * 60 * 60, 0);
        let hour = Duration::new(60 * 60, 0);
        let mut deck = SuperMemoDeck::new();

        let test_data = vec![
            EventAtTime {  // 0
                time: now,
                event: Event::NewCard("banana-uuid".to_string()),
            },
            EventAtTime {  // 1
                time: now.add(day),
                event: Event::NewCard("coconut-uuid".to_string()),
            },
            EventAtTime {  // 2
                time: now.add(day),
                event: Event::Draw(Some("banana-uuid".to_string())),
            },
            EventAtTime {  // 3 - Good recall pushes banana-uuid to later repeat date (+1d)
                time: now.add(day),
                event: Event::Attempt(AttemptRecord {
                    uuid: "banana-uuid".to_string(),
                    time: now.add(day),
                    quality: AttemptQuality::CorrectAfterHesitation,
                }),
            },
            EventAtTime {  // 4 - Now expect coconut-uuid from sorted_deck
                time: now.add(day).add(hour),
                event: Event::Draw(Some("coconut-uuid".to_string())),
            },
            EventAtTime {  // 5 - Perfect recall pushes coconut-uuid to later repeat date (+1d)
                time: now.add(day).add(hour),
                event: Event::Attempt(AttemptRecord {
                    uuid: "coconut-uuid".to_string(),
                    time: now.add(day).add(hour),
                    quality: AttemptQuality::Perfect,
                }),
            },
            EventAtTime {  // 6 - A day later, banana-uuid should come up first
                time: now.add(day.mul(2).add(hour)),
                event: Event::Draw(Some("banana-uuid".to_string())),
            },
            EventAtTime {  // 7
                time: now.add(day.mul(2).add(hour)),
                event: Event::Attempt(AttemptRecord {
                    uuid: "banana-uuid".to_string(),
                    time: now.add(day.mul(2).add(hour)),
                    quality: AttemptQuality::CorrectAfterHesitation,
                }),
            },
            EventAtTime {  // 8 - Followed by coconut-uuid
                time: now.add(day.mul(2).add(hour)),
                event: Event::Draw(Some("coconut-uuid".to_string())),
            },
      ];

        execute_data_driven_test(&mut deck, &test_data);
    }

    // Test repeated use of repeat deck if necessary
    // Test timeout from repeat deck

    // Property Checks
    // Number of daily draw opportunities for attempts strictly better (+1) strictly less or equal
}
