use chrono::{DateTime, TimeZone, Utc};

fn f64_to_datetime_utc(timestamp: f64) -> Option<DateTime<Utc>> {
    // Convert the f64 timestamp to seconds and nanoseconds
    let seconds = timestamp.trunc() as i64;
    let nanoseconds = ((timestamp.fract() * 1_000_000_000.0) as u32).max(0);
    
    // Create a NaiveDateTime from seconds and nanoseconds
    return DateTime::from_timestamp(seconds, nanoseconds);

}