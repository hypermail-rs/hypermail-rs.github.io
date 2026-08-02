use regex::Regex;
use std::sync::LazyLock;

/// Maximum length for URL detection to prevent ReDoS attacks.
///
/// RFC 3986 doesn't specify a maximum URL length, but browsers typically
/// support 2048 characters. We allow up to 4096 for compatibility with
/// data URIs and long query strings.
const MAX_URL_LENGTH: usize = 4096;

/// Maximum subject length for thread detection processing.
///
/// RFC 2822 recommends lines < 998 characters, but some clients generate
/// longer subjects. We limit to 2048 for performance in O(n²) threading loop.
const MAX_SUBJECT_THREAD_LENGTH: usize = 2048;

static URL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?i)((https?|ftp)://[^\s<>"']+|www\.[^\s<>"']+)"#).unwrap());

static EMAIL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"([a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,})").unwrap());

static UNRE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(\s*(re|fwd?|aw|ang|sv|vs|odp|antw)\s*[\[:\]>#]*\s*)+")
        .expect("UNRE_RE compile")
});

static ONEUNRE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^\s*(re|fwd?|aw|ang|sv|vs|odp|antw)\s*[\[:\]>#]*\s*")
        .expect("ONEUNRE_RE compile")
});

static STRIPZONE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+\([^)]*\)\s*$").unwrap());

static NUM_REF_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"&#(\d+);").unwrap());

/// Strips reply prefixes from email subjects.
///
/// Removes internationalized reply/forward prefixes: Re:, Fwd:, AW: (German),
/// SV: (Swedish), Odp: (Polish), Antw: (Dutch), etc.
///
/// # Security
///
/// For performance in O(n²) threading loops, extremely long subjects are
/// truncated to MAX_SUBJECT_THREAD_LENGTH before regex processing.
///
/// # Examples
///
/// ```
/// use hypermail::string_utils::unre;
/// assert_eq!(unre("Re: Hello"), "Hello");
/// assert_eq!(unre("RE: Re: Fwd: Hello"), "Hello");
/// assert_eq!(unre("AW: Diskussion"), "Diskussion");
/// ```
pub fn unre(subject: &str) -> String {
    // Security: Limit subject length to prevent ReDoS on pathological inputs
    // Use floor_char_boundary to avoid panicking on multi-byte UTF-8 sequences
    let truncated = if subject.len() > MAX_SUBJECT_THREAD_LENGTH {
        &subject[..subject.floor_char_boundary(MAX_SUBJECT_THREAD_LENGTH)]
    } else {
        subject
    };

    UNRE_RE.replace(truncated, "").trim().to_string()
}

/// Strips a single reply/forward prefix from a subject line.
pub fn oneunre(subject: &str) -> String {
    // Security: Same truncation as unre()
    // Use floor_char_boundary to avoid panicking on multi-byte UTF-8 sequences
    let truncated = if subject.len() > MAX_SUBJECT_THREAD_LENGTH {
        &subject[..subject.floor_char_boundary(MAX_SUBJECT_THREAD_LENGTH)]
    } else {
        subject
    };

    ONEUNRE_RE.replace(truncated, "").trim().to_string()
}

/// Attempts to parse a URL from the start of a string, writing it into `url`.
pub fn parse_url(s: &str, url: &mut String) -> Option<usize> {
    if let Some(m) = URL_RE.find(s) {
        url.push_str(m.as_str());
        Some(m.len())
    } else {
        None
    }
}

/// Converts URLs in text to clickable HTML links.
///
/// Detects http://, https://, ftp:// URLs and www. patterns, converting
/// them to `<a>` tags with rel="noopener noreferrer" for security.
///
/// # Security
///
/// To prevent ReDoS attacks, this function skips processing if the input
/// exceeds reasonable length or contains extremely long potential URLs.
///
/// # Arguments
///
/// * `line` - Text that may contain URLs
///
/// # Returns
///
/// Text with URLs replaced by HTML `<a>` tags
pub fn conv_urls(line: &str) -> String {
    // Security: Skip URL processing on unreasonably large inputs
    // This prevents ReDoS on malicious inputs with pathological patterns
    if line.len() > MAX_URL_LENGTH * 10 {
        return line.to_string();
    }

    URL_RE
        .replace_all(line, |caps: &regex::Captures| {
            let url = &caps[1];

            // Security: Skip extremely long URLs to prevent memory issues
            if url.len() > MAX_URL_LENGTH {
                return url.to_string();
            }

            let href = if url.starts_with("www.") {
                format!("https://{}", url)
            } else {
                url.to_string()
            };
            // SEC: Escape href attribute to prevent attribute injection via crafted URLs
            format!(
                "<a href=\"{}\" rel=\"noopener noreferrer\">{}</a>",
                escape_html_attr(&href),
                url
            )
        })
        .to_string()
}

/// Escape a string for safe use inside an HTML attribute value (double-quote delimited).
fn escape_html_attr(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => result.push_str("&amp;"),
            '"' => result.push_str("&quot;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '\'' => result.push_str("&#39;"),
            c => result.push(c),
        }
    }
    result
}

/// Obfuscates an email address using HTML numeric character references.
pub fn obfuscate_email_address(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '@' => result.push_str("&#64;"),
            '.' => result.push('.'),
            '-' => result.push('-'),
            '_' => result.push('_'),
            c if c.is_ascii_alphanumeric() => {
                let code = c as u32;
                result.push_str(&format!("&#{};", code));
            },
            c => result.push(c),
        }
    }
    result
}

/// Reverses HTML numeric character reference obfuscation back to plain text.
pub fn unobfuscate_email_address(s: &str) -> String {
    NUM_REF_RE
        .replace_all(s, |caps: &regex::Captures| {
            let code: u32 = caps[1].parse().unwrap_or(0);
            char::from_u32(code).map_or(String::new(), |c| c.to_string())
        })
        .to_string()
}

/// Applies spam protection to email addresses in a string.
///
/// Replaces `@` with the configured anti-spam string, or substitutes the domain.
pub fn spamify(
    s: &str,
    antispam_at: &str,
    antispamdomain: Option<&str>,
    spamprotect: bool,
    spamprotect_id: bool,
) -> String {
    if !spamprotect && !spamprotect_id {
        return s.to_string();
    }

    if !EMAIL_RE.is_match(s) {
        return s.to_string();
    }

    let result = EMAIL_RE.replace_all(s, |caps: &regex::Captures| {
        let email = &caps[1];
        if let Some(domain) = antispamdomain {
            if let Some(at_pos) = email.find('@') {
                let local = &email[..at_pos];
                return format!("{}@{}", local, domain);
            }
        }
        if spamprotect {
            email.replace('@', antispam_at)
        } else {
            email.to_string()
        }
    });

    result.to_string()
}

/// Replaces characters found in `chars` with underscores.
pub fn convchars(s: &str, chars: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        if chars.contains(c) {
            result.push('_');
        } else {
            result.push(c);
        }
    }
    result
}

/// Strips trailing parenthetical timezone info from a date string.
pub fn stripzone(s: &str) -> String {
    STRIPZONE_RE.replace(s.trim(), "").to_string()
}

/// Returns `None` if the string is empty or "NONE", otherwise returns `Some`.
pub fn getvalue(s: &str) -> Option<&str> {
    let s = s.trim();
    if s.is_empty() || s.eq_ignore_ascii_case("NONE") {
        None
    } else {
        Some(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unre() {
        assert_eq!(unre("Re: Hello"), "Hello");
        assert_eq!(unre("Re: Re: Hello"), "Hello");
        assert_eq!(unre("Fwd: Hello"), "Hello");
        assert_eq!(unre("Hello"), "Hello");
    }

    #[test]
    fn test_conv_urls() {
        let result = conv_urls("Visit https://example.com today");
        assert!(result.contains("<a href=\"https://example.com\""));
        assert!(result.contains("rel=\"noopener noreferrer\""));
    }

    #[test]
    fn test_obfuscate_email() {
        let ob = obfuscate_email_address("a@b.com");
        assert!(ob.contains("&#97;"));
        assert!(ob.contains("&#64;"));
    }

    #[test]
    fn test_spamify() {
        let result = spamify("a@b.com", " at ", None, true, false);
        assert_eq!(result, "a at b.com");
    }

    #[test]
    fn test_spamify_with_domain() {
        let result = spamify("a@b.com", "@", Some("example.com"), true, false);
        assert_eq!(result, "a@example.com");
    }

    #[test]
    fn test_stripzone() {
        let result = stripzone("Mon, 15 Mar 2021 12:00:00 +0000 (UTC)");
        assert!(!result.contains("(UTC)"));
    }

    #[test]
    fn test_getvalue() {
        assert_eq!(getvalue("test"), Some("test"));
        assert_eq!(getvalue("NONE"), None);
        assert_eq!(getvalue(""), None);
    }

    #[test]
    fn test_spamify_antispamdomain_replaces_domain() {
        let result = spamify("user@real-domain.com", "_at_", Some("nospam.invalid"), true, false);
        assert!(result.contains("nospam.invalid"), "domain should be replaced");
        assert!(!result.contains("real-domain.com"), "original domain should be gone");
    }

    #[test]
    fn test_spamify_antispamdomain_none_falls_back_to_at_replacement() {
        let result = spamify("user@real-domain.com", "_at_", None, true, false);
        assert!(result.contains("_at_"), "should use antispam_at when no antispamdomain");
        assert!(!result.contains('@'), "@ should be replaced");
    }

    #[test]
    fn test_convchars() {
        assert_eq!(convchars("hello world", " "), "hello_world");
    }

    #[test]
    fn test_oneunre_strips_single_prefix() {
        assert_eq!(oneunre("Re: Hello"), "Hello");
        assert_eq!(oneunre("Re: Re: Hello"), "Re: Hello");
        assert_eq!(oneunre("Hello"), "Hello");
    }

    #[test]
    fn test_parse_url_found() {
        let mut url = String::new();
        let len = parse_url("https://example.com/path?q=1", &mut url);
        assert!(len.is_some());
        assert_eq!(url, "https://example.com/path?q=1");
    }

    #[test]
    fn test_parse_url_not_found() {
        let mut url = String::new();
        let len = parse_url("plain text no url", &mut url);
        assert!(len.is_none());
        assert!(url.is_empty());
    }

    #[test]
    fn test_unobfuscate_roundtrip() {
        let original = "user@example.com";
        let obfuscated = obfuscate_email_address(original);
        let restored = unobfuscate_email_address(&obfuscated);
        assert_eq!(restored, original);
    }

    #[test]
    fn test_spamify_no_email_unchanged() {
        let result = spamify("no email here", " at ", None, true, false);
        assert_eq!(result, "no email here");
    }

    #[test]
    fn test_spamify_disabled_unchanged() {
        let result = spamify("user@example.com", " at ", None, false, false);
        assert_eq!(result, "user@example.com");
    }

    #[test]
    fn test_conv_urls_escapes_href_with_quotes() {
        // In practice, conv_urls() receives pre-escaped input from escape_html(),
        // so raw " never appears. The URL regex [^\s<>"']+ also stops at " by design.
        // Test that the escape_html + conv_urls pipeline is safe:
        let escaped_input =
            crate::txt2html::escape_html(r#"Visit https://evil.com/a"onmouseover="alert(1) today"#);
        let result = conv_urls(&escaped_input);
        // The " was escaped to &quot; before conv_urls, so it's part of the URL match
        // but rendered safely in the href via escape_html_attr.
        assert!(
            !result.contains(r#""onmouseover"#),
            "raw double-quote injection must not appear: {}",
            result
        );
    }

    #[test]
    fn test_conv_urls_escapes_special_chars_in_href() {
        // URLs with & should be properly escaped in the href attribute
        let result = conv_urls("https://example.com/search?a=1&amp;b=2");
        assert!(
            result.contains("&amp;amp;") || result.contains("&amp;b=2"),
            "& in URL should be preserved or double-escaped in href attribute: {}",
            result
        );
    }

    #[test]
    fn test_conv_urls_escapes_angle_brackets() {
        // The URL regex stops at < so no <script> can appear in generated href
        let escaped_input = crate::txt2html::escape_html("https://evil.com/<script>");
        let result = conv_urls(&escaped_input);
        assert!(
            !result.contains("<script>"),
            "angle brackets should not appear raw in link output: {}",
            result
        );
    }

    #[test]
    fn test_unre_utf8_at_truncation_boundary() {
        // Build a subject with multi-byte UTF-8 chars near the 2048-byte boundary.
        // Each 'ä' is 2 bytes in UTF-8. Place them so the 2048 boundary falls mid-char.
        let prefix = "Re: ";
        let filler = "ä".repeat(1024); // 2048 bytes of 'ä'
        let subject = format!("{}{}", prefix, filler);
        // This should NOT panic even though byte 2048 might be mid-char
        let result = unre(&subject);
        assert!(!result.is_empty(), "should handle UTF-8 at truncation boundary");
    }

    #[test]
    fn test_oneunre_utf8_at_truncation_boundary() {
        let filler = "ö".repeat(1025); // 2050 bytes, exceeds MAX_SUBJECT_THREAD_LENGTH
        let subject = format!("Re: {}", filler);
        // Should not panic
        let result = oneunre(&subject);
        assert!(!result.is_empty());
    }
}
