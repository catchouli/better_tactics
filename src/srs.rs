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
    pub due: DateTime<FixedOffset>,
    pub interval: Duration,
    pub review_count: i64,
    pub ease: f64,
    pub learning: bool,
    pub learning_stage: i64,
}

impl Card {
    pub fn new(id: &str) -> Self
    {
        Self {
            id: id.to_string(),
            due: Local::now().fixed_offset(),
            interval: INITIAL_INTERVALS[0],
            review_count: 0,
            ease: DEFAULT_EASE,
            learning: true,
            learning_stage: 0,
        }
    }

    /// Generate the next 'due time', i.e. if a card is due before this time it is essentially due
    /// today. For now we just use tommorow (local time) at 4am as the end of the day, and all cards
    /// before that are due today, like in anki.
    pub fn due_time() -> SrsResult<DateTime<FixedOffset>> {
        Ok((Local::now() + Duration::seconds(60 * 60 * 24))
            .date_naive()
            .and_hms_opt(4, 0, 0)
            .unwrap()
            .and_local_timezone(Local)
            .latest()
            .ok_or_else(|| format!(""))?
            .fixed_offset())
    }

    /// Check whether the card is in 'learning' state.
    fn card_in_learning(&self) -> bool {
        let is_learning_review = self.review_count < INITIAL_INTERVALS.len() as i64;

        if is_learning_review {
            let interval_index = i64::clamp(self.review_count, 0, INITIAL_INTERVALS.len() as i64 - 1);
            let standard_learning_interval = INITIAL_INTERVALS[interval_index as usize];

            // If the interval is already past the normal interval for this 'learning' level, (e.g.
            // because the user pressed 'easy' previously), we can just consider the card not in
            // learning. This seems overly complex though and we probably just want to store if the
            // card is mature or not yet and update it whenever it goes out of learning.
            if self.interval < standard_learning_interval {
                true
            }
            else {
                false
            }
        }
        else {
            false
        }
    }

    /// Get the next interval after a review with score `score`.
    /// TODO: oh my god this got complicated as I tweaked it to work better. It needs fully
    /// rewriting to keep track of whether the card is mature yet, and what learning review we're on.
    pub fn next_interval(&self, score: Difficulty) -> Duration {
        let in_learning = self.card_in_learning();
        let review_count = self.review_count;
        let interval = self.interval;

        // Choose the new interval based on the score and if the card is in learning still.
        match score {
            Difficulty::Again => {
                // Failing a card always just resets it to e.g. 10 mins.
                INITIAL_INTERVALS[1]
            }
            Difficulty::Hard => if in_learning {
                // We never want hard or good to be less than again.
                let interval_index = i64::clamp(review_count, 0, INITIAL_INTERVALS.len() as i64 - 1);
                Duration::max(
                    Self::mul_duration(INITIAL_INTERVALS[interval_index as usize], HARD_INTERVAL),
                    INITIAL_INTERVALS[1])
            }
            else {
                Duration::max(
                    Self::mul_duration(interval, HARD_INTERVAL),
                    INITIAL_INTERVALS[1])
            },
            Difficulty::Good => {
                let good_duration = Duration::max(
                    Self::mul_duration(interval, self.ease),
                    INITIAL_INTERVALS[1]);

                if in_learning {
                    let interval_index = i64::clamp(review_count, 0, INITIAL_INTERVALS.len() as i64 - 1);
                    Duration::max(INITIAL_INTERVALS[interval_index as usize], good_duration)
                }
                else {
                    // If it's out of learning, the next interval shouldn't be less than the last
                    // learning interval.
                    Duration::max(good_duration, *INITIAL_INTERVALS.last().unwrap())
                }
            },
            Difficulty::Easy => {
                let easy_duration = Self::mul_duration(interval, self.ease * EASY_BONUS);

                // If we're in learning and it's easy, we can just skip the card past learning by
                // putting it tommorow.
                Duration::max(Duration::days(1), easy_duration)
            },
        }
    }

    // TODO: a bit unnecessarily complicated and messy now I've tweaked it to work better. We
    // should just track if the card is mature or not, and update it as necessary.
    pub fn review(&mut self, time_now: DateTime<FixedOffset>, score: Difficulty) {
        let new_interval = self.next_interval(score);

        // https://faqs.ankiweb.net/what-spaced-repetition-algorithm.html
        // For learning/relearning the algorithm is a bit different. We track if a card is
        // currently in the learning stage by its review count, if there's a corresponding entry in
        // INITIAL_INTERVALS that's one of the initial learning stages, once it passes out of there
        // it graduates to no longer being a new card.
        if self.card_in_learning() {
            // For cards in learning/relearning:
            // * Again moves the card back to the first stage of the new card intervals
            // * Hard repeats the current step
            // * Good moves the card to the next step, if the card was on the final step, it is
            //   converted into a review card
            // * Easy immediately converts the card into a review card
            // There are no ease adjustments for new cards.
            self.review_count = match score {
                Difficulty::Again => 0,
                Difficulty::Hard => self.review_count + 1,
                Difficulty::Good => self.review_count + 1,
                Difficulty::Easy => self.review_count + 1,
            };

            let new_due = time_now + new_interval;

            self.interval = new_interval;
            self.due = new_due.fixed_offset();
        }
        else {
            // For cards that have graduated learning:
            // * Again puts the card back into learning mode, and decreases the ease by 20%
            // * Hard multiplies the current interval by the hard interval (1.2 by default) and
            //   decreases the ease by 15%
            // * Good multiplies the current interval by the ease
            // * Easy multiplies the current interval by the ease times the easy bonus (1.3 by
            //   default) and increases the ease by 15%
            let (new_ease, new_review_count) = match score {
                Difficulty::Again => {
                    (self.ease - 0.2, 0)
                },
                Difficulty::Hard => {
                    (self.ease - 0.15, self.review_count + 1)
                },
                Difficulty::Good => {
                    (self.ease, self.review_count + 1)
                },
                Difficulty::Easy => {
                    (self.ease + 0.15, self.review_count + 1)
                },
            };

            let new_due = time_now + new_interval;

            self.interval = new_interval;
            self.due = new_due.fixed_offset();
            self.ease = f64::max(MINIMUM_EASE, new_ease);
            self.review_count = new_review_count;
        }
    }

    /// Multiply a duration by a float.
    fn mul_duration(duration: Duration, multiplier: f64) -> Duration {
        let new_interval_secs = duration.num_seconds() as f64 * multiplier;
        Duration::seconds(new_interval_secs as i64)
    }

    /// Get whether the card is due.
    pub fn is_due(&self) -> bool {
        let due_time = Self::due_time().unwrap();
        (self.due - due_time).num_seconds() <= 0
    }

    /// Get a human readable time until due.
    pub fn human_readable_due(&self) -> String {
        let time_until_due = self.due - Local::now().fixed_offset();
        crate::util::review_duration_to_human(&time_until_due)
    }
}
