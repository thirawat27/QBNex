//! Date and time: Unix milliseconds (UTC) as the single representation,
//! ISO 8601 / RFC 3339 text, strftime-like formatting and parsing, local
//! time through the operating system's time zone.

use jiff::civil::{DateTime, Time};
use jiff::fmt::strtime;
use jiff::tz::{AmbiguousOffset, TimeZone};
use jiff::{Timestamp, ZonedDateTime};

/// Current time in milliseconds since 1970-01-01T00:00:00Z.
pub fn now_ms() -> f64 {
    Timestamp::now().as_millisecond() as f64
}

/// `ms` as an instant; `None` when it is not finite or outside the years
/// -9999 ..= 9999.
fn utc(ms: f64) -> Option<Timestamp> {
    if !ms.is_finite() {
        return None;
    }
    Timestamp::from_millisecond(ms.floor() as i64).ok()
}

fn zone(local: bool) -> TimeZone {
    if local { TimeZone::system() } else { TimeZone::UTC }
}

/// Offset of local time from UTC at `ms`, in seconds (east positive).
pub fn local_offset_seconds(ms: f64) -> Option<i32> {
    utc(ms).map(|t| TimeZone::system().to_offset(t).seconds())
}

fn zoned(ms: f64, local: bool) -> Option<ZonedDateTime> {
    Some(utc(ms)?.to_zoned_datetime(zone(local)))
}

/// ISO 8601 text with milliseconds: `2024-03-05T07:08:09.010Z` (UTC) or
/// `2024-03-05T14:08:09.010+07:00` (local).
pub fn iso(ms: f64, local: bool) -> Option<String> {
    let t = zoned(ms, local)?;
    strtime::format(if local { "%Y-%m-%dT%H:%M:%S%.3f%:z" } else { "%Y-%m-%dT%H:%M:%S%.3fZ" }, &t).ok()
}

/// strftime-like formatting (`%Y-%m-%d %H:%M:%S`, `%A`, `%B`, `%j`, `%3f`,
/// `%z`, `%%`, ...). `Err` for an invalid format.
pub fn format(ms: f64, fmt: &str, local: bool) -> Result<String, String> {
    let t = zoned(ms, local).ok_or("time out of range")?;
    strtime::format(fmt, &t).map_err(|_| format!("invalid format: {fmt}"))
}

/// Parses text. With an empty `fmt`: RFC 3339 / ISO 8601 (with offset, or
/// `Z`; without an offset the time is UTC, or local with `local`), a plain
/// date `2024-03-05` (midnight) also works. Otherwise `fmt` is a strftime
/// pattern (a pattern without an offset reads UTC or local time).
pub fn parse(text: &str, fmt: &str, local: bool) -> Result<f64, String> {
    let text = text.trim();
    let civil = if fmt.is_empty() {
        if let Ok(t) = text.parse::<Timestamp>() {
            return Ok(t.as_millisecond() as f64);
        }
        text.parse::<DateTime>().map_err(|_| format!("not an ISO 8601 date/time: {text}"))?
    } else {
        let parts = strtime::parse(fmt, text).map_err(|e| format!("{e}: {text}"))?;
        if let Ok(t) = parts.to_timestamp() {
            return Ok(t.as_millisecond() as f64);
        }
        match parts.to_datetime() {
            Ok(dt) => dt,
            Err(_) => parts.to_date().map(|d| d.to_datetime(Time::midnight())).map_err(|e| format!("{e}: {text}"))?,
        }
    };
    to_ms(civil, local)
}

fn to_ms(dt: DateTime, local: bool) -> Result<f64, String> {
    let zone = zone(local);
    let t = zone.to_ambiguous_timestamp(dt);
    // The earlier of two candidates in a DST overlap, an error in a gap.
    if matches!(t.offset(), AmbiguousOffset::Gap { .. }) {
        return Err("time does not exist in the local time zone".into());
    }
    t.earlier().map(|t| t.as_millisecond() as f64).map_err(|e| e.to_string())
}

/// Builds a time from calendar parts (month 1-12). Out-of-range parts are
/// an error (no rolling over).
#[allow(clippy::too_many_arguments)]
pub fn make(year: i32, month: u32, day: u32, hour: u32, minute: u32, second: u32, millis: u32, local: bool) -> Result<f64, String> {
    let bad = || format!("invalid date/time {year}-{month}-{day} {hour}:{minute}:{second}.{millis}");
    let small = |v: u32| i8::try_from(v).map_err(|_| bad());
    let year = i16::try_from(year).map_err(|_| bad())?;
    let nanos = i32::try_from(millis).ok().and_then(|m| m.checked_mul(1_000_000)).ok_or_else(bad)?;
    let dt = DateTime::new(year, small(month)?, small(day)?, small(hour)?, small(minute)?, small(second)?, nanos).map_err(|_| bad())?;
    to_ms(dt, local)
}

/// Calendar parts, numbered as in the export interface.
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
    let t = zoned(ms, local)?;
    Some(match p {
        Part::Year => i64::from(t.year()),
        Part::Month => i64::from(t.month()),
        Part::Day => i64::from(t.day()),
        Part::Hour => i64::from(t.hour()),
        Part::Minute => i64::from(t.minute()),
        Part::Second => i64::from(t.second()),
        Part::Millisecond => i64::from(t.millisecond()),
        Part::Weekday => i64::from(t.weekday().to_sunday_zero_offset()),
        Part::YearDay => i64::from(t.day_of_year()),
        Part::IsoWeek => i64::from(t.iso_week_date().week()),
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
        assert!(format(T, "%v", false).is_err());
        assert!(format(T, "abc%", false).is_err());
        assert_eq!(part(T, Part::Weekday, false), Some(2));
        assert_eq!(part(T, Part::YearDay, false), Some(65));
        assert_eq!(part(T, Part::Millisecond, false), Some(10));
        assert_eq!(part(f64::NAN, Part::Year, false), None);
        assert!(iso(T, true).is_some());
        assert!(local_offset_seconds(T).is_some());
    }

    /// Every format code the standard library documents (compiler/stdlib/datetime.bas).
    #[test]
    fn documented_format_codes() {
        let f = |s: &str| format(T, s, false).unwrap();
        assert_eq!(f("%Y-%m-%d %H %I %M %S %3f %p"), "2024-03-05 07 07 08 09 010 AM");
        assert_eq!(f("%A %a %B %b %j %V"), "Tuesday Tue March Mar 065 10");
        assert_eq!(f("%z %:z %s"), "+0000 +00:00 1709622489");
        assert_eq!(f("%F %T %%"), "2024-03-05 07:08:09 %");
        assert_eq!(format(T + 12.0 * 3_600_000.0, "%I %p", false).unwrap(), "07 PM");
        // the representable years are -9999 ..= 9999
        assert_eq!(iso(1e18, false), None);
    }

    #[test]
    fn parse_and_make() {
        assert_eq!(parse("2024-03-05T07:08:09.010Z", "", false), Ok(T));
        assert_eq!(parse("2024-03-05T14:08:09.010+07:00", "", false), Ok(T));
        assert_eq!(parse("2024-03-05 07:08:09.010", "", false), Ok(T));
        assert_eq!(parse("2024-03-05T07:08", "", false), Ok(T - 9_010.0));
        assert_eq!(parse("2024-03-05", "", false), Ok(1_709_596_800_000.0));
        assert_eq!(parse("05/03/2024 07:08", "%d/%m/%Y %H:%M", false), Ok(1_709_622_480_000.0));
        assert_eq!(parse("05.03.2024", "%d.%m.%Y", false), Ok(1_709_596_800_000.0));
        assert_eq!(parse("05.03.2024 14:08 +0700", "%d.%m.%Y %H:%M %z", false), Ok(1_709_622_480_000.0));
        assert!(parse("yesterday", "", false).is_err());
        assert!(parse("31.02.2024", "%d.%m.%Y", false).is_err());
        assert_eq!(make(2024, 3, 5, 7, 8, 9, 10, false), Ok(T));
        assert_eq!(make(1900, 1, 1, 0, 0, 0, 0, false), Ok(-2_208_988_800_000.0));
        assert!(make(2023, 2, 29, 0, 0, 0, 0, false).is_err());
        assert!(make(2024, 3, 5, 7, 8, 9, 1000, false).is_err());
        let local = make(2024, 3, 5, 7, 8, 9, 10, true).unwrap();
        assert_eq!(local + f64::from(local_offset_seconds(local).unwrap()) * 1000.0, T);
    }
}