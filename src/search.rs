use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::config::Config;
use crate::error::Result;
use crate::message::EmailInfo;
use crate::structs::EmailStore;

type BigramIndex = HashMap<String, Vec<usize>>;

pub fn build_bigram_index(store: &EmailStore) -> BigramIndex {
    let mut index: BigramIndex = HashMap::new();

    for (i, email) in store.emails.iter().enumerate() {
        let text = extract_searchable_text(email);
        let bigrams = extract_bigrams(&text);
        for bigram in bigrams {
            index.entry(bigram).or_default().push(i);
        }
    }

    index
}

pub fn search_bigrams(query: &str, index: &BigramIndex, _store: &EmailStore) -> Vec<usize> {
    let query_bigrams = extract_bigrams(query);
    if query_bigrams.is_empty() {
        return Vec::new();
    }

    let mut result_scores: HashMap<usize, usize> = HashMap::new();

    for bigram in &query_bigrams {
        if let Some(entries) = index.get(bigram) {
            for &idx in entries {
                *result_scores.entry(idx).or_default() += 1;
            }
        }
    }

    let threshold = query_bigrams.len().saturating_sub(1);
    let mut results: Vec<(usize, usize)> =
        result_scores.into_iter().filter(|(_, score)| *score >= threshold).collect();

    results.sort_by_key(|b| std::cmp::Reverse(b.1));

    results.into_iter().map(|(idx, _)| idx).collect()
}

fn extract_searchable_text(email: &EmailInfo) -> String {
    let mut text = String::new();

    if let Some(ref subject) = email.subject {
        text.push_str(subject);
        text.push(' ');
    }
    if let Some(ref name) = email.name {
        text.push_str(name);
        text.push(' ');
    }
    if let Some(ref addr) = email.email_addr {
        text.push_str(addr);
        text.push(' ');
    }

    for body in &email.bodylist.bodies {
        if !body.attached && !body.header {
            text.push_str(&body.line);
            text.push(' ');
        }
    }

    text
}

fn extract_bigrams(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    let chars: Vec<char> =
        lower.chars().filter(|c| c.is_alphanumeric() || c.is_whitespace()).collect();

    let mut bigrams = HashSet::new();
    for window in chars.windows(2) {
        let bigram: String = window.iter().collect();
        bigrams.insert(bigram);
    }

    bigrams.into_iter().collect()
}

pub fn write_search_index(store: &EmailStore, config: &Config) -> Result<String> {
    let dir = config.dir.as_deref().unwrap_or(".");
    let search_path = Path::new(dir).join("search_index.txt");

    let mut content = String::new();
    for email in &store.emails {
        let text = extract_searchable_text(email);
        let line = format!("{}|{}", email.msgnum, sanitize_for_search(&text));
        content.push_str(&line);
        content.push('\n');
    }

    std::fs::write(&search_path, &content)?;
    Ok(content)
}

fn sanitize_for_search(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '.' || *c == '@' || *c == '-')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{Body, BodyChain, EmailInfo};

    fn make_email(msgnum: i32, subject: &str, name: &str, body_text: &str) -> EmailInfo {
        EmailInfo {
            msgnum,
            subject: Some(subject.to_string()),
            name: Some(name.to_string()),
            bodylist: BodyChain {
                bodies: vec![Body {
                    line: body_text.to_string(),
                    html: false,
                    header: false,
                    parsed_header: false,
                    attached: false,
                    demimed: false,
                    msgnum,
                }],
            },
            ..Default::default()
        }
    }

    #[test]
    fn test_bigram_extraction() {
        let bigrams = extract_bigrams("hello");
        assert!(bigrams.contains(&"he".to_string()));
        assert!(bigrams.contains(&"el".to_string()));
        assert!(bigrams.contains(&"ll".to_string()));
        assert!(bigrams.contains(&"lo".to_string()));
    }

    #[test]
    fn test_bigram_index() {
        let mut store = EmailStore::new();
        store.add_email(make_email(1, "Hello World", "Alice", "This is a test"));
        store.add_email(make_email(2, "Goodbye World", "Bob", "Another test message"));

        let index = build_bigram_index(&store);
        assert!(index.contains_key("wo"));
        assert!(index.contains_key("te"));
    }

    #[test]
    fn test_search() {
        let mut store = EmailStore::new();
        store.add_email(make_email(1, "Hello World", "Alice", "This is a test message"));
        store.add_email(make_email(2, "Goodbye World", "Bob", "Another different message"));

        let index = build_bigram_index(&store);
        let results = search_bigrams("hello", &index, &store);
        assert_eq!(results.len(), 1);
        assert_eq!(store.emails[results[0]].msgnum, 1);
    }

    #[test]
    fn test_extract_searchable_text() {
        let email = make_email(1, "Test Subject", "Alice", "Body content");
        let text = extract_searchable_text(&email);
        assert!(text.contains("Test Subject"));
        assert!(text.contains("Alice"));
        assert!(text.contains("Body content"));
    }
}
