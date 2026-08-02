use crate::config::Config;
use crate::date::{iso_to_secs, parse_rfc2822_date};
use crate::message::FilteredReason;
use regex::{Regex, RegexBuilder};

/// Maximum length of a filter regex pattern (admin config). Prevents pathological patterns.
const MAX_REGEX_PATTERN_LEN: usize = 512;

/// Maximum length of a single string matched against filter regexes.
const MAX_REGEX_INPUT_LEN: usize = 64 * 1024;

/// Compile-time DFA/NFA size budget for filter regexes (ReDoS mitigation).
const REGEX_SIZE_LIMIT: usize = 1 << 20; // 1 MiB compiled size
const REGEX_DFA_SIZE_LIMIT: usize = 1 << 20;

/// Compile a filter pattern with length and compiled-size guards.
fn compile_filter_regex(pattern: &str) -> Option<Regex> {
    if pattern.len() > MAX_REGEX_PATTERN_LEN {
        log::warn!("Filter regex pattern exceeds {} bytes; ignoring", MAX_REGEX_PATTERN_LEN);
        return None;
    }
    match RegexBuilder::new(pattern)
        .size_limit(REGEX_SIZE_LIMIT)
        .dfa_size_limit(REGEX_DFA_SIZE_LIMIT)
        .build()
    {
        Ok(r) => Some(r),
        Err(e) => {
            log::warn!("Invalid or too-complex filter regex '{}': {}", pattern, e);
            None
        },
    }
}

fn truncate_for_regex(s: &str) -> &str {
    if s.len() <= MAX_REGEX_INPUT_LEN {
        s
    } else {
        &s[..s.floor_char_boundary(MAX_REGEX_INPUT_LEN)]
    }
}

pub fn check_header_filter(headers: &[(String, String)], filter_list: &[String]) -> Vec<usize> {
    let mut matches = Vec::new();
    for (i, pattern) in filter_list.iter().enumerate() {
        let re = match compile_filter_regex(pattern) {
            Some(r) => r,
            None => continue,
        };
        for (name, value) in headers {
            let header_line = format!("{}: {}", name, value);
            let name_t = truncate_for_regex(name);
            let value_t = truncate_for_regex(value);
            let line_t = truncate_for_regex(&header_line);
            if re.is_match(line_t) || re.is_match(name_t) || re.is_match(value_t) {
                matches.push(i);
                break;
            }
        }
    }
    if matches.is_empty() {
        matches
    } else {
        vec![matches[0]]
    }
}

pub fn check_body_filter(body_lines: &[String], filter_list: &[String]) -> Vec<usize> {
    let mut matches = Vec::new();
    for (i, pattern) in filter_list.iter().enumerate() {
        let re = match compile_filter_regex(pattern) {
            Some(r) => r,
            None => continue,
        };
        for line in body_lines {
            if re.is_match(truncate_for_regex(line)) {
                matches.push(i);
                break;
            }
        }
    }
    matches
}

pub fn check_deleted_headers(headers: &[(String, String)], deleted_list: &[String]) -> bool {
    for (name, value) in headers {
        if deleted_list.iter().any(|d| d.eq_ignore_ascii_case(name))
            && value.trim().eq_ignore_ascii_case("yes")
        {
            return true;
        }
    }
    false
}

fn parse_date_flexible(s: &str) -> Option<i64> {
    parse_rfc2822_date(s).ok().or_else(|| iso_to_secs(s).ok())
}

pub fn check_expires_headers(headers: &[(String, String)], expires_list: &[String]) -> bool {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    for (name, value) in headers {
        if expires_list.iter().any(|e| e.eq_ignore_ascii_case(name)) {
            if let Some(t) = parse_date_flexible(value.trim()) {
                if t < now {
                    return true;
                }
            }
        }
    }
    false
}

pub fn check_delete_age(date: i64, delete_older: Option<&str>, delete_newer: Option<&str>) -> i32 {
    let mut result = 0;
    if let Some(older) = delete_older {
        match parse_date_flexible(older) {
            Some(threshold) => {
                if date > 0 && date < threshold {
                    result |= FilteredReason::FilteredOld as i32;
                }
            },
            None => log::warn!("Could not parse delete_older date: {}", older),
        }
    }
    if let Some(newer) = delete_newer {
        match parse_date_flexible(newer) {
            Some(threshold) => {
                if date > 0 && date > threshold {
                    result |= FilteredReason::FilteredNew as i32;
                }
            },
            None => log::warn!("Could not parse delete_newer date: {}", newer),
        }
    }
    result
}

pub fn check_delete_msgnum(msgnum: i32, delete_list: &[String]) -> bool {
    for entry in delete_list {
        if let Ok(n) = entry.parse::<i32>() {
            if n == msgnum {
                return true;
            }
        }
    }
    false
}

pub fn require_all_matched(require_results: &[bool]) -> bool {
    require_results.iter().all(|&r| r)
}

pub fn apply_filters(
    msgnum: i32,
    headers: &[(String, String)],
    body_lines: &[String],
    date: i64,
    config: &Config,
) -> (i32, Vec<bool>) {
    let mut is_deleted = 0;

    if !config.filter_out.values.is_empty()
        && !check_header_filter(headers, &config.filter_out.values).is_empty()
    {
        is_deleted |= FilteredReason::FilteredOut as i32;
    }

    if !config.filter_out_full_body.values.is_empty()
        && !check_body_filter(body_lines, &config.filter_out_full_body.values).is_empty()
    {
        is_deleted |= FilteredReason::FilteredOut as i32;
    }

    if !config.deleted.values.is_empty() && check_deleted_headers(headers, &config.deleted.values) {
        is_deleted |= FilteredReason::Delete as i32;
    }

    if !config.expires.values.is_empty() && check_expires_headers(headers, &config.expires.values) {
        is_deleted |= FilteredReason::Expire as i32;
    }

    is_deleted |=
        check_delete_age(date, config.delete_older.as_deref(), config.delete_newer.as_deref());

    if check_delete_msgnum(msgnum, &config.delete_msgnum.values) {
        is_deleted |= FilteredReason::Delete as i32;
    }

    // Compile each require pattern once per apply_filters call (not per header/line).
    let require_results: Vec<bool> = config
        .filter_require
        .values
        .iter()
        .map(|pattern| {
            let re = match compile_filter_regex(pattern) {
                Some(r) => r,
                None => return false,
            };
            headers.iter().any(|(name, value)| {
                let header_line = format!("{}: {}", name, value);
                re.is_match(truncate_for_regex(&header_line))
                    || re.is_match(truncate_for_regex(name))
                    || re.is_match(truncate_for_regex(value))
            })
        })
        .collect();

    let require_body_results: Vec<bool> = config
        .filter_require_full_body
        .values
        .iter()
        .map(|pattern| {
            let re = match compile_filter_regex(pattern) {
                Some(r) => r,
                None => return false,
            };
            body_lines.iter().any(|line| re.is_match(truncate_for_regex(line)))
        })
        .collect();

    let all_require = [require_results, require_body_results].concat();
    (is_deleted, all_require)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn test_check_header_filter_match() {
        let headers = vec![("subject".to_string(), "test message".to_string())];
        let filters = vec!["test".to_string()];
        assert!(!check_header_filter(&headers, &filters).is_empty());
    }

    #[test]
    fn test_check_header_filter_no_match() {
        let headers = vec![("subject".to_string(), "hello".to_string())];
        let filters = vec!["test".to_string()];
        assert!(check_header_filter(&headers, &filters).is_empty());
    }

    #[test]
    fn test_check_body_filter_match() {
        let lines = vec!["hello world".to_string(), "spam content".to_string()];
        let filters = vec!["spam".to_string()];
        assert!(!check_body_filter(&lines, &filters).is_empty());
    }

    #[test]
    fn test_check_body_filter_no_match() {
        let lines = vec!["hello world".to_string()];
        let filters = vec!["spam".to_string()];
        assert!(check_body_filter(&lines, &filters).is_empty());
    }

    #[test]
    fn test_check_deleted_headers_match() {
        let headers = vec![("x-hypermail-deleted".to_string(), "yes".to_string())];
        let deleted = vec!["X-Hypermail-Deleted".to_string()];
        assert!(check_deleted_headers(&headers, &deleted));
    }

    #[test]
    fn test_check_deleted_headers_no_value() {
        let headers = vec![("x-hypermail-deleted".to_string(), "no".to_string())];
        let deleted = vec!["X-Hypermail-Deleted".to_string()];
        assert!(!check_deleted_headers(&headers, &deleted));
    }

    #[test]
    fn test_check_expires_past() {
        let headers = vec![("expires".to_string(), "2000-01-01T00:00:00Z".to_string())];
        let expires = vec!["Expires".to_string()];
        assert!(check_expires_headers(&headers, &expires));
    }

    #[test]
    fn test_check_expires_future() {
        let headers = vec![("expires".to_string(), "Mon, 01 Jan 2099 00:00:00 +0000".to_string())];
        let expires = vec!["Expires".to_string()];
        assert!(!check_expires_headers(&headers, &expires));
    }

    #[test]
    fn test_delete_older() {
        let result = check_delete_age(1000, Some("2000-01-01"), None);
        assert!(result & FilteredReason::FilteredOld as i32 != 0);
    }

    #[test]
    fn test_delete_newer() {
        let result = check_delete_age(9999999999, None, Some("2000-01-01"));
        assert!(result & FilteredReason::FilteredNew as i32 != 0);
    }

    #[test]
    fn test_check_delete_msgnum_match() {
        let list = vec!["5".to_string(), "10".to_string()];
        assert!(check_delete_msgnum(5, &list));
        assert!(check_delete_msgnum(10, &list));
        assert!(!check_delete_msgnum(7, &list));
    }

    #[test]
    fn test_apply_filters_basic() {
        let config = Config::default();
        let headers = vec![("subject".to_string(), "hello".to_string())];
        let body = vec!["some body".to_string()];
        let (deleted, _) = apply_filters(1, &headers, &body, 1000, &config);
        assert_eq!(deleted, 0);
    }

    #[test]
    fn test_require_all_matched_true() {
        assert!(require_all_matched(&[true, true, true]));
    }

    #[test]
    fn test_require_all_matched_false() {
        assert!(!require_all_matched(&[true, false, true]));
    }

    #[test]
    fn test_require_all_matched_empty_is_true() {
        assert!(require_all_matched(&[]));
    }

    #[test]
    fn test_apply_filters_filter_out_header_match() {
        use crate::message::FilteredReason;
        let mut config = Config::default();
        config.filter_out.values.push("spam".to_string());
        let headers = vec![("subject".to_string(), "spam message".to_string())];
        let (deleted, _) = apply_filters(1, &headers, &[], 1000, &config);
        assert_ne!(deleted & FilteredReason::FilteredOut as i32, 0);
    }

    #[test]
    fn test_apply_filters_deleted_header() {
        use crate::message::FilteredReason;
        let mut config = Config::default();
        config.deleted.values.push("X-Hypermail-Deleted".to_string());
        let headers = vec![("x-hypermail-deleted".to_string(), "yes".to_string())];
        let (deleted, _) = apply_filters(1, &headers, &[], 1000, &config);
        assert_ne!(deleted & FilteredReason::Delete as i32, 0);
    }

    #[test]
    fn test_apply_filters_delete_msgnum() {
        use crate::message::FilteredReason;
        let mut config = Config::default();
        config.delete_msgnum.values.push("42".to_string());
        let (deleted, _) = apply_filters(42, &[], &[], 1000, &config);
        assert_ne!(deleted & FilteredReason::Delete as i32, 0);
    }

    #[test]
    fn test_check_header_filter_matches_header_name() {
        let headers = vec![("x-spam-flag".to_string(), "no".to_string())];
        let filters = vec!["x-spam-flag".to_string()];
        assert!(!check_header_filter(&headers, &filters).is_empty());
    }

    #[test]
    fn test_oversized_regex_pattern_ignored() {
        let headers = vec![("subject".to_string(), "test".to_string())];
        let huge = "a".repeat(MAX_REGEX_PATTERN_LEN + 1);
        let filters = vec![huge];
        assert!(check_header_filter(&headers, &filters).is_empty());
    }
}
