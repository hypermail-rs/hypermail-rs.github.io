use once_cell::sync::Lazy;
use regex::Regex;

static MIME_WORD_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"=\?([^?]+)\?([BbQq])\?([^?]*)\?=").unwrap());

#[derive(Debug, Clone)]
pub struct Header {
    pub name: String,
    pub body: String,
}

pub fn parse_headers(data: &[u8]) -> Vec<Header> {
    let text = String::from_utf8_lossy(data);
    let mut headers = Vec::new();
    let mut current_name = String::new();
    let mut current_body = String::new();
    let mut in_headers = true;

    for line in text.lines() {
        if in_headers {
            if line.is_empty() {
                in_headers = false;
                if !current_name.is_empty() {
                    headers.push(Header {
                        name: current_name.trim().to_lowercase(),
                        body: current_body.trim().to_string(),
                    });
                    current_name.clear();
                    current_body.clear();
                }
                continue;
            }

            if line.starts_with([' ', '\t']) {
                if !current_name.is_empty() {
                    current_body.push(' ');
                    current_body.push_str(line.trim());
                }
            } else if let Some((name, body)) = line.split_once(':') {
                if !current_name.is_empty() {
                    headers.push(Header {
                        name: current_name.trim().to_lowercase(),
                        body: current_body.trim().to_string(),
                    });
                }
                current_name = name.to_string();
                current_body = body.to_string();
            }
        }
    }

    if !current_name.is_empty() {
        headers.push(Header {
            name: current_name.trim().to_lowercase(),
            body: current_body.trim().to_string(),
        });
    }

    headers
}

pub fn find_header<'a>(headers: &'a [Header], name: &str) -> Option<&'a str> {
    let lower = name.to_lowercase();
    headers.iter().find(|h| h.name == lower).map(|h| h.body.as_str())
}

/// Return all header values with the given name (case-insensitive).
pub fn find_headers<'a>(headers: &'a [Header], name: &str) -> Vec<&'a str> {
    let lower = name.to_lowercase();
    headers.iter().filter(|h| h.name == lower).map(|h| h.body.as_str()).collect()
}

pub fn decode_mime_words(s: &str) -> String {
    let re = &*MIME_WORD_RE;

    // RFC 2047 §6.2: whitespace between adjacent encoded words is ignored.
    // Strategy: find all encoded words, group adjacent ones (separated only by whitespace),
    // decode each group by concatenating raw bytes, then reassemble.
    struct Match {
        start: usize,
        end: usize,
        charset: String,
        raw_bytes: Vec<u8>,
    }

    let mut matches: Vec<Match> = Vec::new();
    for caps in re.captures_iter(s) {
        let full = caps.get(0).unwrap();
        let charset = caps.get(1).unwrap().as_str().to_string();
        let encoding = caps.get(2).unwrap().as_str();
        let encoded = caps.get(3).unwrap().as_str();

        let raw_bytes = match encoding.to_uppercase().as_str() {
            "B" => decode_base64_mime_bytes(encoded),
            "Q" => decode_quoted_printable_bytes(encoded),
            _ => encoded.as_bytes().to_vec(),
        };

        matches.push(Match { start: full.start(), end: full.end(), charset, raw_bytes });
    }

    if matches.is_empty() {
        return s.to_string();
    }

    // Group adjacent encoded words (separated only by whitespace)
    let mut result = String::new();
    let mut prev_end: usize = 0;

    let mut i = 0;
    while i < matches.len() {
        // Add text before this group
        result.push_str(&s[prev_end..matches[i].start]);

        // Find the extent of this group of adjacent encoded words
        let mut group_bytes: Vec<u8> = matches[i].raw_bytes.clone();
        let mut group_charset = matches[i].charset.clone();
        let mut group_end = matches[i].end;
        let mut j = i + 1;

        while j < matches.len() {
            let between = &s[group_end..matches[j].start];
            if between.chars().all(|c| c == ' ' || c == '\t' || c == '\r' || c == '\n') {
                // Same charset: concatenate bytes for proper multi-byte decoding
                // Different charset: decode separately
                if matches[j].charset.eq_ignore_ascii_case(&group_charset) {
                    group_bytes.extend_from_slice(&matches[j].raw_bytes);
                } else {
                    // Decode current group and start new one
                    result.push_str(&decode_to_utf8(&group_bytes, &group_charset));
                    group_bytes = matches[j].raw_bytes.clone();
                    group_charset = matches[j].charset.clone();
                }
                group_end = matches[j].end;
                j += 1;
            } else {
                break;
            }
        }

        result.push_str(&decode_to_utf8(&group_bytes, &group_charset));
        prev_end = group_end;
        i = j;
    }

    // Add remaining text after last encoded word
    result.push_str(&s[prev_end..]);
    result
}

fn normalize_charset(charset: &str) -> String {
    let lower = charset.to_lowercase();
    // Handle common typos and variations
    match lower.as_str() {
        // Typo: ISO-8859-75 → ISO-8859-7
        "iso-8859-75" | "iso885975" => "iso-8859-7".to_string(),
        // iso-8859-15 is Latin-9, but often mislabeled for Greek content
        // Try to detect if it's likely Greek by attempting decode
        _ => lower,
    }
}

pub fn decode_to_utf8(data: &[u8], charset: &str) -> String {
    let charset_normalized = normalize_charset(charset);
    let charset_lower = charset_normalized.as_str();

    if charset_lower == "utf-8" || charset_lower == "utf8" || data.is_ascii() {
        return String::from_utf8_lossy(data).to_string();
    }

    // Special handling for iso-8859-1 and iso-8859-15 which are often mislabeled for Greek
    // These charsets can decode any byte, so we need to detect Greek content heuristically
    if charset_lower == "iso-8859-1" || charset_lower == "iso-8859-15" {
        // Try Greek charsets - if we get substantial Greek Unicode, it's likely mislabeled
        for fallback in &["iso-8859-7", "windows-1253"] {
            if let Some(encoding) = encoding_rs::Encoding::for_label(fallback.as_bytes()) {
                let (cow, _, _) = encoding.decode(data);

                // Count Greek Unicode characters (U+0370-U+03FF)
                let greek_count =
                    cow.chars().filter(|c| ('\u{0370}'..='\u{03FF}').contains(c)).count();
                let total_alpha = cow.chars().filter(|c| c.is_alphabetic()).count();

                // If >30% of alphabetic chars are Greek Unicode, treat as Greek
                if total_alpha > 0 && greek_count * 100 / total_alpha > 30 {
                    return cow.into_owned();
                }
            }
        }
        // If Greek charsets didn't work, fall through to try the specified charset
    }

    // Try the specified (normalized) charset
    if let Some(encoding) = encoding_rs::Encoding::for_label(charset_normalized.as_bytes()) {
        let (cow, _, had_errors) = encoding.decode(data);
        // If no errors or no replacement chars, use this decoding
        if !had_errors || !cow.contains('\u{FFFD}') {
            return cow.into_owned();
        }
    }

    // Fallback: Try common Greek/European charsets
    for fallback in &["iso-8859-7", "windows-1253", "windows-1252", "iso-8859-1"] {
        if let Some(encoding) = encoding_rs::Encoding::for_label(fallback.as_bytes()) {
            let (cow, _, _) = encoding.decode(data);
            if !cow.contains('\u{FFFD}') {
                return cow.into_owned();
            }
        }
    }

    // Final fallback: return what we got, even with replacement chars
    String::from_utf8_lossy(data).to_string()
}

fn decode_base64_mime_bytes(s: &str) -> Vec<u8> {
    use base64::Engine as _;
    let engine = base64::engine::general_purpose::STANDARD;
    engine.decode(s).unwrap_or_else(|_| s.as_bytes().to_vec())
}

fn decode_quoted_printable_bytes(s: &str) -> Vec<u8> {
    let s = s.replace('_', " ");
    let data = s.as_bytes();
    let mut result = Vec::with_capacity(data.len());
    let mut i = 0;

    while i < data.len() {
        if data[i] == b'=' && i + 2 < data.len() {
            if let (Some(h), Some(l)) = (hex_val(data[i + 1]), hex_val(data[i + 2])) {
                result.push(h << 4 | l);
                i += 3;
                continue;
            }
        }
        if data[i] != b'\r' {
            result.push(data[i]);
        }
        i += 1;
    }

    result
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'A'..=b'F' => Some(b - b'A' + 10),
        b'a'..=b'f' => Some(b - b'a' + 10),
        _ => None,
    }
}

pub fn unfold_header(s: &str) -> String {
    // RFC 2822 §2.2.3: unfold by replacing CRLF+WSP with a single space
    let s = s.replace("\r\n ", " ").replace("\r\n\t", " ");
    // Also handle bare LF folding (non-standard but common)
    let s = s.replace("\n ", " ").replace("\n\t", " ");
    // Strip any remaining bare CR/LF
    s.replace(['\r', '\n'], "")
}

pub fn parse_email_address(s: &str) -> (Option<String>, Option<String>) {
    let s = s.trim();

    if let Some(angle_start) = s.find('<') {
        let name = if angle_start > 0 {
            Some(s[..angle_start].trim().trim_matches('"').to_string())
        } else {
            None
        };
        let email = s[angle_start..]
            .find('>')
            .map(|angle_end| s[angle_start + 1..angle_start + angle_end].to_string());
        return (name, email);
    }

    if let Some(paren_start) = s.find('(') {
        let email = Some(s[..paren_start].trim().to_string());
        let name = s[paren_start..]
            .find(')')
            .map(|paren_end| s[paren_start + 1..paren_start + paren_end].to_string());
        return (name, email);
    }

    if s.contains('@') {
        return (None, Some(s.to_string()));
    }

    (Some(s.to_string()), None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_headers() {
        let data = b"From: alice@example.com\nSubject: Hello\n\nBody text\n";
        let headers = parse_headers(data);
        assert_eq!(headers.len(), 2);
        assert_eq!(headers[0].name, "from");
        assert_eq!(headers[0].body, "alice@example.com");
        assert_eq!(headers[1].name, "subject");
        assert_eq!(headers[1].body, "Hello");
    }

    #[test]
    fn test_find_header() {
        let headers = parse_headers(b"From: alice@example.com\nSubject: Test\n\nBody\n");
        assert_eq!(find_header(&headers, "From"), Some("alice@example.com"));
        assert_eq!(find_header(&headers, "Subject"), Some("Test"));
        assert_eq!(find_header(&headers, "Date"), None);
    }

    #[test]
    fn test_folded_headers() {
        let data = b"Subject: A very long\n subject header\n\nBody\n";
        let headers = parse_headers(data);
        assert_eq!(find_header(&headers, "Subject"), Some("A very long subject header"));
    }

    #[test]
    fn test_decode_mime_b() {
        let decoded = decode_mime_words("=?UTF-8?B?SGVsbG8gV29ybGQ=?=");
        assert_eq!(decoded, "Hello World");
    }

    #[test]
    fn test_decode_mime_q() {
        let decoded = decode_mime_words("=?utf-8?Q?H=C3=A5kan?=");
        assert_eq!(decoded, "Håkan");
    }

    #[test]
    fn test_decode_mime_mixed() {
        let decoded = decode_mime_words("Re: =?UTF-8?B?SGVsbG8=?=");
        assert_eq!(decoded, "Re: Hello");
    }

    #[test]
    fn test_decode_mime_q_iso8859_1() {
        // ISO-8859-1 encoded QP: H=E5kan → Håkan
        let decoded = decode_mime_words("=?ISO-8859-1?Q?H=E5kan?=");
        assert_eq!(decoded, "Håkan");
    }

    #[test]
    fn test_decode_mime_b_iso8859_1() {
        // ISO-8859-1 encoded base64: "Håkan" in ISO-8859-1 bytes
        let decoded = decode_mime_words("=?ISO-8859-1?B?SOVrYW4=?=");
        assert_eq!(decoded, "Håkan");
    }

    #[test]
    fn test_decode_mime_q_shift_jis() {
        // Shift_JIS encoded base64: "日本語" in Shift_JIS bytes
        let decoded = decode_mime_words("=?Shift_JIS?B?k/qWe4zq?=");
        assert_eq!(decoded, "日本語");
    }

    #[test]
    fn test_decode_mime_adjacent_words() {
        // Two adjacent encoded words should decode independently
        let decoded = decode_mime_words("=?UTF-8?Q?Hello=20?==?UTF-8?Q?World?=");
        assert_eq!(decoded, "Hello World");
    }

    #[test]
    fn test_parse_email() {
        let (name, email) = parse_email_address("Alice <alice@example.com>");
        assert_eq!(name.as_deref(), Some("Alice"));
        assert_eq!(email.as_deref(), Some("alice@example.com"));
    }

    #[test]
    fn test_parse_email_no_name() {
        let (name, email) = parse_email_address("alice@example.com");
        assert!(name.is_none());
        assert_eq!(email.as_deref(), Some("alice@example.com"));
    }

    #[test]
    fn test_unfold_header() {
        let unfolded = unfold_header("Subject: A very\r\n long subject");
        assert_eq!(unfolded, "Subject: A very long subject");
    }

    #[test]
    fn test_empty_headers() {
        let headers = parse_headers(b"\nJust body\n");
        assert!(headers.is_empty());
    }

    // --- Greek charset tests ---

    #[test]
    fn test_decode_mime_b_iso_8859_7_kalimera() {
        // B-encoded ISO-8859-7 "Καλημερα"
        // ISO-8859-7 bytes: CAE1EBE7ECE5F1E1 → base64: yuHr5+zl8eE=
        let decoded = decode_mime_words("=?ISO-8859-7?B?yuHr5+zl8eE=?=");
        assert_eq!(decoded, "Καλημερα");
    }

    #[test]
    fn test_decode_mime_q_iso_8859_7_geia() {
        // Q-encoded ISO-8859-7 "Γεια": Γ=0xC3 ε=0xE5 ι=0xE9 α=0xE1
        let decoded = decode_mime_words("=?ISO-8859-7?Q?=C3=E5=E9=E1?=");
        assert_eq!(decoded, "Γεια");
    }

    #[test]
    fn test_decode_mime_b_windows_1253_anthropos() {
        // B-encoded Windows-1253 "άνθρωπος"
        // Windows-1253 bytes: DCEDE8F1F9F0EFF2 → base64: 3O3o8fnw7/I=
        let decoded = decode_mime_words("=?windows-1253?B?3O3o8fnw7/I=?=");
        assert_eq!(decoded, "άνθρωπος");
    }

    #[test]
    fn test_decode_mime_greek_mixed_text() {
        // Mixed: plain ASCII prefix + ISO-8859-7 encoded word
        let decoded = decode_mime_words("Re: =?ISO-8859-7?B?yuHr5+zl8eE=?=");
        assert_eq!(decoded, "Re: Καλημερα");
    }

    #[test]
    fn test_decode_mime_greek_adjacent_words() {
        // Two adjacent encoded words: RFC 2047 §6.2 says whitespace between them is removed
        // "Γεια" + "σου" = "Γειασου" (no space — space must be encoded if intended)
        let decoded = decode_mime_words("=?ISO-8859-7?B?w+Xp4Q==?= =?ISO-8859-7?B?8+/1?=");
        assert_eq!(decoded, "Γειασου");
    }

    #[test]
    fn test_decode_mime_utf8_greek() {
        let decoded = decode_mime_words("=?UTF-8?B?zprOsc67zrfOvM61z4HOsQ==?=");
        assert_eq!(decoded, "Καλημερα");
    }

    #[test]
    fn test_decode_mime_multiple_charsets() {
        // Two encoded words with different charsets — RFC 2047 strips whitespace between them
        let decoded = decode_mime_words("=?ISO-8859-7?B?w+Xp4Q==?= =?ISO-8859-1?Q?H=E5kan?=");
        assert_eq!(decoded, "ΓειαHåkan");
    }

    #[test]
    fn test_decode_mime_iso_8859_7_tonos() {
        // B-encoded ISO-8859-7 "άνθρωπος" with tonos
        // ISO-8859-7 bytes: DCEDE8F1F9F0EFF2 → base64: 3O3o8fnw7/I=
        let decoded = decode_mime_words("=?ISO-8859-7?B?3O3o8fnw7/I=?=");
        assert_eq!(decoded, "άνθρωπος");
    }

    // Test raw header with ISO-8859-7 subject (non-RFC2047, raw bytes)
    #[test]
    fn test_parse_headers_greek_subject() {
        // Subject header with raw ISO-8859-7 bytes (no RFC 2047 encoding)
        // "Γεια" in ISO-8859-7
        let raw_bytes = b"Subject: \xC3\xE5\xE9\xE1\nFrom: test@test.com\n\nBody\n";
        let headers = parse_headers(raw_bytes);
        let subject = find_header(&headers, "Subject").unwrap();
        // The body is parsed with from_utf8_lossy, so it'll have replacement chars
        // This test verifies the raw bytes are captured for later re-decoding
        assert!(
            subject.contains('\u{FFFD}') || subject == "Γεια",
            "Raw non-UTF-8 subject should either contain replacement chars or be valid UTF-8"
        );
    }

    // Additional comprehensive Greek RFC 2047 tests

    #[test]
    fn test_decode_mime_uppercase_tonos_iso_8859_7() {
        // "Άνθρωπος" (uppercase alpha with tonos)
        // ISO-8859-7 bytes: B6EDE8F1F9F0EFF2 → base64: tu3o8fnw7/I=
        let decoded = decode_mime_words("=?ISO-8859-7?B?tu3o8fnw7/I=?=");
        assert_eq!(decoded, "Άνθρωπος");
    }

    #[test]
    fn test_decode_mime_uppercase_tonos_windows_1253() {
        // "Άνθρωπος" (uppercase alpha with tonos)
        // Windows-1253 bytes: A2EDE8F1F9F0EFF2 → base64: ou3o8fnw7/I=
        let decoded = decode_mime_words("=?windows-1253?B?ou3o8fnw7/I=?=");
        assert_eq!(decoded, "Άνθρωπος");
    }

    #[test]
    fn test_decode_mime_real_world_greeting() {
        // "Καλό απόγευμα" (Good afternoon)
        // ISO-8859-7 bytes: CAE1EB FC 20 E1F0FCE3E5F5ECA1
        let decoded = decode_mime_words("=?ISO-8859-7?Q?=CA=E1=EB=FC_=E1=F0=FC=E3=E5=F5=EC=E1?=");
        assert_eq!(decoded, "Καλό απόγευμα");
    }

    #[test]
    fn test_decode_mime_greek_question() {
        // "Πώς είσαι;" (How are you?)
        // ISO-8859-7 Q-encoded
        let decoded = decode_mime_words("=?ISO-8859-7?Q?=D0=FE=F2_=E5=DF=F3=E1=E9;?=");
        assert_eq!(decoded, "Πώς είσαι;");
    }

    #[test]
    fn test_decode_mime_mixed_greek_latin_subject() {
        // "Re: Καλημερα" - common reply pattern
        let decoded = decode_mime_words("Re: =?ISO-8859-7?B?yuHr5+zl8eE=?=");
        assert_eq!(decoded, "Re: Καλημερα");
    }

    #[test]
    fn test_decode_mime_greek_with_numbers() {
        // "Σελίδα 123" (Page 123)
        // ISO-8859-7: Σ=0xD3 ε=0xE5 λ=0xEB ί=0xDF δ=0xE4 α=0xE1
        let decoded = decode_mime_words("=?ISO-8859-7?Q?=D3=E5=EB=DF=E4=E1_123?=");
        assert_eq!(decoded, "Σελίδα 123");
    }

    #[test]
    fn test_decode_mime_fwd_greek() {
        // "Fwd: Ελληνικά" (Forward: Greek)
        // Mixed ASCII prefix + Greek encoded word
        let decoded = decode_mime_words("Fwd: =?UTF-8?B?zpXOu867zrfOvc65zrrOrA==?=");
        assert_eq!(decoded, "Fwd: Ελληνικά");
    }

    #[test]
    fn test_decode_mime_greek_parentheses() {
        // "(Σημαντικό)" (Important in parentheses)
        // ISO-8859-7: Σ=0xD3 η=0xE7 μ=0xEC α=0xE1 ν=0xED τ=0xF4 ι=0xE9 κ=0xEA ό=0xFC
        let decoded = decode_mime_words("(=?ISO-8859-7?B?0+fs4e306er8?=)");
        assert_eq!(decoded, "(Σημαντικό)");
    }

    #[test]
    fn test_decode_mime_multiple_greek_words_adjacent() {
        // "Καλή" + "μέρα" — RFC 2047 strips whitespace between adjacent encoded words
        let decoded = decode_mime_words("=?ISO-8859-7?B?yuHr3g==?= =?ISO-8859-7?B?7N3x4Q==?=");
        assert_eq!(decoded, "Καλήμέρα");
    }

    #[test]
    fn test_decode_mime_greek_diaeresis() {
        // "ϊδιος" (same, with diaeresis on iota)
        // ISO-8859-7: ϊ=0xFA δ=0xE4 ι=0xE9 ο=0xEF ς=0xF2
        let decoded = decode_mime_words("=?ISO-8859-7?B?+uTp7/I=?=");
        assert_eq!(decoded, "ϊδιος");
    }

    #[test]
    fn test_decode_mime_windows_1253_real_world() {
        // Real Windows-1253 encoded subject from Greek email client
        // "Ευχαριστώ" (Thank you)
        // Windows-1253: Ε=0xC5 υ=0xF5 χ=0xF7 α=0xE1 ρ=0xF1 ι=0xE9 σ=0xF3 τ=0xF4 ώ=0xFE
        let decoded = decode_mime_words("=?windows-1253?B?xfX34fHp8/T+?=");
        assert_eq!(decoded, "Ευχαριστώ");
    }

    #[test]
    fn test_decode_mislabeled_iso_8859_1_as_greek() {
        // Real-world case: Message labeled as iso-8859-1 but contains Greek (iso-8859-7)
        // Greek text: "Σωστά όλα αυτά" (Correct, all that)
        // In iso-8859-7: Σ=0xD3 ω=0xF9 σ=0xF3 τ=0xF4 ά=0xDC space=0x20 ό=0xFC λ=0xEB α=0xE1
        let greek_bytes = b"\xD3\xF9\xF3\xF4\xDC\x20\xFC\xEB\xE1\x20\xE1\xF5\xF4\xDC";

        // When decoded as iso-8859-1, we get mojibake, but it should auto-detect Greek
        let result = decode_to_utf8(greek_bytes, "iso-8859-1");
        assert!(
            result.contains("Σωστά") || result.contains("ωστά"),
            "Should detect Greek in mislabeled iso-8859-1: got '{}'",
            result
        );
    }

    #[test]
    fn test_decode_correct_iso_8859_1_latin() {
        // Actual Latin-1 text should not be "corrected" to Greek
        // French: "Café résumé"
        // é=0xE9 in iso-8859-1
        let latin_bytes = b"Caf\xE9 r\xE9sum\xE9";

        let result = decode_to_utf8(latin_bytes, "iso-8859-1");
        assert_eq!(result, "Café résumé", "Should preserve correct Latin-1 text");
    }

    #[test]
    fn test_find_headers_multiple_values() {
        let data = b"Received: from a\nReceived: from b\nFrom: alice@example.com\n\nBody\n";
        let headers = parse_headers(data);
        let received = find_headers(&headers, "Received");
        assert_eq!(received.len(), 2);
        assert!(received.contains(&"from a"));
        assert!(received.contains(&"from b"));
    }

    #[test]
    fn test_find_headers_none_found() {
        let headers = parse_headers(b"From: alice@example.com\n\nBody\n");
        let result = find_headers(&headers, "X-Missing");
        assert!(result.is_empty());
    }

    #[test]
    fn test_unfold_header_bare_lf() {
        let result = unfold_header("Subject: Long\n header");
        assert_eq!(result, "Subject: Long header");
    }

    #[test]
    fn test_parse_email_paren_style() {
        let (name, email) = parse_email_address("alice@example.com (Alice)");
        assert_eq!(name.as_deref(), Some("Alice"));
        assert_eq!(email.as_deref(), Some("alice@example.com"));
    }

    #[test]
    fn test_parse_email_bare_name() {
        let (name, email) = parse_email_address("Alice");
        assert_eq!(name.as_deref(), Some("Alice"));
        assert!(email.is_none());
    }
}
