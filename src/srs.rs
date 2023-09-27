use std::time::Duration;

use lazy_static::lazy_static;
use chrono::{DateTime, FixedOffset};

lazy_static! {
    /// The initial intervals for new cards
    static ref INITIAL_INTERVALS: [Duration; 3] = [
        Duration::from_secs(1 * 60),
        Duration::from_secs(10 * 60),
        Duration::from_secs(24 * 60 * 60),
    ];
}

/// The default ease
const DEFAULT_EASE: f32 = 2.5;

/// The minimum ease
const MINIMUM_EASE: f32 = 1.3;

/// The easy bonus
const EASY_BONUS: f64 = 1.3;

/// The hard interval
const HARD_INTERVAL: f64 = 1.2;

/// A result type that boxes errors to a Box<dyn Error>.
pub type SrsResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Review difficulties.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Difficulty {
    Again = 0,
    Hard = 1,
    Good = 2,
    Easy = 3
}

// A single spaced repetition "card" (e.g. a puzzle).
// TODO: if this is ever going to be a hosted web app we need to make sure time zones are handled
// correctly.
#[derive(Debug)]
pub struct Card {
    pub id: String,
    pub due: Option<DateTime<FixedOffset>>,
    pub interval: Option<Duration>,
    pub review_count: i32,
    pub ease: f32,
}

impl Card {
    pub fn new(id: &str) -> Self
    {
        Self {
            id: id.to_string(),
            due: None,
            interval: None,
            review_count: 0,
            ease: DEFAULT_EASE,
        }
    }

    pub fn review(&mut self, time_now: DateTime<FixedOffset>, score: Difficulty) -> SrsResult<()> {
        // https://faqs.ankiweb.net/what-spaced-repetition-algorithm.html
        // For learning/relearning the algorithm is a bit different. We track if a card is
        // currently in the learning stage by its review count, if there's a corresponding entry in
        // INITIAL_INTERVALS that's one of the initial learning stages, once it passes out of there
        // it graduates to no longer being a new card.
        if self.review_count < INITIAL_INTERVALS.len() as i32 {
            // For cards in learning/relearning:
            // * Again moves the card back to the first stage of the new card intervals
            // * Hard repeats the current step
            // * Good moves the card to the next step, if the card was on the final step, it is
            //   converted into a review card
            // * Easy immediately converts the card into a review card
            // There are no ease adjustments for new cards.
            self.review_count = match score {
                Difficulty::Again => 0,
                Difficulty::Hard => self.review_count,
                Difficulty::Good => self.review_count + 1,
                Difficulty::Easy => INITIAL_INTERVALS.len() as i32,
            };

            let interval_index = i32::clamp(self.review_count, 0, INITIAL_INTERVALS.len() as i32 - 1);
            let new_interval = INITIAL_INTERVALS[interval_index as usize];
            let new_due = time_now + chrono::Duration::from_std(new_interval)?;

            self.interval = Some(new_interval);
            self.due = Some(new_due.fixed_offset());
        }
        else {
            // For cards that have graduated learning:
            // * Again puts the card back into learning mode, and decreases the ease by 20%
            // * Hard multiplies the current interval by the hard interval (1.2 by default) and
            //   decreases the ease by 15%
            // * Good multiplies the current interval by the ease
            // * Easy multiplies the current interval by the ease times the easy bonus (1.3 by
            //   default) and increases the ease by 15%
            let (new_interval, new_ease, new_review_count) = match score {
                Difficulty::Again => {
                    (INITIAL_INTERVALS[0], self.ease - 0.2, 0)
                },
                Difficulty::Hard => {
                    let new_interval = Self::mul_duration(self.interval.unwrap(), HARD_INTERVAL);
                    (new_interval, self.ease - 0.15, self.review_count + 1)
                },
                Difficulty::Good => {
                    let new_interval = Self::mul_duration(self.interval.unwrap(), self.ease as f64);
                    (new_interval, self.ease, self.review_count + 1)
                },
                Difficulty::Easy => {
                    let new_interval = Self::mul_duration(self.interval.unwrap(), self.ease as f64 * EASY_BONUS);
                    (new_interval, self.ease + 0.15, self.review_count + 1)
                },
            };

            let new_due = time_now + chrono::Duration::from_std(new_interval)?;

            self.interval = Some(new_interval);
            self.due = Some(new_due.fixed_offset());
            self.ease = f32::max(MINIMUM_EASE, new_ease);
            self.review_count = new_review_count;
        }

        Ok(())
    }

    fn mul_duration(duration: Duration, multiplier: f64) -> Duration {
        let new_interval_secs = duration.as_secs() as f64 * multiplier;
        Duration::from_secs(new_interval_secs as u64)
    }
}
