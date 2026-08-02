use crate::message::BodyChain;
use crate::structs::EmailStore;
use std::collections::HashMap;

/// Links replies by scanning message bodies for quoted Message-IDs.
///
/// # Performance
///
/// Builds a single msgid → msgnum lookup table, then for each message scans
/// its body once for `<...>`-delimited tokens and checks each against the
/// table. This is O(total body bytes) rather than the naive O(messages² ×
/// body bytes) of re-running `contains()` against every other message's
/// msgid for every message.
pub fn link_quotes(store: &mut EmailStore, ignore_types: bool, linkquotes: bool) {
    if !linkquotes || ignore_types {
        return;
    }

    let msgid_to_msgnum: HashMap<&str, i32> = store
        .emails
        .iter()
        .filter_map(|e| e.msgid.as_deref().map(|mid| (mid, e.msgnum)))
        .collect();

    let mut new_replies = Vec::new();
    for email in &store.emails {
        let to_msgnum = email.msgnum;
        let from_text = collect_body_text(&email.bodylist);
        for candidate in extract_angle_bracket_tokens(&from_text) {
            if let Some(&from_msgnum) = msgid_to_msgnum.get(candidate) {
                if from_msgnum != to_msgnum {
                    new_replies.push(crate::message::Reply {
                        from_msgnum,
                        msgnum: to_msgnum,
                        data: None,
                        maybe_reply: 1,
                    });
                }
            }
        }
    }
    store.replylist.extend(new_replies);
}

/// Yields each `<...>`-delimited token in `text` (e.g. `<a@b.com>`), without allocating.
fn extract_angle_bracket_tokens(text: &str) -> impl Iterator<Item = &str> {
    let mut rest = text;
    std::iter::from_fn(move || {
        let start = rest.find('<')?;
        let after_start = &rest[start + 1..];
        match after_start.find('>') {
            Some(end) => {
                let consumed = start + 1 + end + 1;
                let token = &rest[start..consumed];
                rest = &rest[consumed..];
                Some(token)
            },
            None => None,
        }
    })
}

fn collect_body_text(body_chain: &BodyChain) -> String {
    let mut text = String::new();
    for body in &body_chain.bodies {
        if !body.attached && !body.header {
            text.push_str(&body.line);
            text.push(' ');
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{Body, BodyChain, EmailInfo};

    #[test]
    fn test_link_quotes_empty_store() {
        let mut store = EmailStore::new();
        link_quotes(&mut store, false, true);
        assert!(store.replylist.is_empty());
    }

    #[test]
    fn test_link_quotes_disabled() {
        let mut store = EmailStore::new();
        let e1 = EmailInfo {
            msgnum: 1,
            msgid: Some("<a@b>".to_string()),
            bodylist: BodyChain {
                bodies: vec![Body {
                    line: "reply to <a@b>".to_string(),
                    html: false,
                    header: false,
                    parsed_header: false,
                    attached: false,
                    demimed: false,
                    msgnum: 0,
                }],
            },
            ..Default::default()
        };
        let e2 = EmailInfo {
            msgnum: 2,
            msgid: Some("<b@c>".to_string()),
            bodylist: BodyChain { bodies: Vec::new() },
            ..Default::default()
        };
        store.add_email(e1);
        store.add_email(e2);
        link_quotes(&mut store, false, false);
        assert!(store.replylist.is_empty());
    }

    #[test]
    fn test_link_quotes_finds_quoted_msgid() {
        let mut store = EmailStore::new();
        let e1 = EmailInfo {
            msgnum: 1,
            msgid: Some("<a@b>".to_string()),
            bodylist: BodyChain { bodies: Vec::new() },
            ..Default::default()
        };
        let e2 = EmailInfo {
            msgnum: 2,
            msgid: Some("<c@d>".to_string()),
            bodylist: BodyChain {
                bodies: vec![Body {
                    line: "On Mon, someone wrote: <a@b>".to_string(),
                    html: false,
                    header: false,
                    parsed_header: false,
                    attached: false,
                    demimed: false,
                    msgnum: 0,
                }],
            },
            ..Default::default()
        };
        store.add_email(e1);
        store.add_email(e2);
        link_quotes(&mut store, false, true);
        assert_eq!(store.replylist.len(), 1);
        assert_eq!(store.replylist[0].from_msgnum, 1);
        assert_eq!(store.replylist[0].msgnum, 2);
    }

    #[test]
    fn test_extract_angle_bracket_tokens() {
        let text = "reply to <a@b.com> and also <c@d.com> end";
        let tokens: Vec<&str> = extract_angle_bracket_tokens(text).collect();
        assert_eq!(tokens, vec!["<a@b.com>", "<c@d.com>"]);
    }

    #[test]
    fn test_extract_angle_bracket_tokens_unclosed() {
        let text = "no closing bracket <a@b.com";
        let tokens: Vec<&str> = extract_angle_bracket_tokens(text).collect();
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_collect_body_text() {
        let chain = BodyChain {
            bodies: vec![
                Body {
                    line: "hello".to_string(),
                    html: false,
                    header: false,
                    parsed_header: false,
                    attached: false,
                    demimed: false,
                    msgnum: 0,
                },
                Body {
                    line: "world".to_string(),
                    html: false,
                    header: false,
                    parsed_header: false,
                    attached: true,
                    demimed: false,
                    msgnum: 0,
                },
            ],
        };
        let text = collect_body_text(&chain);
        assert!(!text.contains("world"));
        assert!(text.contains("hello"));
    }
}
