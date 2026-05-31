use crate::message::{Body, BodyChain, EmailInfo, Header, HmList, Reply};
use regex::Regex;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct EmailStore {
    pub emails: Vec<EmailInfo>,
    pub subject_list: Option<Box<Header>>,
    pub author_list: Option<Box<Header>>,
    pub date_list: Option<Box<Header>>,
    pub msgid_table: HashMap<String, usize>,
    pub msgnum_table: HashMap<i32, usize>,
    pub threadlist: Vec<Reply>,
    pub threadlist_by_msgnum: Vec<Option<usize>>,
    pub replylist: Vec<Reply>,
    pub max_msgnum: i32,
}

impl Default for EmailStore {
    fn default() -> Self {
        Self::new()
    }
}

impl EmailStore {
    pub fn new() -> Self {
        EmailStore {
            emails: Vec::new(),
            subject_list: None,
            author_list: None,
            date_list: None,
            msgid_table: HashMap::new(),
            msgnum_table: HashMap::new(),
            threadlist: Vec::new(),
            threadlist_by_msgnum: Vec::new(),
            replylist: Vec::new(),
            max_msgnum: -1,
        }
    }

    pub fn reinit(&mut self) {
        self.emails.clear();
        self.subject_list = None;
        self.author_list = None;
        self.date_list = None;
        self.msgid_table.clear();
        self.msgnum_table.clear();
        self.threadlist.clear();
        self.threadlist_by_msgnum.clear();
        self.replylist.clear();
        self.max_msgnum = -1;
    }

    pub fn find_by_msgid(&self, msgid: &str) -> Option<usize> {
        self.msgid_table.get(msgid).copied()
    }

    pub fn find_by_msgnum(&self, msgnum: i32) -> Option<usize> {
        self.msgnum_table.get(&msgnum).copied()
    }

    pub fn add_email(&mut self, email: EmailInfo) -> usize {
        let idx = self.emails.len();
        let msgnum = email.msgnum;

        if let Some(ref msgid) = email.msgid {
            self.msgid_table.insert(msgid.clone(), idx);
        }

        self.msgnum_table.insert(msgnum, idx);

        if msgnum > self.max_msgnum {
            self.max_msgnum = msgnum;
        }

        self.emails.push(email);
        idx
    }

    pub fn insert_into_subject_list(&mut self, idx: usize) {
        self.subject_list = Self::insert_into_tree_by_field(
            self.subject_list.take(),
            idx,
            &self.emails,
            |e| e.unre_subject.as_deref().or(e.subject.as_deref()).unwrap_or("").to_lowercase(),
            |e| e.msgnum,
        );
    }

    pub fn insert_into_author_list(&mut self, idx: usize) {
        self.author_list = Self::insert_into_tree_by_field(
            self.author_list.take(),
            idx,
            &self.emails,
            |e| e.name.as_deref().or(e.email_addr.as_deref()).unwrap_or("").to_lowercase(),
            |e| e.msgnum,
        );
    }

    pub fn insert_into_date_list(&mut self, idx: usize) {
        self.date_list = Self::insert_into_tree_by_field(
            self.date_list.take(),
            idx,
            &self.emails,
            |e| format!("{:020}", e.date),
            |e| e.msgnum,
        );
    }

    fn insert_into_tree_by_field<F1, F2>(
        node: Option<Box<Header>>,
        idx: usize,
        emails: &[EmailInfo],
        field_fn: F1,
        msgnum_fn: F2,
    ) -> Option<Box<Header>>
    where
        F1: Fn(&EmailInfo) -> String,
        F2: Fn(&EmailInfo) -> i32,
    {
        let email = &emails[idx];
        let key = field_fn(email);
        let msgnum = msgnum_fn(email);

        match node {
            None => Some(Box::new(Header { email_index: idx, left: None, right: None })),
            Some(mut n) => {
                let node_email = &emails[n.email_index];
                let node_key = field_fn(node_email);
                let node_msgnum = msgnum_fn(node_email);

                if key < node_key || (key == node_key && msgnum < node_msgnum) {
                    n.left =
                        Self::insert_into_tree_by_field(n.left, idx, emails, field_fn, msgnum_fn);
                } else {
                    n.right =
                        Self::insert_into_tree_by_field(n.right, idx, emails, field_fn, msgnum_fn);
                }
                Some(n)
            },
        }
    }

    pub fn traverse_date_list(&self) -> Vec<usize> {
        let mut result = Vec::new();
        Self::inorder_traversal(&self.date_list, &mut result);
        result
    }

    pub fn traverse_subject_list(&self) -> Vec<usize> {
        let mut result = Vec::new();
        Self::inorder_traversal(&self.subject_list, &mut result);
        result
    }

    pub fn traverse_author_list(&self) -> Vec<usize> {
        let mut result = Vec::new();
        Self::inorder_traversal(&self.author_list, &mut result);
        result
    }

    fn inorder_traversal(node: &Option<Box<Header>>, result: &mut Vec<usize>) {
        if let Some(n) = node {
            Self::inorder_traversal(&n.left, result);
            result.push(n.email_index);
            Self::inorder_traversal(&n.right, result);
        }
    }
}

pub fn add_body(mut bodylist: BodyChain, line: &str, msgnum: i32) -> BodyChain {
    bodylist.bodies.push(Body {
        line: line.to_string(),
        html: false,
        header: false,
        parsed_header: false,
        attached: false,
        demimed: false,
        msgnum,
    });
    bodylist
}

pub fn inlist(list: &HmList, val: &str) -> bool {
    list.values.iter().any(|v| v == val)
}

pub fn inlist_pos(list: &HmList, val: &str) -> Option<usize> {
    list.values.iter().position(|v| v == val)
}

pub fn inlist_regex_pos(list: &HmList, pattern: &str) -> Option<usize> {
    let re = Regex::new(pattern).ok()?;
    list.values.iter().position(|v| re.is_match(v))
}

pub fn add_to_list(list: &mut HmList, val: &str) {
    if !inlist(list, val) {
        list.values.push(val.to_string());
    }
}

pub fn add_to_list_multi(list: &mut HmList, vals: &str) {
    for v in vals.split_whitespace() {
        add_to_list(list, v);
    }
}

pub fn link_reply(
    replylist: &mut Vec<Reply>,
    from_msgnum: i32,
    to_msgnum: i32,
    data: Option<usize>,
    maybe_reply: bool,
) {
    replylist.push(Reply {
        from_msgnum,
        msgnum: to_msgnum,
        data,
        maybe_reply: if maybe_reply { 1 } else { 0 },
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_email(msgnum: i32, msgid: &str, subject: &str, name: &str, date: i64) -> EmailInfo {
        EmailInfo {
            msgnum,
            msgid: Some(msgid.to_string()),
            subject: Some(subject.to_string()),
            name: Some(name.to_string()),
            date,
            ..Default::default()
        }
    }

    #[test]
    fn test_add_and_find_email() {
        let mut store = EmailStore::new();
        let email = make_email(1, "<test@example.com>", "Test", "Alice", 1000000);
        let idx = store.add_email(email);
        assert_eq!(idx, 0);
        assert_eq!(store.find_by_msgid("<test@example.com>"), Some(0));
        assert_eq!(store.find_by_msgnum(1), Some(0));
    }

    #[test]
    fn test_date_sorting() {
        let mut store = EmailStore::new();
        let e1 = make_email(1, "<a@e>", "Z", "Zoe", 300);
        let e2 = make_email(2, "<b@e>", "A", "Alice", 100);
        let e3 = make_email(3, "<c@e>", "M", "Bob", 200);
        store.add_email(e1);
        store.add_email(e2);
        store.add_email(e3);

        store.insert_into_date_list(0);
        store.insert_into_date_list(1);
        store.insert_into_date_list(2);

        let sorted = store.traverse_date_list();
        assert_eq!(sorted.len(), 3);
        assert_eq!(store.emails[sorted[0]].msgnum, 2); // date=100
        assert_eq!(store.emails[sorted[1]].msgnum, 3); // date=200
        assert_eq!(store.emails[sorted[2]].msgnum, 1); // date=300
    }

    #[test]
    fn test_subject_sorting() {
        let mut store = EmailStore::new();
        let e1 = make_email(1, "<a@e>", "Zebra", "Zoe", 100);
        let e2 = make_email(2, "<b@e>", "Alpha", "Alice", 100);
        let e3 = make_email(3, "<c@e>", "Beta", "Bob", 100);
        store.add_email(e1);
        store.add_email(e2);
        store.add_email(e3);

        store.insert_into_subject_list(0);
        store.insert_into_subject_list(1);
        store.insert_into_subject_list(2);

        let sorted = store.traverse_subject_list();
        assert_eq!(sorted.len(), 3);
        assert_eq!(store.emails[sorted[0]].subject.as_deref(), Some("Alpha"));
        assert_eq!(store.emails[sorted[1]].subject.as_deref(), Some("Beta"));
        assert_eq!(store.emails[sorted[2]].subject.as_deref(), Some("Zebra"));
    }

    #[test]
    fn test_inlist() {
        let list = HmList { values: vec!["a".to_string(), "b".to_string()] };
        assert!(inlist(&list, "a"));
        assert!(inlist(&list, "b"));
        assert!(!inlist(&list, "c"));
    }

    #[test]
    fn test_add_to_list() {
        let mut list = HmList { values: Vec::new() };
        add_to_list(&mut list, "test");
        assert!(inlist(&list, "test"));
        add_to_list(&mut list, "test");
        assert_eq!(list.values.len(), 1);
    }

    #[test]
    fn test_reinit() {
        let mut store = EmailStore::new();
        let email = make_email(1, "<a@e>", "T", "A", 100);
        store.add_email(email);
        assert_eq!(store.emails.len(), 1);
        store.reinit();
        assert_eq!(store.emails.len(), 0);
        assert!(store.msgid_table.is_empty());
    }

    #[test]
    fn test_subject_sorting_uses_unre_subject() {
        // "Re: Alpha" should sort with "Alpha", not after "Zebra"
        let mut store = EmailStore::new();

        let mut e1 = make_email(1, "<a@e>", "Zebra", "Alice", 100);
        e1.unre_subject = Some("zebra".to_string());

        let mut e2 = make_email(2, "<b@e>", "Re: Alpha", "Bob", 200);
        e2.unre_subject = Some("alpha".to_string());

        let mut e3 = make_email(3, "<c@e>", "Alpha", "Carol", 300);
        e3.unre_subject = Some("alpha".to_string());

        store.add_email(e1);
        store.add_email(e2);
        store.add_email(e3);
        store.insert_into_subject_list(0);
        store.insert_into_subject_list(1);
        store.insert_into_subject_list(2);

        let sorted = store.traverse_subject_list();
        assert_eq!(sorted.len(), 3);
        // Both "Alpha" and "Re: Alpha" have unre_subject="alpha" → they sort before "Zebra"
        let subjects: Vec<_> =
            sorted.iter().map(|&i| store.emails[i].subject.as_deref().unwrap()).collect();
        let zebra_pos = subjects.iter().position(|&s| s == "Zebra").unwrap();
        let re_alpha_pos = subjects.iter().position(|&s| s == "Re: Alpha").unwrap();
        let alpha_pos = subjects.iter().position(|&s| s == "Alpha").unwrap();
        assert!(zebra_pos > re_alpha_pos, "Re: Alpha should sort before Zebra");
        assert!(zebra_pos > alpha_pos, "Alpha should sort before Zebra");
    }

    #[test]
    fn test_author_sorting_falls_back_to_email_addr() {
        let mut store = EmailStore::new();

        // Has a name — sorts by name
        let e1 = EmailInfo {
            msgnum: 1,
            msgid: Some("<a@e>".to_string()),
            name: Some("Zoe".to_string()),
            email_addr: Some("zoe@example.com".to_string()),
            date: 100,
            ..Default::default()
        };
        // No name — should fall back to email_addr "amy@example.com" for sorting
        let e2 = EmailInfo {
            msgnum: 2,
            msgid: Some("<b@e>".to_string()),
            name: None,
            email_addr: Some("amy@example.com".to_string()),
            date: 200,
            ..Default::default()
        };
        // Another no-name — falls back to "mid@example.com"
        let e3 = EmailInfo {
            msgnum: 3,
            msgid: Some("<c@e>".to_string()),
            name: None,
            email_addr: Some("mid@example.com".to_string()),
            date: 300,
            ..Default::default()
        };

        store.add_email(e1);
        store.add_email(e2);
        store.add_email(e3);
        store.insert_into_author_list(0);
        store.insert_into_author_list(1);
        store.insert_into_author_list(2);

        let sorted = store.traverse_author_list();
        assert_eq!(sorted.len(), 3);
        // Expected order by sort key: amy@ < mid@ < Zoe
        assert_eq!(store.emails[sorted[0]].email_addr.as_deref(), Some("amy@example.com"));
        assert_eq!(store.emails[sorted[1]].email_addr.as_deref(), Some("mid@example.com"));
        assert_eq!(store.emails[sorted[2]].name.as_deref(), Some("Zoe"));
    }

    #[test]
    fn test_author_sorting_no_name_no_email_sorts_to_front() {
        // Both name and email_addr are None → key is "" → sorts before everything
        let mut store = EmailStore::new();
        let e1 = make_email(1, "<a@e>", "T", "Bob", 100);
        let e2 = EmailInfo {
            msgnum: 2,
            msgid: Some("<b@e>".to_string()),
            name: None,
            email_addr: None,
            date: 200,
            ..Default::default()
        };
        store.add_email(e1);
        store.add_email(e2);
        store.insert_into_author_list(0);
        store.insert_into_author_list(1);
        let sorted = store.traverse_author_list();
        // "" < "bob" → nameless/emailless sorts first
        assert_eq!(store.emails[sorted[0]].msgnum, 2);
        assert_eq!(store.emails[sorted[1]].name.as_deref(), Some("Bob"));
    }

    #[test]
    fn test_add_body() {
        let chain = crate::message::BodyChain { bodies: Vec::new() };
        let chain = add_body(chain, "Hello World", 1);
        assert_eq!(chain.bodies.len(), 1);
        assert_eq!(chain.bodies[0].line, "Hello World");
        assert_eq!(chain.bodies[0].msgnum, 1);
        assert!(!chain.bodies[0].attached);
        assert!(!chain.bodies[0].header);
    }

    #[test]
    fn test_inlist_pos_found() {
        let list = HmList { values: vec!["a".to_string(), "b".to_string(), "c".to_string()] };
        assert_eq!(inlist_pos(&list, "b"), Some(1));
    }

    #[test]
    fn test_inlist_pos_not_found() {
        let list = HmList { values: vec!["a".to_string()] };
        assert_eq!(inlist_pos(&list, "z"), None);
    }

    #[test]
    fn test_inlist_regex_pos_found() {
        let list = HmList { values: vec!["foo".to_string(), "bar123".to_string()] };
        assert_eq!(inlist_regex_pos(&list, r"bar\d+"), Some(1));
    }

    #[test]
    fn test_inlist_regex_pos_not_found() {
        let list = HmList { values: vec!["foo".to_string()] };
        assert_eq!(inlist_regex_pos(&list, "xyz"), None);
    }

    #[test]
    fn test_add_to_list_multi() {
        let mut list = HmList { values: Vec::new() };
        add_to_list_multi(&mut list, "a b c a");
        assert_eq!(list.values.len(), 3);
        assert!(inlist(&list, "a"));
        assert!(inlist(&list, "b"));
        assert!(inlist(&list, "c"));
    }

    #[test]
    fn test_link_reply_adds_to_replylist() {
        let mut replylist = Vec::new();
        link_reply(&mut replylist, 1, 2, None, false);
        assert_eq!(replylist.len(), 1);
        assert_eq!(replylist[0].from_msgnum, 1);
        assert_eq!(replylist[0].msgnum, 2);
        assert_eq!(replylist[0].maybe_reply, 0);
    }

    #[test]
    fn test_link_reply_maybe_flag() {
        let mut replylist = Vec::new();
        link_reply(&mut replylist, 3, 4, Some(0), true);
        assert_eq!(replylist[0].maybe_reply, 1);
    }
}
