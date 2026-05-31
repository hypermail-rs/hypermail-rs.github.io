use std::collections::HashMap;

pub const HASHSIZE: usize = 673;
pub const MAXLINE: usize = 1024;
pub const NAMESTRLEN: usize = 80;
pub const MAILSTRLEN: usize = 80;
pub const DATESTRLEN: usize = 80;
pub const SUBJSTRLEN: usize = 256;
pub const URLSTRLEN: usize = 256;

pub const BASEYEAR: i32 = 1970;

pub const PROGNAME: &str = "hypermail";
pub const HMURL: &str = "http://www.hypermail-project.org/";
pub const INDEXNAME: &str = "index";
pub const DIRNAME: &str = "archive";

pub const NONAME: &str = "(no name)";
pub const NODATE: &str = "(no date)";
pub const NOEMAIL: &str = "(no email)";
pub const NOSUBJECT: &str = "(no subject)";

pub const GDBM_INDEX_NAME: &str = ".hm2index";
pub const HAOF_NAME: &str = "archive_overview.haof";

pub const FILE_SUFFIXER: &str = "part";
pub const DIR_PREFIXER: &str = "att-";
pub const REPLACEMENT_CHAR: char = '_';
pub const META_DIR: &str = ".meta";
pub const META_EXTENSION: &str = ".meta";

pub const PATH_SEPARATOR: char = '/';

pub const PAGE_TOP: i32 = 1;
pub const PAGE_BOTTOM: i32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexType {
    Date,
    Thread,
    Subject,
    Author,
    Attachment,
    Folders,
    NoIndex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilteredReason {
    Delete = 1,
    Expire = 2,
    FilteredOut = 4,
    FilteredRequired = 8,
    FilteredOld = 16,
    FilteredNew = 32,
}

#[derive(Debug, Clone)]
pub struct Push {
    pub string: String,
}

impl Default for Push {
    fn default() -> Self {
        Self::new()
    }
}

impl Push {
    pub fn new() -> Self {
        Push { string: String::new() }
    }

    pub fn push_byte(&mut self, b: u8) {
        self.string.push(b as char);
    }

    pub fn push_str(&mut self, s: &str) {
        self.string.push_str(s);
    }

    pub fn push_nstring(&mut self, s: &str, n: usize) {
        let len = s.len().min(n);
        self.string.push_str(&s[..len]);
    }

    pub fn len(&self) -> usize {
        self.string.len()
    }

    pub fn is_empty(&self) -> bool {
        self.string.is_empty()
    }

    pub fn as_str(&self) -> &str {
        &self.string
    }

    pub fn into_string(self) -> String {
        self.string
    }
}

#[derive(Debug, Clone)]
pub struct Body {
    pub line: String,
    pub html: bool,
    pub header: bool,
    pub parsed_header: bool,
    pub attached: bool,
    pub demimed: bool,
    pub msgnum: i32,
}

#[derive(Debug, Clone)]
pub struct BodyChain {
    pub bodies: Vec<Body>,
}

#[derive(Debug, Clone)]
pub struct Reply {
    pub from_msgnum: i32,
    pub msgnum: i32,
    pub data: Option<usize>,
    pub maybe_reply: i32,
}

#[derive(Debug, Clone)]
pub struct HmList {
    pub values: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct HashEmail {
    pub email_index: usize,
}

#[derive(Debug, Clone)]
pub struct EmailSubdir {
    pub first_email: Option<usize>,
    pub last_email: Option<usize>,
    pub subdir: String,
    pub full_path: String,
    pub rel_path_to_top: String,
    pub count: i32,
    pub description: Option<String>,
    pub a_date: i64,
}

#[derive(Debug, Clone)]
pub struct EmailInfo {
    pub msgnum: i32,
    pub name: Option<String>,
    pub email_addr: Option<String>,
    pub from_date_str: Option<String>,
    pub from_date: i64,
    pub date_str: Option<String>,
    pub date: i64,
    pub msgid: Option<String>,
    pub subject: Option<String>,
    pub unre_subject: Option<String>,
    pub inreplyto: Option<String>,
    pub charset: Option<String>,
    pub datenum: i64,
    pub flags: i32,
    pub initial_next_in_thread: i32,
    pub bodylist: BodyChain,
    pub replylist: Vec<Reply>,
    pub is_reply: bool,
    pub subdir: Option<usize>,
    pub exp_time: i64,
    pub is_deleted: i32,
    pub deletion_completed: i32,
}

impl Default for EmailInfo {
    fn default() -> Self {
        EmailInfo {
            msgnum: 0,
            name: None,
            email_addr: None,
            from_date_str: None,
            from_date: 0,
            date_str: None,
            date: 0,
            msgid: None,
            subject: None,
            unre_subject: None,
            inreplyto: None,
            charset: None,
            datenum: 0,
            flags: 0,
            initial_next_in_thread: 0,
            bodylist: BodyChain { bodies: Vec::new() },
            replylist: Vec::new(),
            is_reply: false,
            subdir: None,
            exp_time: 0,
            is_deleted: 0,
            deletion_completed: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Header {
    pub email_index: usize,
    pub left: Option<Box<Header>>,
    pub right: Option<Box<Header>>,
}

#[derive(Debug, Clone)]
pub struct Attachment {
    pub content_type: String,
    pub name: Option<String>,
    pub id: Option<String>,
    pub stored_as: Option<String>,
    pub descr: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AttachmentItem {
    pub attachment: Attachment,
}

#[derive(Debug, Clone, Default)]
pub struct ArchiveState {
    pub emails: Vec<EmailInfo>,
    pub subject_list: Option<Box<Header>>,
    pub author_list: Option<Box<Header>>,
    pub date_list: Option<Box<Header>>,
    pub deleted_list: Vec<usize>,
    pub reply_list: Vec<Reply>,
    pub thread_list: Vec<Reply>,
    pub etable: HashMap<String, Vec<usize>>,
    pub msgid_table: HashMap<String, usize>,
    pub folders: Vec<EmailSubdir>,
    pub max_msgnum: i32,
}
