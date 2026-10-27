//! The injected environment: the clock and cancellation.
//!
//! This is the only module in the core that may touch `std::time::SystemTime`. Everything
//! else takes a [`Clock`] so tests can fix time and output stays deterministic.

use std::fmt;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// An instant in UTC with millisecond precision.
///
/// Printed and parsed as RFC 3339 with exactly three fractional digits and a `Z` suffix,
/// for example `2026-01-01T00:00:00.000Z`. Parsing accepts any RFC 3339 timestamp (any
/// number of fractional digits, a `Z` or a numeric offset) and converts it to UTC.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp {
    unix_ms: i64,
}

/// The error returned when a string is not an RFC 3339 timestamp.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimestampParseError {
    input: String,
}

impl fmt::Display for TimestampParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "'{}' is not an RFC 3339 timestamp", self.input)
    }
}

impl std::error::Error for TimestampParseError {}

impl Timestamp {
    /// Milliseconds since the Unix epoch, UTC.
    pub const fn from_unix_ms(unix_ms: i64) -> Self {
        Timestamp { unix_ms }
    }

    /// Milliseconds since the Unix epoch, UTC.
    pub const fn unix_ms(self) -> i64 {
        self.unix_ms
    }

    /// Builds an instant from a proleptic Gregorian date and a UTC time of day.
    ///
    /// Returns `None` when any field is out of range (the day is checked against the month,
    /// leap years included).
    pub fn from_civil(
        year: i64,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
        millisecond: u32,
    ) -> Option<Self> {
        if !(1..=12).contains(&month)
            || day == 0
            || day > days_in_month(year, month)
            || hour > 23
            || minute > 59
            || second > 59
            || millisecond > 999
        {
            return None;
        }
        let days = days_from_civil(year, month, day);
        let seconds =
            days * 86_400 + i64::from(hour) * 3_600 + i64::from(minute) * 60 + i64::from(second);
        Some(Timestamp {
            unix_ms: seconds * 1_000 + i64::from(millisecond),
        })
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let seconds = self.unix_ms.div_euclid(1_000);
        let millis = self.unix_ms.rem_euclid(1_000);
        let days = seconds.div_euclid(86_400);
        let day_seconds = seconds.rem_euclid(86_400);
        let (year, month, day) = civil_from_days(days);
        write!(
            f,
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
            year,
            month,
            day,
            day_seconds / 3_600,
            (day_seconds % 3_600) / 60,
            day_seconds % 60,
            millis
        )
    }
}

impl FromStr for Timestamp {
    type Err = TimestampParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_rfc3339(s).ok_or_else(|| TimestampParseError {
            input: s.to_string(),
        })
    }
}

fn parse_rfc3339(s: &str) -> Option<Timestamp> {
    let b = s.as_bytes();
    // Shortest valid form: YYYY-MM-DDTHH:MM:SSZ (20 bytes).
    if b.len() < 20 || !s.is_ascii() {
        return None;
    }
    let num = |from: usize, to: usize| -> Option<u32> {
        if b[from..to].iter().all(u8::is_ascii_digit) {
            s[from..to].parse().ok()
        } else {
            None
        }
    };
    if b[4] != b'-'
        || b[7] != b'-'
        || !matches!(b[10], b'T' | b't')
        || b[13] != b':'
        || b[16] != b':'
    {
        return None;
    }
    let year = i64::from(num(0, 4)?);
    let month = num(5, 7)?;
    let day = num(8, 10)?;
    let hour = num(11, 13)?;
    let minute = num(14, 16)?;
    let second = num(17, 19)?;
    let mut pos = 19;
    let mut millisecond = 0;
    if b[pos] == b'.' {
        let start = pos + 1;
        let mut end = start;
        while end < b.len() && b[end].is_ascii_digit() {
            end += 1;
        }
        if end == start {
            return None;
        }
        // Keep the first three fractional digits, padding a short fraction with zeros.
        let mut digits = s[start..end].to_string();
        digits.truncate(3);
        while digits.len() < 3 {
            digits.push('0');
        }
        millisecond = digits.parse().ok()?;
        pos = end;
    }
    let offset_minutes: i64 = match b.get(pos)? {
        b'Z' | b'z' if pos + 1 == b.len() => 0,
        b'+' | b'-' if pos + 6 == b.len() && b[pos + 3] == b':' => {
            let hours = num(pos + 1, pos + 3)?;
            let minutes = num(pos + 4, pos + 6)?;
            if hours > 23 || minutes > 59 {
                return None;
            }
            let total = i64::from(hours) * 60 + i64::from(minutes);
            if b[pos] == b'-' { -total } else { total }
        }
        _ => return None,
    };
    let local = Timestamp::from_civil(year, month, day, hour, minute, second, millisecond)?;
    Some(Timestamp::from_unix_ms(
        local.unix_ms - offset_minutes * 60_000,
    ))
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// Days since 1970-01-01 for a proleptic Gregorian date (Howard Hinnant's `days_from_civil`).
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let m = i64::from(month);
    let d = i64::from(day);
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The inverse of [`days_from_civil`] (Howard Hinnant's `civil_from_days`).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// The source of the current time. Core code never calls `SystemTime::now` directly.
pub trait Clock: Send + Sync {
    /// The current instant in UTC with millisecond precision.
    fn now_utc(&self) -> Timestamp;
}

/// The real clock, for the app and the CLI in normal use.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_utc(&self) -> Timestamp {
        let since_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        Timestamp::from_unix_ms(i64::try_from(since_epoch.as_millis()).unwrap_or(i64::MAX))
    }
}

/// A clock that always answers the same instant, for tests and golden cases.
#[derive(Clone, Copy, Debug)]
pub struct FixedClock(pub Timestamp);

impl Clock for FixedClock {
    fn now_utc(&self) -> Timestamp {
        self.0
    }
}

/// A cancellation flag shared between the caller and a running job.
///
/// Clones share the same flag, so the UI keeps one clone and the job polls another.
#[derive(Clone, Debug, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    /// A token that is not cancelled.
    pub fn new() -> Self {
        Self::default()
    }

    /// Requests cancellation. Every clone observes it.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    /// Whether cancellation was requested on this token or any of its clones.
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_prints_as_rfc3339_with_milliseconds() {
        assert_eq!(
            Timestamp::from_unix_ms(0).to_string(),
            "1970-01-01T00:00:00.000Z"
        );
    }

    #[test]
    fn known_instant_round_trips() {
        // 2026-01-01T00:00:00Z is 20,454 days after the epoch (56 years, 14 of them leap).
        let t: Timestamp = "2026-01-01T00:00:00.000Z".parse().unwrap();
        assert_eq!(t.unix_ms(), 20_454 * 86_400 * 1_000);
        assert_eq!(t.to_string(), "2026-01-01T00:00:00.000Z");
        assert_eq!(Timestamp::from_civil(2026, 1, 1, 0, 0, 0, 0), Some(t));
    }

    #[test]
    fn fractions_and_offsets_convert_to_utc() {
        let with_offset: Timestamp = "2026-06-13T15:02:11.34+01:00".parse().unwrap();
        assert_eq!(with_offset.to_string(), "2026-06-13T14:02:11.340Z");
        let negative: Timestamp = "2026-06-13T15:02:11-02:30".parse().unwrap();
        assert_eq!(negative.to_string(), "2026-06-13T17:32:11.000Z");
        let long_fraction: Timestamp = "2026-06-13T15:02:11.123456789Z".parse().unwrap();
        assert_eq!(long_fraction.to_string(), "2026-06-13T15:02:11.123Z");
    }

    #[test]
    fn leap_day_and_year_boundaries() {
        assert!(Timestamp::from_civil(2024, 2, 29, 0, 0, 0, 0).is_some());
        assert!(Timestamp::from_civil(2026, 2, 29, 0, 0, 0, 0).is_none());
        assert!(Timestamp::from_civil(2000, 2, 29, 0, 0, 0, 0).is_some());
        assert!(Timestamp::from_civil(1900, 2, 29, 0, 0, 0, 0).is_none());
        let before_epoch: Timestamp = "1969-12-31T23:59:59.999Z".parse().unwrap();
        assert_eq!(before_epoch.unix_ms(), -1);
        assert_eq!(before_epoch.to_string(), "1969-12-31T23:59:59.999Z");
    }

    #[test]
    fn invalid_strings_are_refused() {
        for bad in [
            "",
            "2026-01-01",
            "2026-01-01T00:00:00",
            "2026-13-01T00:00:00Z",
            "2026-01-01T24:00:00Z",
            "2026-01-01T00:00:00.Z",
            "2026-01-01T00:00:00+25:00",
            "2026-01-01 00:00:00Z",
            "yesterday",
        ] {
            let parsed: Result<Timestamp, _> = bad.parse();
            assert!(parsed.is_err(), "{bad:?} should not parse");
        }
        let err = "yesterday".parse::<Timestamp>().unwrap_err();
        assert_eq!(err.to_string(), "'yesterday' is not an RFC 3339 timestamp");
    }

    #[test]
    fn fixed_clock_returns_its_instant() {
        let instant: Timestamp = "2026-01-01T00:00:00.000Z".parse().unwrap();
        let clock = FixedClock(instant);
        assert_eq!(clock.now_utc(), instant);
        assert_eq!(clock.now_utc().to_string(), "2026-01-01T00:00:00.000Z");
    }

    #[test]
    fn system_clock_is_after_the_plan_was_written() {
        let now = SystemClock.now_utc();
        let written: Timestamp = "2026-01-01T00:00:00.000Z".parse().unwrap();
        assert!(now > written);
    }

    #[test]
    fn cancel_token_clones_share_state() {
        let token = CancelToken::new();
        let clone = token.clone();
        assert!(!token.is_cancelled());
        assert!(!clone.is_cancelled());
        clone.cancel();
        assert!(token.is_cancelled());
        assert!(clone.is_cancelled());
        let fresh = CancelToken::default();
        assert!(!fresh.is_cancelled());
    }
}
