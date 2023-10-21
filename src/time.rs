use chrono::{DateTime, FixedOffset, Local};

/// A trait for providing the current time to components.
pub trait TimeProvider {
    type DT;

    fn now() -> Self::DT;
    fn now_fixed() -> DateTime<FixedOffset>;
    fn now_local() -> DateTime<Local>;
}

/// A simple time provider that provides the current local time.
#[derive(Debug)]
pub struct LocalTimeProvider {}

impl TimeProvider for LocalTimeProvider {
    type DT = DateTime<Local>;

    fn now() -> Self::DT {
        Local::now()
    }

    fn now_fixed() -> DateTime<FixedOffset> {
        Self::now().fixed_offset()
    }

    fn now_local() -> DateTime<Local> {
        Self::now()
    }
}

/// A TimeProvider that provides a constant time for use in unit tests.
#[derive(Debug)]
pub struct TestTimeProvider<const YEAR: i32, const MONTH: i32, const DAY: i32, const HOUR: i32,
    const MIN: i32, const SEC: i32, const OFFSET_HOUR: i32, const OFFSET_MIN: i32> {}

impl<const YEAR: i32, const MONTH: i32, const DAY: i32, const HOUR: i32, const MIN: i32, const SEC: i32,
     const OFFSET_HOUR: i32, const OFFSET_MIN: i32> TimeProvider
for TestTimeProvider<YEAR, MONTH, DAY, HOUR, MIN, SEC, OFFSET_HOUR, OFFSET_MIN>
{
    type DT = DateTime<FixedOffset>;

    fn now() -> DateTime<FixedOffset> {
        // If the OFFSET_HOUR is negative we need to prefix the tz offset with a -, otherwise a +.
        let (tz_prefix, offset_hour) = if OFFSET_HOUR < 0 {
            ("-", OFFSET_HOUR.abs())
        } else {
            ("+", OFFSET_HOUR)
        };

        // Format datetime to an rfc3339 string.
        let rfc3339_time = format!(
            "{YEAR}-{MONTH:02}-{DAY:02}T{HOUR:02}:{MIN:02}:{SEC:02}{tz_prefix}{offset_hour:02}:{OFFSET_MIN:02}");

        match DateTime::parse_from_rfc3339(&rfc3339_time) {
            Ok(datetime) => datetime,
            _ => panic!("TestTimeProvider failed to parse rfc3339 datetime {rfc3339_time}"),
        }
    }

    fn now_fixed() -> DateTime<FixedOffset> {
        Self::now().fixed_offset()
    }

    fn now_local() -> DateTime<Local> {
        DateTime::from(Self::now())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_now() {
        type TpPositiveOffset = TestTimeProvider<2000, 7, 14, 1, 2, 3, 4, 5>;
        type TpNegativeOffset = TestTimeProvider<2000, 7, 14, 1, 2, 3, -4, 5>;

        assert_eq!(TpPositiveOffset::now(), DateTime::parse_from_rfc3339("2000-07-14T01:02:03+04:05").unwrap());
        assert_eq!(TpNegativeOffset::now(), DateTime::parse_from_rfc3339("2000-07-14T01:02:03-04:05").unwrap());
    }
}
