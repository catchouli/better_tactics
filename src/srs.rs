use std::error::Error;
use lazy_static::lazy_static;
use chrono::{DateTime, FixedOffset, Local, Duration};

lazy_static! {
    /// The initial intervals for new cards
    static ref INITIAL_INTERVALS: [Duration; 3] = [
        Duration::seconds(1 * 60),
        Duration::seconds(10 * 60),
        Duration::seconds(24 * 60 * 60),
    ];
}

/// The default ease
const DEFAULT_EASE: f64 = 2.5;

/// The minimum ease
const MINIMUM_EASE: f64 = 1.3;

/// The easy bonus
const EASY_BONUS: f64 = 1.3;

/// The hard interval
const HARD_INTERVAL: f64 = 1.2;

/// A result type that boxes errors to a Box<dyn Error>.
pub type SrsResult<T> = Result<T, Box<dyn Error>>;

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
// correctly. (They should be, all times are in ISO form with the timezone, but we need to check.)
#[derive(Debug)]
pub struct Card {
    pub id: String,
    pub due: Option<DateTime<FixedOffset>>,
    pub interval: Option<Duration>,
    pub review_count: i64,
    pub ease: f64,
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

    /// Generate the next 'due time', i.e. if a card is due before this time it is essentially due
    /// today. For now we just use tommorow (local time) at 4am as the end of the day, and all cards
    /// before that are due today, like in anki.
    pub fn due_time() -> DateTime<FixedOffset> {
        // TODO: unsafe unwrap and quite ugly
        (Local::now() + Duration::seconds(60 * 60 * 24))
            .date_naive()
            .and_hms_opt(4, 0, 0)
            .unwrap()
            .and_local_timezone(Local)
            .latest()
            .unwrap()
            .fixed_offset()
    }

    pub fn review(&mut self, time_now: DateTime<FixedOffset>, score: Difficulty) -> SrsResult<()> {
        // https://faqs.ankiweb.net/what-spaced-repetition-algorithm.html
        // For learning/relearning the algorithm is a bit different. We track if a card is
        // currently in the learning stage by its review count, if there's a corresponding entry in
        // INITIAL_INTERVALS that's one of the initial learning stages, once it passes out of there
        // it graduates to no longer being a new card.
        if self.review_count < INITIAL_INTERVALS.len() as i64 {
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
                Difficulty::Easy => INITIAL_INTERVALS.len() as i64,
            };

            let interval_index = i64::clamp(self.review_count, 0, INITIAL_INTERVALS.len() as i64 - 1);
            let new_interval = INITIAL_INTERVALS[interval_index as usize];
            let new_due = time_now + new_interval;

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

            let new_due = time_now + new_interval;

            self.interval = Some(new_interval);
            self.due = Some(new_due.fixed_offset());
            self.ease = f64::max(MINIMUM_EASE, new_ease);
            self.review_count = new_review_count;
        }

        Ok(())
    }

    /// Multiply a duration by a float.
    fn mul_duration(duration: Duration, multiplier: f64) -> Duration {
        let new_interval_secs = duration.num_seconds() as f64 * multiplier;
        Duration::seconds(new_interval_secs as i64)
    }

    /// Get whether the card is due. (A new card with due == None is always due.)
    pub fn is_due(&self) -> bool {
        self.due.map(|due| {
            (due - Self::due_time()).num_seconds() <= 0
        }).unwrap_or(true)
    }

    /// Get a human readable time until due. (A new card with due == None is always due "now".)
    pub fn human_readable_due(&self) -> String {
        self.due.map(|due| {
            let time_until_due = due - Local::now().fixed_offset();
            crate::util::review_duration_to_human(&time_until_due)
        }).unwrap_or("now".to_string())
    }
}
