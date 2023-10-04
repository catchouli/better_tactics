use chrono::{Duration, DateTime, FixedOffset, Local};

// Convert a review duration to a human readable string, or "now" if it's negative.
pub fn review_duration_to_human(duration: Duration) -> String {
    if duration.num_seconds() <= 0 {
        "now".to_string()
    }
    else if duration.num_weeks() > 0 {
        let weeks = duration.num_weeks();
        let days = duration.num_days() - weeks * 7;

        format!("{}w {}d", weeks, days)
    }
    else if duration.num_days() > 0 {
        let days = duration.num_days();
        let hours = duration.num_hours() - days * 24;

        format!("{}d {}h", days, hours)
    }
    else if duration.num_hours() > 0 {
        let hours = duration.num_hours();
        let mins = duration.num_minutes() - hours * 60;

        format!("{}h {}m", hours, mins)
    }
    else if duration.num_minutes() > 0 {
        let mins = duration.num_minutes();

        format!("{}m", mins)
    }
    else {
        let secs = duration.num_seconds();

        format!("{}s", secs)
    }
}

// Convert a review timestamp to a human readable string, or "now" if it's in the past.
pub fn review_timestamp_to_human(time: &DateTime<FixedOffset>) -> String {
    let now = Local::now().fixed_offset();
    let time_until = *time - now;

    if time_until.num_seconds() < 0 {
        "now".to_string()
    }
    else {
        review_duration_to_human(time_until)
    }
}

/// A convenience version of review_timestamp_to_human allows templates to call it directly on an
/// Option<DateTime>.
pub fn maybe_review_timestamp_to_human(time: &Option<DateTime<FixedOffset>>) -> String {
    time.as_ref().map(review_timestamp_to_human).unwrap_or("now".to_string())
}
