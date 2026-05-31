use crate::error::{HypermailError, Result};
use crate::i18n::I18n;
use chrono::{DateTime, FixedOffset, NaiveDateTime, Offset, TimeZone, Utc};

pub const MONTHS: &[&str] = &[
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

pub const DAYS: &[&str] = &["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

/// Replace English day/month abbreviations in a formatted date string with localized equivalents
/// using the locale JSON files.
pub fn localize_date_str(date_str: &str, lang: &str) -> String {
    if lang == "en" {
        return date_str.to_string();
    }
    let i18n = I18n::new(lang);
    let mut result = date_str.to_string();

    // Replace day abbreviations
    for &en_day in DAYS {
        let localized = i18n.get(en_day);
        if localized != en_day && result.contains(en_day) {
            result = result.replacen(en_day, localized, 1);
            break;
        }
    }

    // Replace month abbreviations
    for &en_month in MONTHS {
        let localized = i18n.get(en_month);
        if localized != en_month && result.contains(en_month) {
            result = result.replacen(en_month, localized, 1);
            break;
        }
    }

    result
}

pub fn is_leap(y: i32) -> bool {
    y > 1752 && (y % 4 == 0 && (y % 100 != 0 || y % 400 == 0))
}

pub fn month_from_str(s: &str) -> Option<u32> {
    MONTHS.iter().position(|&m| m.eq_ignore_ascii_case(s)).map(|p| (p + 1) as u32)
}

pub fn parse_rfc2822_date(s: &str) -> Result<i64> {
    let s = s.trim();

    let parsed = DateTime::parse_from_rfc2822(s);
    if let Ok(dt) = parsed {
        return Ok(dt.timestamp());
    }

    try_parse_flexible(s)
}

fn try_parse_flexible(s: &str) -> Result<i64> {
    let s = s.trim().to_string();

    if let Ok(dt) = DateTime::parse_from_rfc3339(&s) {
        return Ok(dt.timestamp());
    }

    let cleaned = s.replace(" (", "(").replace(") ", ")").trim().to_string();

    if let Ok(dt) = DateTime::parse_from_rfc2822(&cleaned) {
        return Ok(dt.timestamp());
    }

    // Normalize non-standard timezone suffixes like "GMT+2", "GMT-5", "UTC+3"
    // into offset form "+0200" that chrono can parse.
    let normalized = normalize_nonstandard_tz(&cleaned);
    if normalized != cleaned {
        if let Ok(dt) = DateTime::parse_from_rfc2822(&normalized) {
            return Ok(dt.timestamp());
        }
    }

    if let Ok(naive) = NaiveDateTime::parse_from_str(&cleaned, "%a %b %d %H:%M:%S %Y") {
        if let Some(local) = LocalFixOffset::local_to_fixed(naive) {
            return Ok(local.timestamp());
        }
    }

    if let Ok(naive) = chrono::NaiveDate::parse_from_str(&cleaned, "%Y-%m-%d")
        .map(|d| d.and_hms_opt(0, 0, 0).unwrap())
    {
        if let Some(local) = LocalFixOffset::local_to_fixed(naive) {
            return Ok(local.timestamp());
        }
    }

    Err(HypermailError::DateParse(format!("Cannot parse date: {}", s)))
}

/// Convert trailing `GMT+N`, `GMT-N`, `UTC+N`, `UTC-N` (and fractional-hour forms like `GMT+5:30`)
/// into an RFC 2822-compatible numeric offset (`+HHMM` / `-HHMM`).
fn normalize_nonstandard_tz(s: &str) -> String {
    // Match pattern: ...whitespace(GMT|UTC)(+|-)H or HH or H:MM or HH:MM at end
    let re = once_cell::sync::Lazy::force(&NONSTANDARD_TZ_RE);
    if let Some(cap) = re.captures(s) {
        let sign = &cap[1];
        let hours_str = &cap[2];
        let mins_str = cap.get(3).map(|m| m.as_str()).unwrap_or("00");
        let hours: i32 = hours_str.parse().unwrap_or(0);
        let mins: i32 = mins_str.parse().unwrap_or(0);
        let offset = format!("{}{:02}{:02}", sign, hours, mins);
        // Replace the matched GMT/UTC+N suffix with the numeric offset
        let match_start = cap.get(0).unwrap().start();
        // Keep everything before the GMT/UTC sign, append the offset
        let base = &s[..match_start];
        return format!("{}{}", base.trim_end(), offset.replace("+", " +").replace("-", " -"));
    }
    s.to_string()
}

static NONSTANDARD_TZ_RE: once_cell::sync::Lazy<regex::Regex> = once_cell::sync::Lazy::new(|| {
    regex::Regex::new(r"(?i)(?:GMT|UTC)([+-])(\d{1,2})(?::(\d{2}))?$").unwrap()
});

struct LocalFixOffset;

impl LocalFixOffset {
    fn local_to_fixed(naive: NaiveDateTime) -> Option<DateTime<FixedOffset>> {
        let local_offset = chrono::Local::now().offset().fix();
        match FixedOffset::east_opt(local_offset.local_minus_utc()) {
            Some(fixed) => fixed.from_local_datetime(&naive).single(),
            None => None,
        }
    }
}

/// Clamp a Unix timestamp to chrono's valid range to prevent panics.
/// chrono supports roughly -262144-01-01 to +262143-12-31, but we clamp
/// to a practical range: 0000-01-01 to 9999-12-31.
fn clamp_timestamp(timestamp: i64) -> i64 {
    const MIN_TS: i64 = -62167219200; // 0000-01-01T00:00:00Z
    const MAX_TS: i64 = 253402300799; // 9999-12-31T23:59:59Z
    timestamp.clamp(MIN_TS, MAX_TS)
}

pub fn get_date_str(
    timestamp: i64,
    fmt: Option<&str>,
    gmtime: bool,
    eurodate: bool,
    isodate: bool,
    lang: &str,
) -> String {
    let timestamp = clamp_timestamp(timestamp);
    let dt: DateTime<FixedOffset> = if gmtime {
        FixedOffset::east_opt(0).unwrap().from_utc_datetime(
            &Utc.timestamp_opt(timestamp, 0).single().unwrap_or_default().naive_utc(),
        )
    } else {
        match chrono::Local.timestamp_opt(timestamp, 0).single() {
            Some(local) => {
                let offset = *local.offset();
                let naive = local.naive_local();
                FixedOffset::east_opt(offset.local_minus_utc())
                    .unwrap()
                    .from_local_datetime(&naive)
                    .single()
                    .unwrap_or_else(|| {
                        FixedOffset::east_opt(0)
                            .unwrap()
                            .from_utc_datetime(&naive.and_utc().naive_utc())
                    })
            },
            None => {
                // Fallback for timestamps that can't be resolved to local time
                FixedOffset::east_opt(0).unwrap().from_utc_datetime(
                    &Utc.timestamp_opt(timestamp, 0).single().unwrap_or_default().naive_utc(),
                )
            },
        }
    };

    let raw = if let Some(format_str) = fmt {
        dt.format(format_str).to_string()
    } else if isodate {
        if gmtime {
            dt.format("%Y-%m-%d %H:%M:%SZ").to_string()
        } else {
            dt.format("%Y-%m-%d %H:%M:%S").to_string()
        }
    } else if eurodate {
        dt.format("%a %d %b %Y %H:%M:%S %z").to_string()
    } else {
        dt.format("%a %b %d %Y %H:%M:%S %z").to_string()
    };

    localize_date_str(&raw, lang)
}

pub fn secs_to_iso(timestamp: i64) -> String {
    let timestamp = clamp_timestamp(timestamp);
    let dt = Utc.timestamp_opt(timestamp, 0).single().unwrap_or_default();
    dt.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

pub fn iso_to_secs(s: &str) -> Result<i64> {
    let cleaned = s.trim();
    if let Ok(dt) = DateTime::parse_from_rfc3339(cleaned) {
        return Ok(dt.timestamp());
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(cleaned, "%Y-%m-%dT%H:%M:%S") {
        return Ok(dt.and_utc().timestamp());
    }
    Err(HypermailError::DateParse(format!("Cannot parse ISO date: {}", s)))
}

pub fn get_timezone_str() -> String {
    let offset = chrono::Local::now().offset().fix();
    let total_secs = offset.local_minus_utc();
    let hours = total_secs / 3600;
    let mins = (total_secs.abs() / 60) % 60;
    if total_secs >= 0 {
        format!("+{:02}{:02}", hours, mins)
    } else {
        format!("-{:02}{:02}", -hours, mins)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    #[test]
    fn test_parse_rfc2822() {
        let ts = parse_rfc2822_date("Mon, 15 Mar 2021 12:00:00 +0000").unwrap();
        assert_eq!(ts, 1615809600);

        let ts = parse_rfc2822_date("15 Mar 2021 12:00:00 +0000").unwrap();
        assert_eq!(ts, 1615809600);

        let ts = parse_rfc2822_date("Mon, 15 Mar 2021 12:00:00 GMT").unwrap();
        assert_eq!(ts, 1615809600);
    }

    #[test]
    fn test_parse_various_dates() {
        assert!(parse_rfc2822_date("Thu, 01 Jan 1998 00:00:00 +0000").is_ok());
        assert!(parse_rfc2822_date("1 Jan 1998 00:00:00 +0000").is_ok());
        assert!(parse_rfc2822_date("1 Jan 1998 00:00:00 +0100 (CET)").is_ok());
        assert!(parse_rfc2822_date("Tue, 1 Jul 2003 10:52:37 +1200").is_ok());
    }

    #[test]
    fn test_date_roundtrip() {
        let original = "Mon, 15 Mar 2021 12:00:00 +0000";
        let ts = parse_rfc2822_date(original).unwrap();
        let formatted = secs_to_iso(ts);
        assert!(formatted.starts_with("2021-03-15T12:00:00"));
    }

    #[test]
    fn test_date_str() {
        let ts = 1615809600; // 2021-03-15 12:00:00 UTC
                             // Default (US): month before day
        let ds = get_date_str(ts, None, true, false, false, "en");
        assert_eq!(ds, "Mon Mar 15 2021 12:00:00 +0000");
    }

    #[test]
    fn test_isodate_str() {
        let ts = 1615809600;
        let ds = get_date_str(ts, None, true, false, true, "en");
        assert!(ds.contains("2021-03-15"));
    }

    #[test]
    fn test_iso_roundtrip() {
        let ts = 1615809600;
        let iso = secs_to_iso(ts);
        let back = iso_to_secs(&iso).unwrap();
        assert_eq!(ts, back);
    }

    #[test]
    fn test_month_from_str() {
        assert_eq!(month_from_str("Jan"), Some(1));
        assert_eq!(month_from_str("jan"), Some(1));
        assert_eq!(month_from_str("Dec"), Some(12));
        assert_eq!(month_from_str("Foo"), None);
    }

    #[test]
    fn test_invalid_date() {
        assert!(parse_rfc2822_date("").is_err());
        assert!(parse_rfc2822_date("not a date").is_err());
    }

    #[test]
    fn test_localize_date_greek() {
        let ts = 1615809600; // Mon Mar 15 2021
        let ds = get_date_str(ts, None, true, false, false, "el");
        assert!(ds.contains("Δευ"), "Expected Greek Monday abbreviation, got: {}", ds);
        assert!(ds.contains("Μαρ"), "Expected Greek March abbreviation, got: {}", ds);
    }

    #[test]
    fn test_localize_date_german() {
        let ts = 1615809600;
        let ds = get_date_str(ts, None, true, false, false, "de");
        assert!(ds.contains("Mo"), "Expected German Monday abbreviation, got: {}", ds);
        assert!(ds.contains("Mär"), "Expected German March abbreviation, got: {}", ds);
    }

    #[test]
    fn test_localize_date_unknown_lang_falls_back() {
        let ts = 1615809600;
        let ds = get_date_str(ts, None, true, false, false, "xx");
        assert!(ds.contains("Mon"), "Unknown lang should keep English, got: {}", ds);
    }

    #[test]
    fn test_parse_gmt_plus_offset() {
        // "GMT+2" style non-standard timezone — seen in old 1996 emails
        let ts = parse_rfc2822_date("Mon, 12 Feb 1996 15:24:04 GMT+2");
        assert!(ts.is_ok(), "GMT+2 should parse successfully, got: {:?}", ts);
        let t = ts.unwrap();
        // 1996-02-12 15:24:04 at +02:00 = 13:24:04 UTC
        let dt = chrono::Utc.timestamp_opt(t, 0).unwrap();
        assert_eq!(dt.hour(), 13, "Expected 13:24 UTC, got {:?}", dt);
        assert_eq!(dt.minute(), 24);
    }

    #[test]
    fn test_parse_gmt_minus_offset() {
        let ts = parse_rfc2822_date("Wed, 01 May 1996 08:00:00 GMT-5");
        assert!(ts.is_ok(), "GMT-5 should parse, got: {:?}", ts);
        let t = ts.unwrap();
        let dt = chrono::Utc.timestamp_opt(t, 0).unwrap();
        assert_eq!(dt.hour(), 13); // 08:00 + 05:00 = 13:00 UTC
    }

    #[test]
    fn test_parse_utc_plus_fractional_offset() {
        let ts = parse_rfc2822_date("Thu, 01 Jan 1998 12:00:00 UTC+5:30");
        assert!(ts.is_ok(), "UTC+5:30 should parse, got: {:?}", ts);
        let t = ts.unwrap();
        let dt = chrono::Utc.timestamp_opt(t, 0).unwrap();
        assert_eq!(dt.hour(), 6); // 12:00 - 05:30 = 06:30 UTC
        assert_eq!(dt.minute(), 30);
    }

    #[test]
    fn test_is_leap_year() {
        assert!(is_leap(2000));
        assert!(is_leap(2004));
        assert!(!is_leap(1900));
        assert!(!is_leap(2001));
    }

    #[test]
    fn test_is_leap_before_1752() {
        // Gregorian calendar reform: years <= 1752 are always false
        assert!(!is_leap(1600));
        assert!(!is_leap(1752));
    }

    #[test]
    fn test_get_timezone_str_format() {
        let tz = get_timezone_str();
        assert!(tz.starts_with('+') || tz.starts_with('-'), "Expected +/- prefix, got: {}", tz);
        assert_eq!(tz.len(), 5, "Expected ±HHMM (5 chars), got: {}", tz);
    }

    #[test]
    fn test_eurodate_str() {
        let ts = 1615809600; // Mon Mar 15 2021 12:00:00 UTC
        let ds = get_date_str(ts, None, true, true, false, "en");
        assert_eq!(ds, "Mon 15 Mar 2021 12:00:00 +0000");
    }

    #[test]
    fn test_custom_format_str() {
        let ts = 1615809600;
        let ds = get_date_str(ts, Some("%Y-%m-%d"), true, false, false, "en");
        assert_eq!(ds, "2021-03-15");
    }
}
