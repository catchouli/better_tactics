use chrono::{Duration, DateTime, FixedOffset, Local, NaiveTime};

/// Serialize a chrono::DateTime.
pub fn _serialize_datetime<S: serde::Serializer>(dt: &DateTime<FixedOffset>, s: S)
    -> Result<S::Ok, S::Error>
{
    s.serialize_str(&dt.to_rfc3339())
}

/// Serialize a chrono::Duration.
pub fn _serialize_duration<S: serde::Serializer>(dt: &Duration, s: S)
    -> Result<S::Ok, S::Error>
{
    s.serialize_i64(dt.num_milliseconds())
}

/// Get the next time `time` occurs after `dt`.
pub fn next_time_after(dt: DateTime<Local>, time: NaiveTime) -> DateTime<Local> {
    // Get the day of the next occurence of `time`.
    let date = if dt.time() < time {
        dt.date_naive()
    }
    // If it's already after that time, we need to get the next day.
    else {
        (dt + Duration::seconds(60 * 60 * 24)).date_naive()
    };

    date.and_time(time)
        .and_local_timezone(Local)
        .latest()
        .unwrap()
}
