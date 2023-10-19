use std::error::Error;
use lazy_static::lazy_static;
use chrono::{DateTime, FixedOffset, Duration, NaiveTime, Timelike};
use crate::time::TimeProvider;

lazy_static! {
    /// The initial intervals for new cards
    pub static ref INITIAL_INTERVALS: [Duration; 2] = [
        Duration::seconds(10 * 60),
        Duration::seconds(24 * 60 * 60),
    ];
    
    /// Max interval to stop our intervals getting insane if somebody chooses to just review the
    /// same card with 'easy' over and over... (50k weeks = roughly 1000 years)
    static ref MAX_INTERVAL: Duration = Duration::weeks(52179);

    /// Minimum interval for 'easy' reviews. If a card is really easy it's allowed to leave
    /// learning immediately, and also gets set to this interval. This gives it a bit of a boost
    /// over just using the last learning interval, because otherwise cards marked easy might just
    /// get the same interval as 'good' the first time which is a bit weird and seems like it's
    /// generating an unnecessary number of reviews for those cards.
    static ref MIN_EASY_INTERVAL: Duration = Duration::days(4);
}

/// Spaced repetition config.
#[derive(Debug, Copy, Clone, serde::Serialize)]
pub struct SrsConfig {
    pub default_ease: f64,
    pub minimum_ease: f64,
    pub easy_bonus: f64,
}

/// The day end time. The user will be able to review-ahead cards before this time (as long as they
/// aren't in learning, in which case the interval is less than 24h, usually around 10 minutes, and
/// we want them to wait until it comes up again naturally.)
/// TODO: make this configurable.
fn day_end_time() -> NaiveTime {
    // 4am by default, like in anki.
    NaiveTime::from_hms_opt(4, 0, 0).unwrap()
}

/// Get day_end_time() as a datetime.
pub fn day_end_datetime<TP: TimeProvider>() -> DateTime<FixedOffset> {
    let day_end = day_end_time();

    // If the current time is before the DAY_END time, (e.g. because it's after midnight but
    // before 4am), we just need to get the current date and set the time to the DAY_END time.
    let now = TP::now();
    let day_end_date = if now.time() < day_end {
        now.date_naive()
    }
    // If it's after that time, we need to add one day to get to the next DAY_END time.
    else {
        (now + Duration::seconds(60 * 60 * 24)).date_naive()
    };

    day_end_date
        .and_hms_opt(day_end.hour(), day_end.minute(), day_end.second())
        .unwrap()
        .and_utc()
        .fixed_offset()
}

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

    pub fn from_i64(value: i64) -> Result<Self, Box<dyn Error>> {
        Ok(match value {
            0 => Self::Again,
            1 => Self::Hard,
            2 => Self::Good,
            3 => Self::Easy,
            _ => Err(format!("Attempted to convert invalid value to Difficulty: {value}"))?
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

impl serde::Serialize for Difficulty {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error>
    {
        serializer.serialize_i64(self.to_i64())
    }
}

/// A single spaced repetition "card" (e.g. a puzzle).
#[derive(Debug)]
pub struct Card {
    pub id: String,
    //#[serde(serialize_with = "serialize_datetime")]
    pub due: DateTime<FixedOffset>,
    //#[serde(serialize_with = "serialize_duration")]
    pub interval: Duration,
    pub review_count: i64,
    pub ease: f64,
    pub learning_stage: i64,
    pub srs_config: SrsConfig,
}

impl Card {
    pub fn new(id: &str, due: DateTime<FixedOffset>, srs_config: SrsConfig) -> Self
    {
        Self {
            id: id.to_string(),
            due,
            interval: INITIAL_INTERVALS[0],
            review_count: 0,
            ease: srs_config.default_ease,
            learning_stage: 0,
            srs_config,
        }
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
            self.interval
                .min(*MAX_INTERVAL)
                .max(self.next_interval(Difficulty::Again))
        }
        // Scores of 'good' should have the normal growth.
        else if score == Difficulty::Good {
            if is_learning {
                INITIAL_INTERVALS[self.learning_stage as usize]
            }
            else {
                Self::mul_duration(self.interval, self.ease)
                    .max(*INITIAL_INTERVALS.last().unwrap())
                    .min(*MAX_INTERVAL)
                    .max(self.next_interval(Difficulty::Hard))
            }
        }
        // Scores of 'easy' should apply the easy growth bonus applied, and cards that are in
        // learning should immediately leave learning.
        else if score == Difficulty::Easy {
            Self::mul_duration(self.interval, self.ease * self.srs_config.easy_bonus)
                .max(*MIN_EASY_INTERVAL)
                .min(*MAX_INTERVAL)
                .max(self.next_interval(Difficulty::Good))
        }
        else {
            panic!("Missing difficulty")
        }
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
        self.ease = f64::max(self.srs_config.minimum_ease, match score {
            Difficulty::Again => self.ease - 0.2,
            Difficulty::Hard => self.ease - 0.15,
            Difficulty::Easy => self.ease + 0.15,
            // A tweak to the ease. If the ease is below the initial ease, allow it to correct
            // towards the ease, but not exceed it.
            Difficulty::Good => if self.ease < self.srs_config.default_ease {
                f64::min(self.srs_config.default_ease, self.ease + 0.15)
            } else { self.ease },
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
    pub fn is_due<TP: TimeProvider>(&self) -> bool {
        let due_time = day_end_datetime::<TP>();
        (self.due - due_time).num_seconds() <= 0
    }
}

#[cfg(test)]
mod tests {
    use crate::srs::day_end_time;
    use crate::time::TestTimeProvider;
    use super::day_end_datetime;
    use chrono::{DateTime, Timelike};

    #[test]
    fn test_day_end_datetime() {
        let day_end = day_end_time();
        let expected = DateTime::parse_from_rfc3339(
            &format!("2023-10-07T{:02}:{:02}:{:02}+00:00",
                     day_end.hour(), day_end.minute(), day_end.second())
        ).unwrap();

        // Check that a regular-ish time in the middle of the day results in a day_end_datetime the
        // the following day.
        assert_eq!(day_end_datetime::<TestTimeProvider<2023, 10, 06, 09, 26, 00, 00, 00>>(), expected);

        // Check that a time just before midnight works properly.
        assert_eq!(day_end_datetime::<TestTimeProvider<2023, 10, 06, 23, 59, 59, 00, 00>>(), expected);

        // Check that a time at midnight doesn't skip to the next day or anything.
        assert_eq!(day_end_datetime::<TestTimeProvider<2023, 10, 07, 00, 00, 00, 00, 00>>(), expected);

        // Just after midnight.
        assert_eq!(day_end_datetime::<TestTimeProvider<2023, 10, 07, 00, 00, 01, 00, 00>>(), expected);

        // Just before the day_end time.
        assert_eq!(day_end_datetime::<TestTimeProvider<2023, 10, 07, 03, 59, 59, 00, 00>>(), expected);

        // Just after/at the day_end time should go to the next day.
        assert_eq!(day_end_datetime::<TestTimeProvider<2023, 10, 07, 04, 00, 00, 00, 00>>(),
            DateTime::parse_from_rfc3339("2023-10-08T04:00:00+00:00").unwrap());
    }
}
