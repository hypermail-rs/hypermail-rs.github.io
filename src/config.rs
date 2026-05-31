use crate::error::{HypermailError, Result};
use std::path::{Path, PathBuf};

pub const ANTISPAM_AT: &str = "@";
pub const LANGUAGE: &str = "en";
pub const HTMLSUFFIX: &str = "html";
pub const DEFAULTINDEX: &str = "date";
pub const INLINE_TYPES: &str = "image/gif image/jpeg image/png";
pub const PROGRESS: i32 = 0;
pub const MAILCOMMAND: &str = "mailto:$TO?subject=$SUBJECT&in-reply-to=$ID";
pub const DOMAINADDR: &str = "";

pub const DELETE_REMOVES_FILES: i32 = 0;
pub const DELETE_LEAVES_STUBS: i32 = 1;
pub const DELETE_LEAVES_EXPIRED_TEXT: i32 = 2;
pub const DELETE_LEAVES_TEXT: i32 = 3;

/// Supported configuration value types for the hypermail config parser.
#[derive(Debug, Clone)]
pub enum ConfigType {
    String,
    Switch,
    Integer,
    List,
    StringList,
    Octal,
}

/// A single configuration entry definition with metadata for parsing.
#[derive(Debug, Clone)]
pub struct ConfigEntry {
    pub label: &'static str,
    pub flags: ConfigType,
    pub default_str: Option<&'static str>,
    pub default_int: i64,
    pub verbose: &'static str,
}

/// A whitespace-separated list of values used for multi-value config options.
#[derive(Debug, Clone)]
pub struct HmList {
    pub values: Vec<String>,
}

impl Default for HmList {
    fn default() -> Self {
        Self::new()
    }
}

impl HmList {
    /// Creates an empty list.
    pub fn new() -> Self {
        HmList { values: Vec::new() }
    }

    /// Creates a list by splitting a whitespace-delimited string.
    pub fn from_whitespace_str(s: &str) -> Self {
        let values: Vec<String> = s.split_whitespace().map(|s| s.to_string()).collect();
        HmList { values }
    }

    /// Returns true if the list contains the given value.
    pub fn contains(&self, val: &str) -> bool {
        self.values.iter().any(|v| v == val)
    }

    /// Adds a value to the list if not already present.
    pub fn add(&mut self, val: &str) {
        if !self.contains(val) {
            self.values.push(val.to_string());
        }
    }

    /// Splits a whitespace-delimited string and adds each token to the list.
    pub fn add_list(&mut self, val: &str) {
        for v in val.split_whitespace() {
            self.add(v);
        }
    }
}

/// Complete runtime configuration for a hypermail archive run.
///
/// Controls input/output paths, HTML generation options, index types,
/// spam protection, i18n, MIME handling, and template customization.
#[derive(Debug, Clone)]
pub struct Config {
    // --- String configs ---
    pub fragment_prefix: String,
    pub htmlmessage_deleted: Option<String>,
    pub antispam_at: String,
    pub antispamdomain: Option<String>,
    pub language: String,
    pub htmlsuffix: String,
    pub mbox: Option<String>,
    pub archives: Option<String>,
    pub custom_archives: Option<String>,
    pub about: Option<String>,
    pub label: Option<String>,
    pub dir: Option<String>,
    pub defaultindex: String,
    pub default_top_index: String,
    pub mailcommand: String,
    pub newmsg_command: String,
    pub replymsg_command: String,
    pub inreplyto_command: Option<String>,
    pub mailto: Option<String>,
    pub hmail: Option<String>,
    pub domainaddr: Option<String>,
    pub css: Option<String>,
    pub icss_url: Option<String>,
    pub mcss_url: Option<String>,
    pub dateformat: Option<String>,
    pub indexdateformat: Option<String>,
    pub stripsubject: Option<String>,
    pub link_to_replies: Option<String>,
    pub quote_link_string: Option<String>,
    pub ihtmlheader: Option<String>,
    pub ihtmlfooter: Option<String>,
    pub ihtmlhead: Option<String>,
    pub ihtmlhelpup: Option<String>,
    pub ihtmlhelplow: Option<String>,
    pub ihtmlnavbar2up: Option<String>,
    pub mhtmlheader: Option<String>,
    pub mhtmlfooter: Option<String>,
    pub attachmentlink: Option<String>,
    pub bodyheader: Option<String>,
    pub bodyheaderend: Option<String>,
    pub bodyfooter: Option<String>,
    pub unsafe_chars: Option<String>,
    pub filename_base: Option<String>,
    pub folder_by_date: Option<String>,
    pub latest_folder: Option<String>,
    pub base_url: Option<String>,
    pub describe_folder: Option<String>,
    pub delete_older: Option<String>,
    pub delete_newer: Option<String>,
    pub alts_text: Option<String>,
    pub description: Option<String>,
    pub theme: Option<String>,
    pub append_filename: Option<String>,
    pub txtsuffix: Option<String>,

    // --- Switch (bool) configs ---
    pub email_address_obfuscation: bool,
    pub i18n: bool,
    pub i18n_body: bool,
    pub overwrite: bool,
    pub inlinehtml: bool,
    pub readone: bool,
    pub reverse: bool,
    pub reverse_folders: bool,
    pub showheaders: bool,
    pub showbr: bool,
    pub showreplies: bool,
    pub indextable: bool,
    pub iquotes: bool,
    pub eurodate: bool,
    pub gmtime: bool,
    pub isodate: bool,
    pub require_msgids: bool,
    pub discard_dup_msgids: bool,
    pub usemeta: bool,
    pub uselock: bool,
    pub ietf_mbox: bool,
    pub linkquotes: bool,
    pub monthly_index: bool,
    pub yearly_index: bool,
    pub spamprotect: bool,
    pub spamprotect_id: bool,
    pub attachmentsindex: bool,
    pub usegdbm: bool,
    pub writehaof: bool,
    pub append: bool,
    pub nonsequential: bool,
    pub warn_surpressions: bool,
    pub files_by_thread: bool,
    pub href_detection: bool,
    pub mbox_shortened: bool,
    pub report_new_file: bool,
    pub report_new_folder: bool,
    pub use_sender_date: bool,
    pub inline_addlink: bool,
    pub iso2022jp: bool,
    pub delete_incremental: bool,
    pub showgenerator: bool,
    pub show_warnings: bool,

    // --- Integer configs ---
    pub increment: i32,
    pub showhtml: i32,
    pub show_msg_links: i32,
    pub show_index_links: i32,
    pub thrdlevels: i32,
    pub dirmode: i32,
    pub filemode: i32,
    pub locktime: i32,
    pub searchbackmsgnum: i32,
    pub quote_hide_threshold: i32,
    pub thread_file_depth: i32,
    pub startmsgnum: i32,
    pub msgsperfolder: i32,
    pub save_alts: i32,
    pub delete_level: i32,
    pub progress: i32,
    pub max_message_size: usize,

    // --- List configs ---
    pub show_headers: HmList,
    pub avoid_indices: HmList,
    pub avoid_top_indices: HmList,
    pub skip_headers: HmList,
    pub text_types: HmList,
    pub inline_types: HmList,
    pub prefered_types: HmList,
    pub ignore_types: HmList,
    pub filter_out: HmList,
    pub filter_require: HmList,
    pub filter_out_full_body: HmList,
    pub filter_require_full_body: HmList,
    pub deleted: HmList,
    pub expires: HmList,
    pub delete_msgnum: HmList,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            fragment_prefix: "msg".to_string(),
            htmlmessage_deleted: None,
            antispam_at: ANTISPAM_AT.to_string(),
            antispamdomain: None,
            language: LANGUAGE.to_string(),
            htmlsuffix: HTMLSUFFIX.to_string(),
            mbox: None,
            archives: None,
            custom_archives: None,
            about: None,
            label: None,
            dir: None,
            defaultindex: DEFAULTINDEX.to_string(),
            default_top_index: "folders".to_string(),
            mailcommand: MAILCOMMAND.to_string(),
            newmsg_command: "mailto:$TO".to_string(),
            replymsg_command: MAILCOMMAND.to_string(),
            inreplyto_command: None,
            mailto: None,
            hmail: None,
            domainaddr: None,
            css: None,
            icss_url: None,
            mcss_url: None,
            dateformat: None,
            indexdateformat: None,
            stripsubject: None,
            link_to_replies: None,
            quote_link_string: None,
            ihtmlheader: None,
            ihtmlfooter: None,
            ihtmlhead: None,
            ihtmlhelpup: None,
            ihtmlhelplow: None,
            ihtmlnavbar2up: None,
            mhtmlheader: None,
            mhtmlfooter: None,
            attachmentlink: None,
            unsafe_chars: None,
            filename_base: None,
            folder_by_date: None,
            latest_folder: None,
            base_url: None,
            describe_folder: None,
            delete_older: None,
            delete_newer: None,
            alts_text: None,
            append_filename: None,
            txtsuffix: None,
            description: None,
            theme: None,
            bodyheader: None,
            bodyheaderend: None,
            bodyfooter: None,
            email_address_obfuscation: false,
            i18n: false,
            i18n_body: false,
            overwrite: false,
            inlinehtml: true,
            readone: false,
            reverse: false,
            reverse_folders: false,
            showheaders: true,
            showbr: true,
            showreplies: true,
            indextable: false,
            iquotes: true,
            eurodate: true,
            gmtime: false,
            isodate: false,
            require_msgids: true,
            discard_dup_msgids: true,
            usemeta: false,
            uselock: true,
            ietf_mbox: false,
            linkquotes: false,
            monthly_index: false,
            yearly_index: false,
            spamprotect: true,
            spamprotect_id: true,
            attachmentsindex: true,
            usegdbm: false,
            writehaof: false,
            append: false,
            nonsequential: false,
            warn_surpressions: true,
            files_by_thread: false,
            href_detection: true,
            mbox_shortened: false,
            report_new_file: false,
            report_new_folder: false,
            use_sender_date: false,
            inline_addlink: true,
            iso2022jp: false,
            delete_incremental: true,
            showgenerator: true,
            show_warnings: false,
            increment: 0,
            showhtml: 1,
            show_msg_links: 1,
            show_index_links: 1,
            thrdlevels: 50, // High default to show full tree structure
            dirmode: 0o755,
            filemode: 0o644,
            locktime: 3600,
            searchbackmsgnum: 500,
            quote_hide_threshold: 100,
            thread_file_depth: 0,
            startmsgnum: 0,
            msgsperfolder: 0,
            save_alts: 0,
            delete_level: DELETE_LEAVES_TEXT,
            progress: PROGRESS,
            max_message_size: 100 * 1024 * 1024,
            show_headers: HmList::new(),
            avoid_indices: HmList::new(),
            avoid_top_indices: HmList::new(),
            skip_headers: HmList::new(),
            text_types: HmList::new(),
            inline_types: HmList::from_whitespace_str(INLINE_TYPES),
            deleted: HmList::from_whitespace_str("X-Hypermail-Deleted X-No-Archive"),
            expires: HmList::from_whitespace_str("Expires"),
            delete_msgnum: HmList::new(),
            filter_out: HmList::new(),
            filter_require: HmList::new(),
            filter_out_full_body: HmList::new(),
            filter_require_full_body: HmList::new(),
            prefered_types: HmList::new(),
            ignore_types: HmList::new(),
        }
    }
}

impl Config {
    /// Sets a string configuration value by key name.
    pub fn set_string(&mut self, key: &str, val: &str) -> Result<()> {
        match key {
            "fragment_prefix" => self.fragment_prefix = val.to_string(),
            "htmlmessage_deleted" => self.htmlmessage_deleted = Some(val.to_string()),
            "antispam_at" => self.antispam_at = val.to_string(),
            "antispamdomain" => {
                if val == "NONE" || val.is_empty() {
                    self.antispamdomain = None;
                } else {
                    self.antispamdomain = Some(val.to_string());
                }
            },
            "language" => self.language = val.to_string(),
            "htmlsuffix" => self.htmlsuffix = val.to_string(),
            "mbox" => {
                if val == "NONE" {
                    self.mbox = None;
                } else {
                    self.mbox = Some(val.to_string());
                }
            },
            "archives" => {
                if val == "NONE" {
                    self.archives = None;
                } else {
                    self.archives = Some(val.to_string());
                }
            },
            "custom_archives" => {
                if val == "NONE" {
                    self.custom_archives = None;
                } else {
                    self.custom_archives = Some(val.to_string());
                }
            },
            "about" => {
                if val == "NONE" {
                    self.about = None;
                } else {
                    self.about = Some(val.to_string());
                }
            },
            "label" => {
                if val == "NONE" {
                    self.label = None;
                } else {
                    self.label = Some(val.to_string());
                }
            },
            "dir" => {
                if val == "NONE" {
                    self.dir = None;
                } else {
                    self.dir = Some(val.to_string());
                }
            },
            "defaultindex" => self.defaultindex = val.to_string(),
            "default_top_index" => self.default_top_index = val.to_string(),
            "mailcommand" => self.mailcommand = val.to_string(),
            "newmsg_command" => self.newmsg_command = val.to_string(),
            "replymsg_command" => self.replymsg_command = val.to_string(),
            "inreplyto_command" => self.inreplyto_command = Some(val.to_string()),
            "mailto" => {
                if val == "NONE" {
                    self.mailto = None;
                } else {
                    self.mailto = Some(val.to_string());
                }
            },
            "hmail" => {
                if val == "NONE" {
                    self.hmail = None;
                } else {
                    self.hmail = Some(val.to_string());
                }
            },
            "domainaddr" => {
                if val == "NONE" {
                    self.domainaddr = None;
                } else {
                    self.domainaddr = Some(val.to_string());
                }
            },
            "css" => self.css = Some(val.to_string()),
            "icss_url" => self.icss_url = Some(val.to_string()),
            "mcss_url" => self.mcss_url = Some(val.to_string()),
            "dateformat" => self.dateformat = Some(val.to_string()),
            "indexdateformat" => self.indexdateformat = Some(val.to_string()),
            "stripsubject" => self.stripsubject = Some(val.to_string()),
            "link_to_replies" => self.link_to_replies = Some(val.to_string()),
            "quote_link_string" => self.quote_link_string = Some(val.to_string()),
            "ihtmlheaderfile" => self.ihtmlheader = Some(val.to_string()),
            "ihtmlfooterfile" => self.ihtmlfooter = Some(val.to_string()),
            "ihtmlheadfile" => self.ihtmlhead = Some(val.to_string()),
            "ihtmlhelpupfile" => self.ihtmlhelpup = Some(val.to_string()),
            "ihtmlhelplowfile" => self.ihtmlhelplow = Some(val.to_string()),
            "ihtmlnavbar2upfile" => self.ihtmlnavbar2up = Some(val.to_string()),
            "mhtmlheaderfile" => self.mhtmlheader = Some(val.to_string()),
            "mhtmlfooterfile" => self.mhtmlfooter = Some(val.to_string()),
            // Aliases that apply to both index and message pages simultaneously.
            "htmlheaderfile" => {
                self.ihtmlheader = Some(val.to_string());
                self.mhtmlheader = Some(val.to_string());
            },
            "htmlfooterfile" => {
                self.ihtmlfooter = Some(val.to_string());
                self.mhtmlfooter = Some(val.to_string());
            },
            "attachmentlink" => self.attachmentlink = Some(val.to_string()),
            "unsafe_chars" => self.unsafe_chars = Some(val.to_string()),
            "description" => self.description = Some(val.to_string()),
            "theme" => self.theme = Some(val.to_string()),
            "bodyheader" => self.bodyheader = Some(val.to_string()),
            "bodyheaderend" => self.bodyheaderend = Some(val.to_string()),
            "bodyfooter" => self.bodyfooter = Some(val.to_string()),
            "filename_base" => self.filename_base = Some(val.to_string()),
            "folder_by_date" => {
                if val.is_empty() || val == "NONE" {
                    self.folder_by_date = None;
                } else {
                    self.folder_by_date = Some(val.to_string());
                }
            },
            "latest_folder" => self.latest_folder = Some(val.to_string()),
            "base_url" => self.base_url = Some(val.to_string()),
            "describe_folder" => self.describe_folder = Some(val.to_string()),
            "delete_older" => self.delete_older = Some(val.to_string()),
            "delete_newer" => self.delete_newer = Some(val.to_string()),
            "alts_text" => self.alts_text = Some(val.to_string()),
            "append_filename" => self.append_filename = Some(val.to_string()),
            "txtsuffix" => self.txtsuffix = Some(val.to_string()),
            _ => {
                return Err(HypermailError::InvalidConfigValue {
                    key: key.to_string(),
                    message: format!("unknown string config key: {}", key),
                })
            },
        }
        Ok(())
    }

    /// Sets a boolean switch configuration value by key name.
    pub fn set_switch(&mut self, key: &str, val: bool) -> Result<()> {
        match key {
            "email_address_obfuscation" => self.email_address_obfuscation = val,
            "i18n" => self.i18n = val,
            "i18n_body" => self.i18n_body = val,
            "overwrite" => self.overwrite = val,
            "inlinehtml" => self.inlinehtml = val,
            "readone" => self.readone = val,
            "reverse" => self.reverse = val,
            "reverse_folders" => self.reverse_folders = val,
            "showheaders" => self.showheaders = val,
            "showbr" => self.showbr = val,
            "showreplies" => self.showreplies = val,
            "indextable" => self.indextable = val,
            "iquotes" => self.iquotes = val,
            "eurodate" => self.eurodate = val,
            "gmtime" => self.gmtime = val,
            "isodate" => self.isodate = val,
            "require_msgids" => self.require_msgids = val,
            "discard_dup_msgids" => self.discard_dup_msgids = val,
            "usemeta" => self.usemeta = val,
            "uselock" => self.uselock = val,
            "ietf_mbox" => self.ietf_mbox = val,
            "linkquotes" => self.linkquotes = val,
            "monthly_index" => self.monthly_index = val,
            "yearly_index" => self.yearly_index = val,
            "spamprotect" => self.spamprotect = val,
            "spamprotect_id" => self.spamprotect_id = val,
            "attachmentsindex" => self.attachmentsindex = val,
            "usegdbm" => self.usegdbm = val,
            "writehaof" => self.writehaof = val,
            "append" => self.append = val,
            "nonsequential" => self.nonsequential = val,
            "warn_surpressions" => self.warn_surpressions = val,
            "files_by_thread" => self.files_by_thread = val,
            "href_detection" => self.href_detection = val,
            "mbox_shortened" => self.mbox_shortened = val,
            "report_new_file" => self.report_new_file = val,
            "report_new_folder" => self.report_new_folder = val,
            "use_sender_date" => self.use_sender_date = val,
            "inline_addlink" => self.inline_addlink = val,
            "iso2022jp" => self.iso2022jp = val,
            "delete_incremental" => self.delete_incremental = val,
            "showgenerator" => self.showgenerator = val,
            "show_warnings" => self.show_warnings = val,
            _ => {
                return Err(HypermailError::InvalidConfigValue {
                    key: key.to_string(),
                    message: format!("unknown switch config key: {}", key),
                })
            },
        }
        Ok(())
    }

    /// Sets an integer configuration value by key name.
    pub fn set_integer(&mut self, key: &str, val: i64) -> Result<()> {
        match key {
            "increment" => self.increment = val as i32,
            "showhtml" => self.showhtml = val as i32,
            "show_msg_links" => self.show_msg_links = val as i32,
            "show_index_links" => self.show_index_links = val as i32,
            "thrdlevels" => self.thrdlevels = val as i32,
            "dirmode" => self.dirmode = val as i32,
            "filemode" => self.filemode = val as i32,
            "locktime" => self.locktime = val as i32,
            "searchbackmsgnum" => self.searchbackmsgnum = val as i32,
            "quote_hide_threshold" => self.quote_hide_threshold = val as i32,
            "thread_file_depth" => self.thread_file_depth = val as i32,
            "startmsgnum" => self.startmsgnum = val as i32,
            "msgsperfolder" => self.msgsperfolder = val as i32,
            "save_alts" => self.save_alts = val as i32,
            "delete_level" => self.delete_level = val as i32,
            "progress" => self.progress = val as i32,
            "max_message_size" => self.max_message_size = val as usize,
            _ => {
                return Err(HypermailError::InvalidConfigValue {
                    key: key.to_string(),
                    message: format!("unknown integer config key: {}", key),
                })
            },
        }
        Ok(())
    }

    /// Appends whitespace-separated values to a list configuration by key name.
    pub fn set_list(&mut self, key: &str, val: &str) -> Result<()> {
        let list = match key {
            "show_headers" => &mut self.show_headers,
            "avoid_indices" => &mut self.avoid_indices,
            "avoid_top_indices" => &mut self.avoid_top_indices,
            "text_types" => &mut self.text_types,
            "inline_types" => &mut self.inline_types,
            "prefered_types" => &mut self.prefered_types,
            "ignore_types" => &mut self.ignore_types,
            "filter_out" => &mut self.filter_out,
            "filter_require" => &mut self.filter_require,
            "filter_out_full_body" => &mut self.filter_out_full_body,
            "filter_require_full_body" => &mut self.filter_require_full_body,
            "deleted" => &mut self.deleted,
            "expires" => &mut self.expires,
            "delete_msgnum" => &mut self.delete_msgnum,
            _ => {
                return Err(HypermailError::InvalidConfigValue {
                    key: key.to_string(),
                    message: format!("unknown list config key: {}", key),
                })
            },
        };
        list.add_list(val);
        Ok(())
    }

    /// Applies a CLI argument, auto-detecting the value type (bool/int/string/list).
    ///
    /// Supports `key=value` syntax, `hm_` and `set_` prefixes, and ON/OFF/YES/NO values.
    pub fn apply_cli_arg(&mut self, key: &str, val: &str) -> Result<()> {
        let (actual_key, actual_val) = if let Some(eq_pos) = key.find('=') {
            let k = &key[..eq_pos];
            let v = &key[eq_pos + 1..];
            (k, v)
        } else {
            (key, val)
        };

        let actual_key = actual_key.strip_prefix("hm_").unwrap_or(actual_key);
        let actual_key = actual_key.strip_prefix("set_").unwrap_or(actual_key);

        // Deprecated keys from C hypermail that are no longer functional.
        // Return a recognizable error so callers can emit a deprecation warning.
        if matches!(actual_key, "showhr" | "usetable" | "body") {
            return Err(HypermailError::InvalidConfigValue {
                key: actual_key.to_string(),
                message: "deprecated config key — has no effect; remove from your config file"
                    .to_string(),
            });
        }
        let actual_val = actual_val.trim();

        if actual_val == "ON"
            || actual_val == "YES"
            || actual_val == "On"
            || actual_val == "Yes"
            || actual_val == "on"
            || actual_val == "yes"
        {
            if let Ok(()) = self.set_switch(actual_key, true) {
                return Ok(());
            }
            return self
                .set_integer(actual_key, 1)
                .or_else(|_| self.set_string(actual_key, actual_val));
        }
        if (actual_val == "OFF"
            || actual_val == "NO"
            || actual_val == "Off"
            || actual_val == "No"
            || actual_val == "off"
            || actual_val == "no")
            && self.set_switch(actual_key, false).is_ok()
        {
            return Ok(());
        }

        // Octal parsing for permission modes (dirmode, filemode)
        // Always parse as octal — "755" means 0o755, with or without leading zero
        if actual_key == "dirmode" || actual_key == "filemode" {
            let octal_str = actual_val.strip_prefix('0').unwrap_or(actual_val);
            if let Ok(i) = i64::from_str_radix(octal_str, 8) {
                if self.set_integer(actual_key, i).is_ok() {
                    return Ok(());
                }
            }
        }

        let int_val = actual_val.parse::<i64>();
        if let Ok(i) = int_val {
            if self.set_integer(actual_key, i).is_ok() {
                return Ok(());
            }
        }
        if self.set_string(actual_key, actual_val).is_ok() {
            return Ok(());
        }
        if self.set_list(actual_key, actual_val).is_ok() {
            return Ok(());
        }

        Err(HypermailError::InvalidConfigValue {
            key: actual_key.to_string(),
            message: format!("unrecognized config key or invalid value: {}", actual_val),
        })
    }

    /// Loads configuration from environment variables prefixed with `HM_`.
    pub fn load_env(&mut self) {
        for (key, val) in std::env::vars() {
            if let Some(stripped) = key.strip_prefix("HM_") {
                let config_key = stripped.to_lowercase();
                let _ = self.apply_cli_arg(&config_key, &val);
            }
        }
    }

    /// Applies post-processing defaults (e.g., always-skipped headers).
    pub fn post_process(&mut self) {
        self.skip_headers.add("from");
        self.skip_headers.add("date");
        self.skip_headers.add("subject");
    }

    /// Returns the resolved CSS path, joining with `dir` if relative.
    pub fn css_path(&self) -> String {
        if let Some(ref css) = self.css {
            if css.starts_with("http") || Path::new(css).is_absolute() {
                css.clone()
            } else if let Some(ref dir) = self.dir {
                PathBuf::from(dir).join(css).to_string_lossy().into_owned()
            } else {
                css.clone()
            }
        } else {
            String::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = Config::default();
        assert_eq!(cfg.language, "en");
        assert_eq!(cfg.htmlsuffix, "html");
        assert_eq!(cfg.defaultindex, "date");
        assert!(cfg.inlinehtml);
        assert!(!cfg.overwrite);
        assert_eq!(cfg.showhtml, 1);
        assert_eq!(cfg.thrdlevels, 50); // Changed from 4 to 50 for deeper thread display
        assert_eq!(cfg.dirmode, 0o755);
        assert_eq!(cfg.filemode, 0o644);
        assert_eq!(cfg.locktime, 3600);
        assert_eq!(cfg.searchbackmsgnum, 500);
        assert_eq!(cfg.quote_hide_threshold, 100);
        assert_eq!(cfg.delete_level, DELETE_LEAVES_TEXT);
        assert!(cfg.discard_dup_msgids);
        assert!(cfg.require_msgids);
        assert!(cfg.uselock);
        assert!(cfg.href_detection);
        assert!(cfg.warn_surpressions);
        assert!(cfg.attachmentsindex);
        assert!(cfg.spamprotect);
        assert!(cfg.spamprotect_id);
        assert!(cfg.showbr);
        assert!(cfg.showreplies);
        assert!(cfg.inline_addlink);
        assert!(cfg.delete_incremental);
        assert_eq!(cfg.fragment_prefix, "msg");
        assert_eq!(cfg.antispam_at, "@");
        assert_eq!(cfg.progress, 0);
        assert!(cfg.inline_types.contains("image/gif"));
        assert!(cfg.deleted.contains("X-Hypermail-Deleted"));
        assert!(cfg.expires.contains("Expires"));
    }

    #[test]
    fn test_set_string() {
        let mut cfg = Config::default();
        cfg.set_string("language", "de").unwrap();
        assert_eq!(cfg.language, "de");
        cfg.set_string("mbox", "NONE").unwrap();
        assert!(cfg.mbox.is_none());
        cfg.set_string("label", "test list").unwrap();
        assert_eq!(cfg.label.as_deref(), Some("test list"));
    }

    #[test]
    fn test_set_switch() {
        let mut cfg = Config::default();
        assert!(!cfg.overwrite);
        cfg.set_switch("overwrite", true).unwrap();
        assert!(cfg.overwrite);
    }

    #[test]
    fn test_set_integer() {
        let mut cfg = Config::default();
        cfg.set_integer("showhtml", 2).unwrap();
        assert_eq!(cfg.showhtml, 2);
        cfg.set_integer("thrdlevels", 8).unwrap();
        assert_eq!(cfg.thrdlevels, 8);
    }

    #[test]
    fn test_set_list() {
        let mut cfg = Config::default();
        cfg.set_list("text_types", "text/html text/plain").unwrap();
        assert!(cfg.text_types.contains("text/html"));
        assert!(cfg.text_types.contains("text/plain"));
    }

    #[test]
    fn test_apply_cli_arg() {
        let mut cfg = Config::default();
        cfg.apply_cli_arg("overwrite", "On").unwrap();
        assert!(cfg.overwrite);
        cfg.apply_cli_arg("showhtml", "2").unwrap();
        assert_eq!(cfg.showhtml, 2);
        cfg.apply_cli_arg("language=de", "").unwrap();
        assert_eq!(cfg.language, "de");
    }

    #[test]
    fn test_post_process() {
        let mut cfg = Config::default();
        cfg.post_process();
        assert!(cfg.skip_headers.contains("from"));
        assert!(cfg.skip_headers.contains("date"));
        assert!(cfg.skip_headers.contains("subject"));
    }

    #[test]
    fn test_unknown_key() {
        let mut cfg = Config::default();
        assert!(cfg.set_string("nonexistent", "value").is_err());
        assert!(cfg.set_switch("nonexistent", true).is_err());
        assert!(cfg.set_integer("nonexistent", 42).is_err());
    }

    #[test]
    fn test_none_strings() {
        let mut cfg = Config::default();
        cfg.set_string("archives", "NONE").unwrap();
        assert!(cfg.archives.is_none());
        cfg.set_string("about", "NONE").unwrap();
        assert!(cfg.about.is_none());
        cfg.set_string("custom_archives", "NONE").unwrap();
        assert!(cfg.custom_archives.is_none());
    }

    #[test]
    fn test_apply_cli_hm_prefix() {
        let mut cfg = Config::default();
        cfg.apply_cli_arg("hm_overwrite", "On").unwrap();
        assert!(cfg.overwrite);
    }

    #[test]
    fn test_apply_cli_set_prefix() {
        let mut cfg = Config::default();
        cfg.apply_cli_arg("set_overwrite", "On").unwrap();
        assert!(cfg.overwrite);
    }

    #[test]
    fn test_apply_cli_bool_yes() {
        let mut cfg = Config::default();
        cfg.apply_cli_arg("overwrite", "YES").unwrap();
        assert!(cfg.overwrite);
    }

    #[test]
    fn test_apply_cli_bool_no() {
        let mut cfg = Config::default();
        assert!(cfg.inlinehtml);
        cfg.apply_cli_arg("inlinehtml", "OFF").unwrap();
        assert!(!cfg.inlinehtml);
    }

    #[test]
    fn test_apply_cli_octal_dirmode() {
        let mut cfg = Config::default();
        cfg.apply_cli_arg("dirmode", "0755").unwrap();
        assert_eq!(cfg.dirmode, 0o755);
    }

    #[test]
    fn test_apply_cli_octal_filemode() {
        let mut cfg = Config::default();
        cfg.apply_cli_arg("filemode", "0644").unwrap();
        assert_eq!(cfg.filemode, 0o644);
    }

    #[test]
    fn test_apply_cli_decimal_dirmode() {
        let mut cfg = Config::default();
        // "755" without leading zero is still parsed as octal for dirmode/filemode
        cfg.apply_cli_arg("dirmode", "755").unwrap();
        assert_eq!(cfg.dirmode, 0o755);
    }

    #[test]
    fn test_apply_cli_inline_eq() {
        let mut cfg = Config::default();
        cfg.apply_cli_arg("language=fr", "").unwrap();
        assert_eq!(cfg.language, "fr");
    }

    #[test]
    fn test_apply_cli_list() {
        let mut cfg = Config::default();
        cfg.apply_cli_arg("filter_out", "spam").unwrap();
        assert!(cfg.filter_out.contains("spam"));
    }

    #[test]
    fn test_apply_cli_with_quoted_value() {
        let mut cfg = Config::default();
        cfg.apply_cli_arg("label", "\"My Archive\"").unwrap();
        assert_eq!(cfg.label.as_deref(), Some("\"My Archive\""));
    }

    #[test]
    fn test_apply_cli_unknown_key() {
        let mut cfg = Config::default();
        assert!(cfg.apply_cli_arg("nonexistent", "value").is_err());
    }

    #[test]
    fn test_htmlheaderfile_sets_both_i_and_m() {
        let mut cfg = Config::default();
        cfg.apply_cli_arg("htmlheaderfile", "/path/to/header.html").unwrap();
        assert_eq!(cfg.ihtmlheader.as_deref(), Some("/path/to/header.html"));
        assert_eq!(cfg.mhtmlheader.as_deref(), Some("/path/to/header.html"));
    }

    #[test]
    fn test_htmlfooterfile_sets_both_i_and_m() {
        let mut cfg = Config::default();
        cfg.apply_cli_arg("htmlfooterfile", "/path/to/footer.html").unwrap();
        assert_eq!(cfg.ihtmlfooter.as_deref(), Some("/path/to/footer.html"));
        assert_eq!(cfg.mhtmlfooter.as_deref(), Some("/path/to/footer.html"));
    }

    #[test]
    fn test_htmlheaderfile_does_not_override_specific_variants() {
        let mut cfg = Config::default();
        cfg.apply_cli_arg("ihtmlheaderfile", "/index/header.html").unwrap();
        cfg.apply_cli_arg("htmlheaderfile", "/shared/header.html").unwrap();
        // htmlheaderfile overwrites both — if you want per-page control, use the specific keys
        assert_eq!(cfg.ihtmlheader.as_deref(), Some("/shared/header.html"));
        assert_eq!(cfg.mhtmlheader.as_deref(), Some("/shared/header.html"));
    }

    #[test]
    fn test_deprecated_showhr_returns_deprecation_error() {
        let mut cfg = Config::default();
        let err = cfg.apply_cli_arg("showhr", "1").unwrap_err();
        assert!(
            err.to_string().contains("deprecated config key"),
            "expected deprecation message, got: {err}"
        );
    }

    #[test]
    fn test_deprecated_usetable_returns_deprecation_error() {
        let mut cfg = Config::default();
        let err = cfg.apply_cli_arg("usetable", "1").unwrap_err();
        assert!(
            err.to_string().contains("deprecated config key"),
            "expected deprecation message, got: {err}"
        );
    }

    #[test]
    fn test_deprecated_body_returns_deprecation_error() {
        let mut cfg = Config::default();
        let err = cfg.apply_cli_arg("body", "1").unwrap_err();
        assert!(
            err.to_string().contains("deprecated config key"),
            "expected deprecation message, got: {err}"
        );
    }

    #[test]
    fn test_env_loading() {
        let mut cfg = Config::default();
        std::env::set_var("HM_LANGUAGE", "de");
        cfg.load_env();
        std::env::remove_var("HM_LANGUAGE");
        assert_eq!(cfg.language, "de");
    }

    #[test]
    fn test_set_all_switches() {
        let mut cfg = Config::default();
        for (key, initial) in &[
            ("email_address_obfuscation", false),
            ("i18n", true),
            ("i18n_body", false),
            ("overwrite", false),
            ("inlinehtml", true),
            ("readone", false),
            ("reverse", false),
            ("reverse_folders", false),
            ("showheaders", true),
            ("showbr", true),
            ("showreplies", true),
            ("indextable", false),
            ("iquotes", true),
            ("eurodate", true),
            ("gmtime", false),
            ("isodate", false),
            ("require_msgids", true),
            ("discard_dup_msgids", true),
            ("usemeta", false),
            ("uselock", true),
            ("ietf_mbox", false),
            ("linkquotes", false),
            ("monthly_index", false),
            ("yearly_index", false),
            ("spamprotect", true),
            ("spamprotect_id", true),
            ("attachmentsindex", true),
            ("usegdbm", false),
            ("writehaof", false),
            ("append", false),
            ("nonsequential", false),
            ("warn_surpressions", true),
            ("files_by_thread", false),
            ("href_detection", true),
            ("mbox_shortened", false),
            ("report_new_file", false),
            ("report_new_folder", false),
            ("use_sender_date", false),
            ("inline_addlink", true),
            ("iso2022jp", false),
            ("delete_incremental", true),
            ("showgenerator", true),
            ("show_warnings", false),
        ] {
            assert!(cfg.set_switch(key, *initial).is_ok(), "switch {} should exist", key);
        }
    }

    #[test]
    fn test_set_all_integers() {
        let mut cfg = Config::default();
        for key in &[
            "increment",
            "showhtml",
            "show_msg_links",
            "show_index_links",
            "thrdlevels",
            "dirmode",
            "filemode",
            "locktime",
            "searchbackmsgnum",
            "quote_hide_threshold",
            "thread_file_depth",
            "startmsgnum",
            "msgsperfolder",
            "save_alts",
            "delete_level",
            "progress",
            "max_message_size",
        ] {
            assert!(cfg.set_integer(key, 1).is_ok(), "integer {} should exist", key);
        }
    }

    #[test]
    fn test_set_all_strings() {
        let mut cfg = Config::default();
        for key in &[
            "fragment_prefix",
            "htmlmessage_deleted",
            "antispam_at",
            "antispamdomain",
            "language",
            "htmlsuffix",
            "mbox",
            "archives",
            "custom_archives",
            "about",
            "label",
            "dir",
            "defaultindex",
            "default_top_index",
            "mailcommand",
            "newmsg_command",
            "replymsg_command",
            "inreplyto_command",
            "mailto",
            "hmail",
            "domainaddr",
            "css",
            "icss_url",
            "mcss_url",
            "dateformat",
            "indexdateformat",
            "stripsubject",
            "link_to_replies",
            "quote_link_string",
            "ihtmlheaderfile",
            "ihtmlfooterfile",
            "ihtmlheadfile",
            "ihtmlhelpupfile",
            "ihtmlhelplowfile",
            "ihtmlnavbar2upfile",
            "mhtmlheaderfile",
            "mhtmlfooterfile",
            "htmlheaderfile",
            "htmlfooterfile",
            "attachmentlink",
            "bodyheader",
            "bodyheaderend",
            "bodyfooter",
            "unsafe_chars",
            "filename_base",
            "folder_by_date",
            "latest_folder",
            "base_url",
            "describe_folder",
            "delete_older",
            "delete_newer",
            "alts_text",
            "description",
            "theme",
            "append_filename",
            "txtsuffix",
        ] {
            assert!(cfg.set_string(key, "test").is_ok(), "string {} should exist", key);
        }
    }

    #[test]
    fn test_set_all_lists() {
        let mut cfg = Config::default();
        for key in &[
            "show_headers",
            "avoid_indices",
            "avoid_top_indices",
            "text_types",
            "inline_types",
            "prefered_types",
            "ignore_types",
            "filter_out",
            "filter_require",
            "filter_out_full_body",
            "filter_require_full_body",
            "deleted",
            "expires",
            "delete_msgnum",
        ] {
            assert!(cfg.set_list(key, "test").is_ok(), "list {} should exist", key);
        }
    }

    #[test]
    fn test_config_file_content_can_parse() {
        let config_content = "\
# comment
set language=de
hm_overwrite=On
nonsequential On
dirmode 0755
mbox mailbox/test
label \"My Archive\"
";
        // Verify each line can be parsed via apply_cli_arg
        let mut cfg = Config::default();
        for line in config_content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let line = line.strip_prefix("set ").unwrap_or(line);
            let eq_pos = line.find('=').or_else(|| line.find(':'));
            if let Some(eq_pos) = eq_pos {
                let key = line[..eq_pos].trim();
                let val = line[eq_pos + 1..].trim();
                let val = val.trim_matches('"');
                cfg.apply_cli_arg(key, val).unwrap_or_else(|e| {
                    panic!("Failed to parse line '{}': {e}", line);
                });
            }
        }
        assert_eq!(cfg.language, "de");
        assert!(cfg.overwrite);
    }

    #[test]
    fn test_antispamdomain_roundtrip() {
        let mut cfg = Config::default();
        cfg.set_string("antispamdomain", "nospam.invalid").unwrap();
        assert_eq!(cfg.antispamdomain.as_deref(), Some("nospam.invalid"));
    }

    #[test]
    fn test_antispamdomain_none_on_empty() {
        let mut cfg = Config::default();
        cfg.set_string("antispamdomain", "").unwrap();
        assert!(cfg.antispamdomain.is_none());
    }

    #[test]
    fn test_antispamdomain_none_on_keyword() {
        let mut cfg = Config::default();
        cfg.set_string("antispamdomain", "NONE").unwrap();
        assert!(cfg.antispamdomain.is_none());
    }
}
