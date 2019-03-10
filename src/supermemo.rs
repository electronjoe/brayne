use card::{AttemptQuality, AttemptRecord, UuidString};
use priority_queue::PriorityQueue;
use std::cmp::Reverse;
use std::collections::HashMap;
use std::ops::Add;
use std::time::{Duration, SystemTime};

#[derive(Debug)]
pub struct SuperMemoDeck {
    card_states: HashMap<UuidString, CardState>,
    sorted_deck: PriorityQueue<UuidString, Reverse<SystemTime>>,
}

impl SuperMemoDeck {
    pub fn new() -> SuperMemoDeck {
        SuperMemoDeck {
            card_states: HashMap::new(),
            sorted_deck: PriorityQueue::new(),
        }
    }

    pub fn new_card(&mut self, uuid: &UuidString, created: &SystemTime) {
        self.card_states
            .insert(uuid.clone(), CardState::new(created.clone()));
    }

    pub fn delete_card(&mut self, uuid: &UuidString) -> bool {
        self.card_states.remove(uuid).is_some()
    }

    pub fn insert_attempt(
        &mut self,
        uuid: &UuidString,
        attempt: &AttemptRecord,
    ) -> Result<(), String> {
        Ok(())
    }

    pub fn draw_card(&self) -> Option<UuidString> {
        //         let card_ready = match schedule.peek() {
        //     Some((_uuid, Reverse(next_challenge_time))) => {
        //         *next_challenge_time <= SystemTime::now()
        //     }
        //     None => false,
        // };
        Some("uuid".to_string())
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
            _ => 0,
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
            AttemptQuality::Perfect | AttemptQuality::CorrectAfterHesitation => {
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
            _ => attempt_record.time,
        };

        self.last_attempt = Some(attempt_record.time);
    }

    pub fn next_attempt(&self) -> SystemTime {
        self.next_attempt
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
    use std::ops::{Add, Mul, Sub};
    use std::time::{Duration, SystemTime};

    #[test]
    fn test_perfect_update() {
        let day = std::time::Duration::new(24 * 60 * 60, 0);
        let first_attempt_time = SystemTime::now().sub(Duration::new(20, 0));
        let mut card_state = CardState::new(SystemTime::now().sub(Duration::new(22, 0)));
        card_state.update(&AttemptRecord {
            uuid: "flat-banana".to_string(),
            time: first_attempt_time,
            quality: AttemptQuality::Perfect,
        });
        assert_eq!(card_state.recall_count, 1);
        assert_approx_eq!(card_state.effort_factor, 2.6);
        assert_eq!(card_state.last_attempt, Some(first_attempt_time),);
        assert_eq!(card_state.next_attempt, first_attempt_time.add(day),);
    }

    #[test]
    fn test_forget_update() {
        let first_attempt_time = SystemTime::now().sub(Duration::new(20, 0));
        let mut card_state = CardState::new(SystemTime::now().sub(Duration::new(22, 0)));
        card_state.update(&AttemptRecord {
            uuid: "flat-banana".to_string(),
            time: first_attempt_time,
            quality: AttemptQuality::IncorrectButEasyRecall,
        });
        assert_eq!(card_state.recall_count, 0);
        assert_approx_eq!(card_state.effort_factor, 2.5);
        assert_eq!(card_state.last_attempt, Some(first_attempt_time),);
        assert_eq!(card_state.next_attempt, first_attempt_time,);
    }

    #[test]
    fn test_recover_from_failure() {
        const DAY_IN_SECONDS: u64 = 24 * 60 * 60;
        let first_attempt_time = SystemTime::now().sub(Duration::new(20, 0));
        let mut card_state = CardState::new(SystemTime::now().sub(Duration::new(22, 0)));
        card_state.update(&AttemptRecord {
            uuid: "flat-banana".to_string(),
            time: first_attempt_time,
            quality: AttemptQuality::IncorrectButEasyRecall,
        });
        assert_eq!(card_state.recall_count, 0);
        assert_approx_eq!(card_state.effort_factor, 2.5);
        assert_eq!(card_state.last_attempt, Some(first_attempt_time),);
        assert_eq!(card_state.next_attempt, first_attempt_time,);
        card_state.update(&AttemptRecord {
            uuid: "flat-banana".to_string(),
            time: first_attempt_time.add(Duration::new(2, 0)),
            quality: AttemptQuality::Perfect,
        });
        assert_eq!(card_state.recall_count, 1);
        assert_approx_eq!(card_state.effort_factor, 2.6);
        assert_eq!(
            card_state.last_attempt,
            Some(first_attempt_time.add(Duration::new(2, 0))),
        );
        assert_eq!(
            card_state.next_attempt,
            first_attempt_time
                .add(Duration::new(2, 0))
                .add(Duration::new(DAY_IN_SECONDS, 0)),
        );
    }

    #[test]
    fn test_persistent_effort_factor() {
        let first_attempt_time = SystemTime::now().sub(Duration::new(20, 0));
        let mut card_state = CardState::new(SystemTime::now().sub(Duration::new(22, 0)));
        card_state.update(&AttemptRecord {
            uuid: "flat-banana".to_string(),
            time: first_attempt_time,
            quality: AttemptQuality::Perfect,
        });
        card_state.update(&AttemptRecord {
            uuid: "flat-banana".to_string(),
            time: first_attempt_time.add(Duration::new(2, 0)),
            quality: AttemptQuality::IncorrectButEasyRecall,
        });
        card_state.update(&AttemptRecord {
            uuid: "flat-banana".to_string(),
            time: first_attempt_time.add(Duration::new(4, 0)),
            quality: AttemptQuality::IncorrectButRemembered,
        });
        card_state.update(&AttemptRecord {
            uuid: "flat-banana".to_string(),
            time: first_attempt_time.add(Duration::new(6, 0)),
            quality: AttemptQuality::Blackout,
        });
        assert_eq!(card_state.recall_count, 0);
        assert_approx_eq!(card_state.effort_factor, 2.6);
        assert_eq!(
            card_state.last_attempt,
            Some(first_attempt_time.add(Duration::new(6, 0))),
        );
        assert_eq!(
            card_state.next_attempt,
            first_attempt_time.add(Duration::new(6, 0)),
        );
    }

    #[test]
    fn test_require_recall_for_passage() {
        let first_attempt_time = SystemTime::now().sub(Duration::new(20, 0));
        let mut card_state = CardState::new(SystemTime::now().sub(Duration::new(22, 0)));
        card_state.update(&AttemptRecord {
            uuid: "flat-banana".to_string(),
            time: first_attempt_time,
            quality: AttemptQuality::Perfect,
        });
        card_state.update(&AttemptRecord {
            uuid: "flat-banana".to_string(),
            time: first_attempt_time.add(Duration::new(2, 0)),
            quality: AttemptQuality::Blackout,
        });
        assert_eq!(card_state.recall_count, 0);
        assert_approx_eq!(card_state.effort_factor, 2.6);
        assert_eq!(
            card_state.last_attempt,
            Some(first_attempt_time.add(Duration::new(2, 0))),
        );
        assert_eq!(
            card_state.next_attempt,
            first_attempt_time.add(Duration::new(2, 0)),
        );
        card_state.update(&AttemptRecord {
            uuid: "flat-banana".to_string(),
            time: first_attempt_time.add(Duration::new(4, 0)),
            quality: AttemptQuality::CorrectSeriousDifficulty,
        });
        assert_eq!(card_state.recall_count, 1);
        assert_approx_eq!(card_state.effort_factor, 2.46);
        assert_eq!(
            card_state.last_attempt,
            Some(first_attempt_time.add(Duration::new(4, 0))),
        );
        assert_eq!(
            card_state.next_attempt,
            first_attempt_time.add(Duration::new(4, 0)),
        );
    }

    #[test]
    fn test_first_three_intervals() {
        let first_attempt_time = SystemTime::now().sub(Duration::new(20, 0));
        let day = std::time::Duration::new(24 * 60 * 60, 0);

        let mut card_state = CardState::new(SystemTime::now().sub(Duration::new(22, 0)));
        card_state.update(&AttemptRecord {
            uuid: "flat-banana".to_string(),
            time: first_attempt_time,
            quality: AttemptQuality::Perfect,
        });
        assert_eq!(card_state.recall_count, 1);
        assert_approx_eq!(card_state.effort_factor, 2.6);
        assert_eq!(card_state.last_attempt, Some(first_attempt_time),);
        assert_eq!(card_state.next_attempt, first_attempt_time.add(day),);
        card_state.update(&AttemptRecord {
            uuid: "flat-banana".to_string(),
            time: first_attempt_time.add(Duration::new(2, 0)),
            quality: AttemptQuality::Perfect,
        });
        assert_eq!(card_state.recall_count, 2);
        assert_approx_eq!(card_state.effort_factor, 2.7);
        assert_eq!(
            card_state.last_attempt,
            Some(first_attempt_time.add(Duration::new(2, 0))),
        );
        assert_eq!(
            card_state.next_attempt,
            first_attempt_time.add(Duration::new(2, 0)).add(day.mul(6)),
        );
        card_state.update(&AttemptRecord {
            uuid: "flat-banana".to_string(),
            time: first_attempt_time.add(Duration::new(4, 0)),
            quality: AttemptQuality::Perfect,
        });
        assert_eq!(card_state.recall_count, 3);
        assert_approx_eq!(card_state.effort_factor, 2.8);
        assert_eq!(
            card_state.last_attempt,
            Some(first_attempt_time.add(Duration::new(4, 0))),
        );
        let final_interval_secs = day.mul(6).as_secs() as f32 * 2.8;
        assert_eq!(
            card_state.next_attempt,
            card_state
                .last_attempt
                .unwrap()
                .add(Duration::new(final_interval_secs as u64 - 1, 0)),
        );
    }
}
