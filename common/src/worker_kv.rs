// Function to round down a timestamp to the nearest interval
// ts: Unix timestamp (seconds since epoch)
// interval_seconds: The duration of the interval in seconds
pub fn round_down_timestamp(ts: i64, interval_seconds: i64) -> i64 {
    if interval_seconds <= 0 {
        // Avoid division by zero or negative intervals
        return ts;
    }
    // Integer division truncates towards zero.
    // (ts / interval) gives the number of full intervals since epoch.
    // Multiplying back by interval gives the timestamp at the start of the current interval.
    (ts / interval_seconds) * interval_seconds
}

// Define the time intervals as an enum
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Interval {
    Minute5,
    Minute15,
    Hour1,
    Hour4,
    Day1,
}

impl Interval {
    // Helper function to get the duration in seconds for each interval
    fn duration_seconds(&self) -> i64 {
        match self {
            Interval::Minute5 => 5 * 60,
            Interval::Minute15 => 15 * 60,
            Interval::Hour1 => 60 * 60,
            Interval::Hour4 => 4 * 60 * 60,
            Interval::Day1 => 24 * 60 * 60,
        }
    }
}

// Function to get the rounded-down key based on the current timestamp and an Interval enum
pub fn get_key_from_interval(ts: i64, interval: Interval) -> i64 {
    let interval_seconds = interval.duration_seconds();
    round_down_timestamp(ts, interval_seconds)
}
#[cfg(test)]
mod test {
    use crate::{get_key_from_interval, Interval};
    use chrono::{DateTime, Utc};

    #[test]
    fn test_get_key_from_interval() {
        // Get the current time in UTC
        let now: DateTime<Utc> = Utc::now();
        let current_ts: i64 = now.timestamp(); // Get current Unix timestamp (seconds)

        println!("Current Time: {}", now.to_rfc3339());
        println!("Current Timestamp (seconds): {}", current_ts);
        println!("---");

        // Calculate the keys using the new function and enum
        let key_5m = get_key_from_interval(current_ts, Interval::Minute5);
        let key_15m = get_key_from_interval(current_ts, Interval::Minute15);
        let key_1h = get_key_from_interval(current_ts, Interval::Hour1);
        let key_4h = get_key_from_interval(current_ts, Interval::Hour4);
        let key_1d = get_key_from_interval(current_ts, Interval::Day1);

        // --- Optional: Convert keys back to DateTime for verification ---
        let dt_5m = DateTime::from_timestamp(key_5m, 0).unwrap();
        let dt_15m = DateTime::from_timestamp(key_15m, 0).unwrap();
        let dt_1h = DateTime::from_timestamp(key_1h, 0).unwrap();
        let dt_4h = DateTime::from_timestamp(key_4h, 0).unwrap();
        let dt_1d = DateTime::from_timestamp(key_1d, 0).unwrap();

        // Print the results
        println!(" 5m Key Timestamp: {} ({})", key_5m, dt_5m.to_rfc3339());
        println!("15m Key Timestamp: {} ({})", key_15m, dt_15m.to_rfc3339());
        println!(" 1h Key Timestamp: {} ({})", key_1h, dt_1h.to_rfc3339());
        println!(" 4h Key Timestamp: {} ({})", key_4h, dt_4h.to_rfc3339());
        println!(" 1d Key Timestamp: {} ({})", key_1d, dt_1d.to_rfc3339());

        // Basic assertions (adjust expected values based on how round_down works)
        assert!(key_5m <= current_ts);
        assert!(key_15m <= current_ts);
        assert!(key_1h <= current_ts);
        assert!(key_4h <= current_ts);
        assert!(key_1d <= current_ts);

        assert_eq!(key_5m % Interval::Minute5.duration_seconds(), 0);
        assert_eq!(key_15m % Interval::Minute15.duration_seconds(), 0);
        assert_eq!(key_1h % Interval::Hour1.duration_seconds(), 0);
        assert_eq!(key_4h % Interval::Hour4.duration_seconds(), 0);
        assert_eq!(key_1d % Interval::Day1.duration_seconds(), 0);
    }
}
