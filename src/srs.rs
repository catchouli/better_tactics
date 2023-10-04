use std::error::Error;
use lazy_static::lazy_static;
use chrono::{DateTime, FixedOffset, Local, Duration};

lazy_static! {
    /// The initial intervals for new cards
    pub static ref INITIAL_INTERVALS: [Duration; 2] = [
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

impl Difficulty {
    pub fn to_i64(&self) -> i64 {
        match self {
            Self::Again => 0,
            Self::Hard => 1,
            Self::Good => 2,
            Self::Easy => 3
        }
    }

    pub fn from_i64(value: i64) -> SrsResult<Self> {
        Ok(match value {
            0 => Self::Again,
            1 => Self::Hard,
            2 => Self::Good,
            3 => Self::Easy,
            _ => Err("")?
        })
    }

    /// The score for a review, for the rating system. A score of 0.0 represents a loss, 0.5
    /// represents a draw, and 1.0 represents a win.
    pub fn score(&self) -> f64 {
        match self {
            Self::Again => 0.0,
            Self::Hard => 0.5,
            // Experimentally determined to lead to good rating growth if a puzzle is around the
            // user's level. 'Easy' reviews are determined to be a win, and 'Hard' reviews a draw,
            // but 'Good' reviews are somewhere in between. This score typically gives around ~5-6
            // points for completing a puzzle at your level.
            Self::Good => 0.66,
            Self::Easy => 1.0
        }
    }
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
    pub fn in_learning(&self) -> bool {
        self.learning_stage < INITIAL_INTERVALS.len() as i64 &&
        self.interval <= INITIAL_INTERVALS[self.learning_stage as usize]
    }

    /// Get the next interval after a review with score `score`.
    pub fn next_interval(&self, score: Difficulty) -> Duration {
        // If the card is still in learning, use the initial learning stages.
        let is_learning = self.in_learning();

        // Scores of 'again' should always reset the interval to default.
        if score == Difficulty::Again {
            INITIAL_INTERVALS[0]
        }
        // Scores of 'hard' should stop the interval from growing, but shouldn't ever be any less
        // than a score of 'again' would result in.
        else if score == Difficulty::Hard {
            self.interval.max(self.next_interval(Difficulty::Again))
        }
        // Scores of 'good' should have the normal growth.
        else if score == Difficulty::Good {
            if is_learning {
                INITIAL_INTERVALS[self.learning_stage as usize]
            }
            else {
                Self::mul_duration(self.interval, self.ease)
                    .max(*INITIAL_INTERVALS.last().unwrap())
                    .max(self.next_interval(Difficulty::Hard))
            }
        }
        // Scores of 'easy' should apply the easy growth bonus applied, and cards that are in
        // learning should immediately leave learning.
        else if score == Difficulty::Easy {
            Self::mul_duration(self.interval, self.ease * EASY_BONUS)
                .max(*INITIAL_INTERVALS.last().unwrap())
                .max(self.next_interval(Difficulty::Good))
        }
        else {
            panic!("Missing difficulty")
        }
    }

    /// Get the next interval after a review with score `score` in human readable form.
    pub fn next_interval_human(&self, score: Difficulty) -> String {
        crate::util::review_duration_to_human(self.next_interval(score))
    }

    /// Review a card and update the interval, ease and due date.
    pub fn review(&mut self, time_now: DateTime<FixedOffset>, score: Difficulty) {
        // Update interval and due time.
        self.interval = self.next_interval(score);
        self.due = time_now + self.interval;

        // Update learning stage, it should increase by one each time it's reviewed until it's no
        // longer in learning. Difficulty::Again should send any card back to learning stage 0, but
        // Difficulty::Easy should remove any card from learning.
        if score == Difficulty::Again {
            self.learning_stage = 0;
        }
        else if self.in_learning() {
            if score == Difficulty::Easy {
                self.learning_stage = INITIAL_INTERVALS.len() as i64;
            }
            else {
                self.learning_stage += 1;
            }
        }

        // Update ease according to difficulty.
        self.ease = f64::max(MINIMUM_EASE, match score {
            Difficulty::Again => self.ease - 0.2,
            Difficulty::Hard => self.ease - 0.15,
            Difficulty::Good => self.ease,
            Difficulty::Easy => self.ease + 0.15,
        });

        // Update review count.
        self.review_count += 1;
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
        crate::util::review_duration_to_human(time_until_due)
    }
}
