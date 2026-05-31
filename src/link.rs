use once_cell::sync::Lazy;
use regex::Regex;

use crate::message::BodyChain;
use crate::structs::EmailStore;

#[allow(dead_code)]
static MSGID_QUOTE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^On\s.*\n?\s*wrote:\s*$").unwrap());

#[allow(dead_code)]
static EMAIL_QUOTE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"<([a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,})>").unwrap());

pub fn link_quotes(store: &mut EmailStore, ignore_types: bool, linkquotes: bool) {
    if !linkquotes {
        return;
    }

    let msgids: Vec<Option<String>> = store.emails.iter().map(|e| e.msgid.clone()).collect();

    for i in 0..store.emails.len() {
        let body_chain = &store.emails[i].bodylist;

        if !ignore_types {
            let from_text = collect_body_text(body_chain);
            for (j, other_msgid) in msgids.iter().enumerate() {
                if i == j || other_msgid.is_none() {
                    continue;
                }
                if let Some(ref mid) = other_msgid {
                    if from_text.contains(mid) {
                        let email_info = &store.emails[i];
                        let from_msgnum = store.emails[j].msgnum;
                        let to_msgnum = email_info.msgnum;
                        if from_msgnum != to_msgnum {
                            store.replylist.push(crate::message::Reply {
                                from_msgnum,
                                msgnum: to_msgnum,
                                data: None,
                                maybe_reply: 1,
                            });
                        }
                    }
                }
            }
        }
    }
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
