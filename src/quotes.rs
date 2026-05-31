use crate::message::BodyChain;

const QUOTE_PREFIXES: &[&str] = &["> ", ">", "| ", "|", ": ", ":"];

pub fn find_quote_prefix(bodies: &BodyChain) -> Option<&'static str> {
    let mut counts: Vec<(&str, usize)> = QUOTE_PREFIXES.iter().map(|p| (*p, 0)).collect();

    for body in &bodies.bodies {
        if body.attached || body.header {
            continue;
        }
        for (prefix, count) in &mut counts {
            if body.line.starts_with(*prefix) {
                *count += 1;
            }
        }
    }

    counts
        .into_iter()
        .filter(|(_, c)| *c > 0)
        .max_by_key(|(_, c)| *c)
        .map(|(p, _)| p)
}

pub fn get_quote_prefix() -> &'static str {
    "> "
}

pub fn is_quote(line: &str) -> bool {
    QUOTE_PREFIXES.iter().any(|p| line.starts_with(p))
}

pub fn unquote(line: &str) -> String {
    for prefix in QUOTE_PREFIXES {
        if let Some(rest) = line.strip_prefix(prefix) {
            return rest.to_string();
        }
    }
    line.to_string()
}

pub fn find_quote_depth(line: &str) -> i32 {
    let mut depth = 0;
    let mut s = line;
    loop {
        let mut found = false;
        for prefix in QUOTE_PREFIXES {
            if let Some(rest) = s.strip_prefix(prefix) {
                depth += 1;
                s = rest;
                found = true;
                break;
            }
        }
        if !found {
            break;
        }
    }
    depth
}

pub fn find_quote_class(line: &str) -> String {
    let depth = find_quote_depth(line).min(9);
    if depth > 0 {
        format!("hm-quote-{}", depth)
    } else {
        String::new()
    }
}

pub fn compute_quoted_percent(bodies: &BodyChain) -> i32 {
    let total = bodies.bodies.iter().filter(|b| !b.attached && !b.header).count();
    if total == 0 {
        return 0;
    }
    let quoted = bodies
        .bodies
        .iter()
        .filter(|b| !b.attached && !b.header && is_quote(&b.line))
        .count();
    (quoted * 100 / total) as i32
}

pub fn is_sig_start(line: &str) -> bool {
    line == "-- " || line == "-- \r" || line == "--" || line == "--\r"
}

pub fn remove_hypermail_tags(line: &str) -> String {
    if line.contains("class=\"hm-") || line.contains("<a name=") || line.contains("<article id=") {
        let mut result = String::with_capacity(line.len());
        let mut skip_anchor = false;
        let mut i = 0;
        let chars: Vec<char> = line.chars().collect();

        while i < chars.len() {
            if chars[i] == '<' {
                if i + 7 <= chars.len() {
                    let tag: String = chars[i..i + 7].iter().collect();
                    if tag.to_lowercase() == "<a name" {
                        skip_anchor = true;
                        i += 1;
                        continue;
                    }
                }
                if i + 4 <= chars.len() {
                    let end_a: String = chars[i..i + 4].iter().collect();
                    if end_a.to_lowercase() == "</a>" && skip_anchor {
                        skip_anchor = false;
                        i += 4;
                        continue;
                    }
                }
            }
            if !skip_anchor {
                result.push(chars[i]);
            }
            i += 1;
        }
        result
    } else {
        line.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{Body, BodyChain};

    fn make_body(lines: &[&str]) -> BodyChain {
        BodyChain {
            bodies: lines
                .iter()
                .map(|l| Body {
                    line: l.to_string(),
                    html: false,
                    header: false,
                    parsed_header: false,
                    attached: false,
                    demimed: false,
                    msgnum: 0,
                })
                .collect(),
        }
    }

    #[test]
    fn test_is_quote() {
        assert!(is_quote("> quoted text"));
        assert!(is_quote(">quoted"));
        assert!(!is_quote("not quoted"));
    }

    #[test]
    fn test_unquote() {
        assert_eq!(unquote("> text"), "text");
        assert_eq!(unquote(">text"), "text");
        assert_eq!(unquote("no prefix"), "no prefix");
    }

    #[test]
    fn test_quote_depth() {
        assert_eq!(find_quote_depth(">>> deep"), 3);
        assert_eq!(find_quote_depth("> single"), 1);
        assert_eq!(find_quote_depth("none"), 0);
    }

    #[test]
    fn test_compute_quoted_percent() {
        let body = make_body(&["> q1", "not q", "> q2", "not q"]);
        assert_eq!(compute_quoted_percent(&body), 50);
    }

    #[test]
    fn test_is_sig_start() {
        assert!(is_sig_start("-- "));
        assert!(is_sig_start("--"));
        assert!(!is_sig_start("not sig"));
        assert!(!is_sig_start("---"));
    }

    #[test]
    fn test_find_quote_prefix() {
        let body = make_body(&["> a", "> b", "normal", "> c"]);
        let prefix = find_quote_prefix(&body);
        assert!(prefix.is_some());
    }

    #[test]
    fn test_get_quote_prefix() {
        assert_eq!(get_quote_prefix(), "> ");
    }

    #[test]
    fn test_find_quote_class_depth_1() {
        assert_eq!(find_quote_class("> text"), "hm-quote-1");
    }

    #[test]
    fn test_find_quote_class_depth_3() {
        assert_eq!(find_quote_class(">>> text"), "hm-quote-3");
    }

    #[test]
    fn test_find_quote_class_non_quote() {
        assert_eq!(find_quote_class("normal text"), "");
    }

    #[test]
    fn test_remove_hypermail_tags_strips_anchor() {
        let line = r#"<a name="msg1"></a>Some text"#;
        let result = remove_hypermail_tags(line);
        assert!(!result.contains("<a name="));
        assert!(result.contains("Some text"));
    }

    #[test]
    fn test_remove_hypermail_tags_no_tags_unchanged() {
        let line = "plain text without tags";
        assert_eq!(remove_hypermail_tags(line), line);
    }

    #[test]
    fn test_compute_quoted_percent_zero() {
        let body = make_body(&["line 1", "line 2", "line 3"]);
        assert_eq!(compute_quoted_percent(&body), 0);
    }

    #[test]
    fn test_compute_quoted_percent_empty() {
        let body = BodyChain { bodies: Vec::new() };
        assert_eq!(compute_quoted_percent(&body), 0);
    }

    #[test]
    fn test_is_sig_start_cr_variant() {
        assert!(is_sig_start("-- \r"));
    }

    #[test]
    fn test_find_quote_prefix_no_quotes() {
        let body = make_body(&["normal", "also normal"]);
        let prefix = find_quote_prefix(&body);
        assert!(prefix.is_none());
    }
}
