use card::{AttemptQuality, AttemptRecord};
use std::ops::Add;
use std::time::Duration;

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
    fn new(created: std::time::SystemTime) -> CardState {
        CardState {
            recall_count: 0,
            effort_factor: 2.5,
            last_attempt: None,
            next_attempt: created,
        }
    }

    fn update(&mut self, attempt_record: &AttemptRecord) {
        const DAY_IN_SECONDS: u64 = 24 * 60 * 60;
        self.recall_count = match attempt_record.quality {
            AttemptQuality::Perfect
            | AttemptQuality::CorrectAfterHesitation
            | AttemptQuality::CorrectSeriousDifficulty => self.recall_count + 1,
            _ => 0,
        };
        self.effort_factor = update_effort_factor(self.effort_factor, attempt_record.quality);
        self.next_attempt = if self.recall_count == 1 {
            attempt_record.time.add(Duration::new(DAY_IN_SECONDS, 0))
        } else if self.recall_count == 2 {
            attempt_record
                .time
                .add(Duration::new(6 * DAY_IN_SECONDS, 0))
        } else {
            match attempt_record.quality {
                AttemptQuality::Perfect | AttemptQuality::CorrectAfterHesitation => {
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
                _ => attempt_record.time,
            }
        };
        self.last_attempt = Some(attempt_record.time);
    }
}

fn update_effort_factor(effort_factor: f32, quality: AttemptQuality) -> f32 {
    let quality_as_float = quality as i8 as f32;
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
        assert_approx_eq!(card_state.effort_factor, 2.18);
        assert_eq!(card_state.last_attempt, Some(first_attempt_time),);
        assert_eq!(card_state.next_attempt, first_attempt_time,);
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
        assert_approx_eq!(card_state.effort_factor, 1.3);
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
        let day = std::time::Duration::new(24 * 60 * 60, 0);

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
        assert_approx_eq!(card_state.effort_factor, 1.8);
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
        assert_approx_eq!(card_state.effort_factor, 1.66);
        assert_eq!(
            card_state.last_attempt,
            Some(first_attempt_time.add(Duration::new(4, 0))),
        );
        assert_eq!(
            card_state.next_attempt,
            first_attempt_time.add(Duration::new(4, 0)).add(day),
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
