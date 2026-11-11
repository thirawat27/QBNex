//! Date and time: Unix milliseconds (UTC) as the single representation,
//! ISO 8601 / RFC 3339 text, strftime-like formatting and parsing, local
//! time through the operating system's time zone.

use chrono::format::{Item, StrftimeItems};
use chrono::{DateTime, Datelike, FixedOffset, Local, NaiveDate, NaiveDateTime, TimeZone, Timelike, Utc};
use std::fmt::Write;

/// Current time in milliseconds since 1970-01-01T00:00:00Z.
pub fn now_ms() -> f64 {
    Utc::now().timestamp_millis() as f64
}

fn utc(ms: f64) -> Option<DateTime<Utc>> {
    if !ms.is_finite() {
        return None;
    }
    DateTime::from_timestamp_millis(ms.floor() as i64)
}

/// Offset of local time from UTC at `ms`, in seconds (east positive).
pub fn local_offset_seconds(ms: f64) -> Option<i32> {
    utc(ms).map(|t| t.with_timezone(&Local).offset().local_minus_utc())
}

fn fixed(ms: f64, local: bool) -> Option<DateTime<FixedOffset>> {
    let t = utc(ms)?;
    Some(if local { t.with_timezone(&Local).fixed_offset() } else { t.fixed_offset() })
}

/// ISO 8601 text with milliseconds: `2024-03-05T07:08:09.010Z` (UTC) or
/// `2024-03-05T14:08:09.010+07:00` (local).
pub fn iso(ms: f64, local: bool) -> Option<String> {
    let t = fixed(ms, local)?;
    Some(if local {
        t.format("%Y-%m-%dT%H:%M:%S%.3f%:z").to_string()
    } else {
        t.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
    })
}

/// strftime-like formatting (chrono syntax: `%Y-%m-%d %H:%M:%S`, `%A`, `%B`,
/// `%j`, `%3f`, `%z`, `%%`, ...). `Err` for an invalid format.
pub fn format(ms: f64, fmt: &str, local: bool) -> Result<String, String> {
    let t = fixed(ms, local).ok_or("time out of range")?;
    let items: Vec<Item> = StrftimeItems::new(fmt).collect();
    if items.iter().any(|i| matches!(i, Item::Error)) {
        return Err(format!("invalid format: {fmt}"));
    }
    let mut s = String::new();
    write!(s, "{}", t.format_with_items(items.into_iter())).map_err(|_| format!("cannot format with: {fmt}"))?;
    Ok(s)
}

/// Parses text. With an empty `fmt`: RFC 3339 / ISO 8601 (with offset, or
/// `Z`; without an offset the time is UTC, or local with `local`), a plain
/// date `2024-03-05` (midnight) also works. Otherwise `fmt` is a chrono
/// strftime pattern (a pattern without an offset reads UTC or local time).
pub fn parse(text: &str, fmt: &str, local: bool) -> Result<f64, String> {
    let text = text.trim();
    let naive = if fmt.is_empty() {
        if let Ok(t) = DateTime::parse_from_rfc3339(text) {
            return Ok(t.timestamp_millis() as f64);
        }
        let t2 = text.replacen(' ', "T", 1);
        if let Ok(t) = DateTime::parse_from_rfc3339(&t2) {
            return Ok(t.timestamp_millis() as f64);
        }
        if let Ok(n) = NaiveDateTime::parse_from_str(&t2, "%Y-%m-%dT%H:%M:%S%.f") {
            n
        } else if let Ok(n) = NaiveDateTime::parse_from_str(&t2, "%Y-%m-%dT%H:%M") {
            n
        } else if let Ok(d) = NaiveDate::parse_from_str(text, "%Y-%m-%d") {
            d.and_hms_opt(0, 0, 0).ok_or("bad date")?
        } else {
            return Err(format!("not an ISO 8601 date/time: {text}"));
        }
    } else {
        if let Ok(t) = DateTime::parse_from_str(text, fmt) {
            return Ok(t.timestamp_millis() as f64);
        }
        match NaiveDateTime::parse_from_str(text, fmt) {
            Ok(n) => n,
            Err(_) => match NaiveDate::parse_from_str(text, fmt) {
                Ok(d) => d.and_hms_opt(0, 0, 0).ok_or("bad date")?,
                Err(e) => return Err(format!("{e}: {text}")),
            },
        }
    };
    to_ms(naive, local)
}

fn to_ms(n: NaiveDateTime, local: bool) -> Result<f64, String> {
    if local {
        // The earlier of two candidates in a DST overlap, an error in a gap.
        Local.from_local_datetime(&n).earliest().map(|t| t.timestamp_millis() as f64).ok_or_else(|| "time does not exist in the local time zone".into())
    } else {
        Ok(n.and_utc().timestamp_millis() as f64)
    }
}

/// Builds a time from calendar parts (month 1-12). Out-of-range parts are
/// an error (no rolling over).
#[allow(clippy::too_many_arguments)]
pub fn make(year: i32, month: u32, day: u32, hour: u32, minute: u32, second: u32, millis: u32, local: bool) -> Result<f64, String> {
    let n = NaiveDate::from_ymd_opt(year, month, day)
        .and_then(|d| d.and_hms_milli_opt(hour, minute, second, millis))
        .ok_or_else(|| format!("invalid date/time {year}-{month}-{day} {hour}:{minute}:{second}.{millis}"))?;
    to_ms(n, local)
}

/// Calendar parts, numbered as in the C ABI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Year = 1,
    Month = 2,
    Day = 3,
    Hour = 4,
    Minute = 5,
    Second = 6,
    Millisecond = 7,
    /// 0 = Sunday ... 6 = Saturday.
    Weekday = 8,
    /// 1 ..= 366.
    YearDay = 9,
    /// ISO 8601 week number.
    IsoWeek = 10,
}

impl Part {
    pub fn from_code(c: i32) -> Option<Self> {
        use Part::*;
        Some(match c {
            1 => Year,
            2 => Month,
            3 => Day,
            4 => Hour,
            5 => Minute,
            6 => Second,
            7 => Millisecond,
            8 => Weekday,
            9 => YearDay,
            10 => IsoWeek,
            _ => return None,
        })
    }
}

/// One calendar part of `ms` (UTC or local).
pub fn part(ms: f64, p: Part, local: bool) -> Option<i64> {
    let t = fixed(ms, local)?;
    Some(match p {
        Part::Year => i64::from(t.year()),
        Part::Month => i64::from(t.month()),
        Part::Day => i64::from(t.day()),
        Part::Hour => i64::from(t.hour()),
        Part::Minute => i64::from(t.minute()),
        Part::Second => i64::from(t.second()),
        Part::Millisecond => i64::from(t.timestamp_subsec_millis()),
        Part::Weekday => i64::from(t.weekday().num_days_from_sunday()),
        Part::YearDay => i64::from(t.ordinal()),
        Part::IsoWeek => i64::from(t.iso_week().week()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: f64 = 1_709_622_489_010.0; // 2024-03-05T07:08:09.010Z (Tuesday)

    #[test]
    fn iso_format_parts() {
        assert_eq!(iso(T, false).unwrap(), "2024-03-05T07:08:09.010Z");
        assert_eq!(iso(-1.0, false).unwrap(), "1969-12-31T23:59:59.999Z");
        assert_eq!(format(T, "%A %d %B %Y, %H:%M:%S.%3f day %j week %V %%", false).unwrap(), "Tuesday 05 March 2024, 07:08:09.010 day 065 week 10 %");
        assert!(format(T, "%Q", false).is_err());
        assert_eq!(part(T, Part::Weekday, false), Some(2));
        assert_eq!(part(T, Part::YearDay, false), Some(65));
        assert_eq!(part(T, Part::Millisecond, false), Some(10));
        assert_eq!(part(f64::NAN, Part::Year, false), None);
        assert!(iso(T, true).is_some());
        assert!(local_offset_seconds(T).is_some());
    }

    #[test]
    fn parse_and_make() {
        assert_eq!(parse("2024-03-05T07:08:09.010Z", "", false), Ok(T));
        assert_eq!(parse("2024-03-05T14:08:09.010+07:00", "", false), Ok(T));
        assert_eq!(parse("2024-03-05 07:08:09.010", "", false), Ok(T));
        assert_eq!(parse("2024-03-05", "", false), Ok(1_709_596_800_000.0));
        assert_eq!(parse("05/03/2024 07:08", "%d/%m/%Y %H:%M", false), Ok(1_709_622_480_000.0));
        assert_eq!(parse("05.03.2024", "%d.%m.%Y", false), Ok(1_709_596_800_000.0));
        assert!(parse("yesterday", "", false).is_err());
        assert_eq!(make(2024, 3, 5, 7, 8, 9, 10, false), Ok(T));
        assert_eq!(make(1900, 1, 1, 0, 0, 0, 0, false), Ok(-2_208_988_800_000.0));
        assert!(make(2023, 2, 29, 0, 0, 0, 0, false).is_err());
        let local = make(2024, 3, 5, 7, 8, 9, 10, true).unwrap();
        assert_eq!(local + f64::from(local_offset_seconds(local).unwrap()) * 1000.0, T);
    }
}
