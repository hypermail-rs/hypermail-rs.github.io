use crate::config::Config;
use crate::message::EmailInfo;
use chrono::{Local, TimeZone, Utc};
use std::fs;
use std::path::{Path, PathBuf};

/// Apply configured permissions to a path (Unix only).
#[cfg(unix)]
pub fn apply_permissions(path: &Path, mode: i32) {
    use std::os::unix::fs::PermissionsExt;
    let perms = std::fs::Permissions::from_mode(mode as u32);
    if let Err(e) = std::fs::set_permissions(path, perms) {
        log::debug!("Failed to set permissions on {:?}: {}", path, e);
    }
}

/// Apply configured permissions to a path (no-op on non-Unix platforms).
#[cfg(not(unix))]
pub fn apply_permissions(_path: &Path, _mode: i32) {
    // Permissions are Unix-only; no-op on other platforms
}

/// Generates a unique name for an email message.
///
/// Uses either sequential numbering (default) or content-based hashing
/// when `config.nonsequential` is enabled.
///
/// # Arguments
///
/// * `email` - The email to generate a name for
/// * `config` - Configuration determining naming scheme
///
/// # Returns
///
/// - Sequential: `"0001"`, `"0042"`, etc. (4 digits, zero-padded)
/// - Hashed: `"a3f2c891be4f3210"` (16 hex digits from FNV32 hash)
///
/// # Security
///
/// The hashed mode uses FNV-1a to prevent predictable filenames, which
/// could be used to guess Message-IDs. The hash includes both message ID
/// and timestamp for uniqueness.
pub fn message_name(email: &EmailInfo, config: &Config) -> String {
    if config.nonsequential {
        if let Some(ref msgid) = email.msgid {
            let hash = fnv32(msgid.as_bytes(), email.from_date);
            return format!("{:08x}{:08x}", hash, email.from_date as u32);
        }
    }
    format!("{:04}", email.msgnum)
}

/// FNV-1a 32-bit hash function.
///
/// Computes a hash using the FNV-1a algorithm with an optional seed.
/// This is used for generating content-based filenames that don't reveal
/// the original Message-ID.
///
/// # Security Note
///
/// Uses `wrapping_mul` intentionally as required by the FNV algorithm.
/// Integer overflow is part of the hash specification and produces
/// correct hash distribution.
///
/// # Arguments
///
/// * `buf` - Input bytes to hash
/// * `seed` - Optional seed value (typically timestamp)
///
/// # Returns
///
/// 32-bit hash value as u32
fn fnv32(buf: &[u8], seed: i64) -> u32 {
    const FNV1_32_INIT: u32 = 0x811c9dc5;
    const FNV_32_PRIME: u32 = 0x01000193;
    let mut hash = FNV1_32_INIT;
    for &b in buf {
        hash ^= b as u32;
        hash = hash.wrapping_mul(FNV_32_PRIME);
    }
    if seed != 0 {
        for b in seed.to_le_bytes() {
            hash ^= b as u32;
            hash = hash.wrapping_mul(FNV_32_PRIME);
        }
    }
    hash
}

/// Returns the full filename for a message (name + HTML suffix).
pub fn message_filename(email: &EmailInfo, config: &Config) -> String {
    format!("{}.{}", message_name(email, config), config.htmlsuffix)
}

/// Returns the full filesystem path for a message's HTML file.
pub fn message_path(email: &EmailInfo, config: &Config) -> PathBuf {
    let dir = config.dir.as_deref().unwrap_or(".");
    let sub = msg_subdir(email, config);
    let base = match sub {
        Some(ref s) => PathBuf::from(dir).join(&s.subdir),
        None => PathBuf::from(dir),
    };
    base.join(message_filename(email, config))
}

/// Returns the relative URL string for a message, including subdirectory if applicable.
pub fn message_url_str(email: &EmailInfo, config: &Config) -> String {
    let sub = msg_subdir(email, config);
    let filename = message_filename(email, config);
    match sub {
        Some(ref s) => {
            let subdir = s.subdir.trim_end_matches('/');
            if subdir.is_empty() {
                filename
            } else {
                format!("{}/{}", subdir, filename)
            }
        },
        None => filename,
    }
}

/// Returns the path to the message index file.
pub fn messageindex_name(config: &Config) -> PathBuf {
    PathBuf::from(config.dir.as_deref().unwrap_or(".")).join("msgindex")
}

/// Expands date format placeholders (`%y`, `%m`, `%d`, etc.) in a path template.
pub fn dirpath(frmptr: &str) -> String {
    let now = chrono::Local::now();
    let mut result = String::new();
    let mut chars = frmptr.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            match chars.next() {
                Some('d') => result.push_str(&now.format("%d").to_string()),
                Some('D') => result.push_str(&now.format("%a").to_string()),
                Some('j') => result.push_str(&now.format("%j").to_string()),
                Some('m') => result.push_str(&now.format("%m").to_string()),
                Some('M') => result.push_str(&now.format("%b").to_string()),
                Some('y') => result.push_str(&now.format("%Y").to_string()),
                Some('%') => result.push('%'),
                Some(c) => {
                    result.push('%');
                    result.push(c);
                },
                None => result.push('%'),
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Information about the subdirectory for a message when using folder-based layouts.
pub struct EmailSubdirInfo {
    pub subdir: String,
    pub full_path: String,
    pub rel_path_to_top: String,
    pub description: Option<String>,
}

/// Determines the subdirectory for a message based on folder configuration.
pub fn msg_subdir(email: &EmailInfo, config: &Config) -> Option<EmailSubdirInfo> {
    if config.msgsperfolder > 0 {
        let subdir_no = email.msgnum / config.msgsperfolder;
        let sub = format!("{}/", subdir_no);
        let base = config.dir.as_deref().unwrap_or(".");
        let full = PathBuf::from(base).join(&sub).to_string_lossy().to_string();
        let rel = if subdir_no == 0 {
            "./".to_string()
        } else {
            let mut r = String::new();
            let depth = sub.matches('/').count();
            for _ in 0..depth {
                r.push_str("../");
            }
            r
        };
        return Some(EmailSubdirInfo {
            subdir: sub,
            full_path: full,
            rel_path_to_top: rel,
            description: None,
        });
    }
    if let Some(ref fbd) = config.folder_by_date {
        if email.date > 0 {
            let ts = Utc.timestamp_opt(email.date, 0).single().unwrap_or_default();
            let sub = if config.gmtime {
                ts.format(fbd).to_string()
            } else {
                ts.with_timezone(&Local).format(fbd).to_string()
            };
            // Security: reject path traversal attempts via format string
            if sub.contains("..") || std::path::Path::new(&sub).is_absolute() {
                log::warn!("folder_by_date produced suspicious path '{}', using flat layout", sub);
                return None;
            }
            let sub = if !sub.ends_with('/') {
                format!("{}/", sub)
            } else {
                sub
            };
            let base = config.dir.as_deref().unwrap_or(".");
            let full = PathBuf::from(base).join(&sub).to_string_lossy().to_string();
            let depth = sub.matches('/').count();
            let rel = if depth == 0 {
                "./".to_string()
            } else {
                let mut r = String::new();
                for _ in 0..depth {
                    r.push_str("../");
                }
                r
            };
            return Some(EmailSubdirInfo {
                subdir: sub,
                full_path: full,
                rel_path_to_top: rel,
                description: None,
            });
        }
    }
    None
}

/// Creates a symlink pointing to the latest folder (if configured).
///
/// Target is the subdirectory of the newest message by date (folder_by_date or
/// msgsperfolder layout). Relative link target matches classic Hypermail.
pub fn symlink_latest(store: &crate::structs::EmailStore, config: &Config) -> std::io::Result<()> {
    if let Some(ref latest) = config.latest_folder {
        // Security: reject path traversal / absolute paths in the (operator-configured)
        // latest_folder value, same guard as folder_by_date above — a misconfigured or
        // malicious config could otherwise place a symlink outside the archive dir.
        if latest.contains("..") || std::path::Path::new(latest).is_absolute() {
            log::warn!(
                "latest_folder '{}' looks unsafe (absolute or contains ..); skipping symlink",
                latest
            );
            return Ok(());
        }
        let dir = config.dir.as_deref().unwrap_or(".");
        let link_path = PathBuf::from(dir).join(latest);
        let target = latest_folder_target(store, config);
        let _ = fs::remove_file(&link_path);
        // Also remove if it is a directory symlink/junction
        let _ = fs::remove_dir(&link_path);
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &link_path)?;
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&target, &link_path)?;
    }
    Ok(())
}

/// Relative path of the folder containing the newest message, or `"."` if flat layout.
fn latest_folder_target(store: &crate::structs::EmailStore, config: &Config) -> String {
    let mut best: Option<(i64, String)> = None;
    for email in &store.emails {
        if let Some(sub) = msg_subdir(email, config) {
            let subdir = sub.subdir.trim_end_matches('/').to_string();
            if subdir.is_empty() {
                continue;
            }
            match best {
                Some((d, _)) if email.date < d => {},
                _ => best = Some((email.date, subdir)),
            }
        }
    }
    best.map(|(_, s)| s).unwrap_or_else(|| ".".to_string())
}

/// Creates a directory and all parent directories if they don't exist.
pub fn checkdir(path: &str) -> std::io::Result<()> {
    let p = Path::new(path);
    if !p.exists() {
        fs::create_dir_all(p)?;
    }
    Ok(())
}

/// Returns true if the archive directory contains no non-hidden files.
pub fn is_empty_archive(config: &Config) -> bool {
    let dir = config.dir.as_deref().unwrap_or(".");
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if !name.starts_with('.') {
                    return false;
                }
            }
        }
    }
    true
}

/// Writes the message index file mapping message numbers to filenames.
pub fn write_messageindex(
    store: &crate::structs::EmailStore,
    config: &Config,
) -> std::io::Result<()> {
    let path = messageindex_name(config);
    let mut content = String::new();
    content.push_str(&format!("{:04} {:04}\n", 0, store.max_msgnum.max(0)));
    for email in &store.emails {
        let name = message_name(email, config);
        content.push_str(&format!("{:04} {}\n", email.msgnum, name));
    }
    fs::write(&path, &content)
}

/// Reads the message index file, returning a table of message number to filename mappings.
pub fn read_messageindex(config: &Config) -> std::io::Result<Vec<Option<String>>> {
    let path = messageindex_name(config);
    let content = fs::read_to_string(&path)?;
    let mut lines = content.lines();
    let mut table: Vec<Option<String>> = Vec::new();
    if let Some(first) = lines.next() {
        let parts: Vec<&str> = first.split_whitespace().collect();
        let max_num: i32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        // Reject negative or implausibly large values to prevent DoS via a corrupted
        // messageindex file: `(max_num + 1) as usize` would otherwise overflow or
        // attempt to allocate gigabytes of `Vec` slots.
        const MAX_MESSAGES: i32 = 100_000_000;
        if !(0..=MAX_MESSAGES).contains(&max_num) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid message count in messageindex header: {}", max_num),
            ));
        }
        let size = (max_num as usize).saturating_add(1);
        table.resize(size, None);
        for line in lines {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(num) = parts[0].parse::<i32>() {
                    if (num as usize) < table.len() {
                        table[num as usize] = Some(parts[1].to_string());
                    }
                }
            }
        }
    }
    Ok(table)
}

/// Loads email metadata from existing HTML archive files for incremental updates.
pub fn load_old_headers_from_html(store: &mut crate::structs::EmailStore, config: &Config) -> i32 {
    let dir = config.dir.as_deref().unwrap_or(".");
    let suffix = &config.htmlsuffix;
    let mut count = 0;

    let mut files = Vec::new();
    collect_html_files(Path::new(dir), suffix, &mut files);
    // Also scan subdirectories if using folders
    if config.msgsperfolder > 0 || config.folder_by_date.is_some() {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    collect_html_files(&path, suffix, &mut files);
                }
            }
        }
    }

    for path in files {
        let msgnum = if config.nonsequential {
            if let Ok(content) = fs::read_to_string(&path) {
                extract_msgnum_from_html(&content, &config.fragment_prefix).unwrap_or(0)
            } else {
                0
            }
        } else {
            path.file_stem()
                .and_then(|s| s.to_str())
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(0)
        };
        if msgnum == 0 {
            continue;
        }
        if let Ok(content) = fs::read_to_string(&path) {
            if let Some(email) = parse_old_html_comments(&content, msgnum) {
                let idx = store.add_email(email);
                store.insert_into_date_list(idx);
                store.insert_into_subject_list(idx);
                store.insert_into_author_list(idx);
                count += 1;
            }
        }
    }
    count
}

fn collect_html_files(dir: &Path, suffix: &str, files: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext.to_string_lossy() == suffix {
                        files.push(path);
                    }
                }
            }
        }
    }
}

fn extract_msgnum_from_html(html: &str, fragment_prefix: &str) -> Option<i32> {
    // New format: <article id="PREFIXmsgnum">
    let id_needle_str = format!("<article id=\"{}\"", fragment_prefix);
    let id_needle = id_needle_str.trim_end_matches('"');
    if let Some(start) = html.find(id_needle) {
        let after_prefix = start + id_needle.len();
        if let Some(end) = html[after_prefix..].find('"') {
            let msgnum_str = &html[after_prefix..after_prefix + end];
            if let Ok(n) = msgnum_str.parse::<i32>() {
                return Some(n);
            }
        }
    }
    // Legacy format: <a name="PREFIXmsgnum"> (for backward compatibility with old archives)
    let needle_str = format!("<a name=\"{}\"", fragment_prefix);
    let needle = needle_str.trim_end_matches('"');
    if let Some(start) = html.find(needle) {
        let after_prefix = start + needle.len();
        if let Some(end) = html[after_prefix..].find('"') {
            let msgnum_str = &html[after_prefix..after_prefix + end];
            return msgnum_str.parse::<i32>().ok();
        }
    }
    None
}

fn parse_old_html_comments(html: &str, msgnum: i32) -> Option<EmailInfo> {
    let mut name = None;
    let mut email_addr = None;
    let mut subject = None;
    let mut msgid = None;
    let mut inreplyto = None;
    let mut date_str = None;
    let mut from_date_str = None;
    let mut charset = None;
    let mut is_deleted = 0;
    let mut found_body = false;

    for line in html.lines() {
        let line = line.trim();
        if let Some(val) = extract_comment(line, "received") {
            from_date_str = Some(val.to_string());
        } else if let Some(val) = extract_comment(line, "sent") {
            date_str = Some(val.to_string());
        } else if let Some(val) = extract_comment(line, "name") {
            name = Some(val.to_string());
        } else if let Some(val) = extract_comment(line, "email") {
            email_addr = Some(val.to_string());
        } else if let Some(val) = extract_comment(line, "subject") {
            subject = Some(val.to_string());
        } else if let Some(val) = extract_comment(line, "id") {
            msgid = Some(val.to_string());
        } else if let Some(val) = extract_comment(line, "charset") {
            charset = Some(val.to_string());
        } else if let Some(val) = extract_comment(line, "inreplyto") {
            inreplyto = Some(val.to_string());
        } else if let Some(val) = extract_comment(line, "isdeleted") {
            is_deleted = val.parse().unwrap_or(0);
        } else if let Some(val) = extract_comment(line, "body") {
            if val == "start" {
                found_body = true;
            }
        }
    }

    if !found_body && msgid.is_none() {
        return None;
    }

    // Restore timestamps for incremental updates / date-based folders
    let date = date_str
        .as_deref()
        .and_then(parse_comment_timestamp)
        .or_else(|| from_date_str.as_deref().and_then(parse_comment_timestamp))
        .unwrap_or(0);
    let from_date = from_date_str.as_deref().and_then(parse_comment_timestamp).unwrap_or(date);

    Some(EmailInfo {
        msgnum,
        name,
        email_addr,
        from_date_str,
        from_date,
        date_str,
        date,
        datenum: date,
        subject,
        msgid,
        inreplyto,
        charset,
        is_deleted,
        ..Default::default()
    })
}

/// Parse a timestamp from an HTML comment value (ISO or RFC 2822).
fn parse_comment_timestamp(s: &str) -> Option<i64> {
    crate::date::iso_to_secs(s)
        .ok()
        .or_else(|| crate::date::parse_rfc2822_date(s).ok())
        .filter(|&t| t > 0)
}

fn extract_comment<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let pattern = format!("<!-- {}=\"", key);
    if let Some(start) = line.find(&pattern) {
        let val_start = start + pattern.len();
        if let Some(end) = line[val_start..].find('"') {
            return Some(&line[val_start..val_start + end]);
        }
    }
    None
}

/// Returns true if two Message-IDs match (trimmed comparison).
pub fn matches_existing(msgid: &str, existing_msgid: &str) -> bool {
    msgid.trim() == existing_msgid.trim()
}

/// Returns the path to the lock file for this archive.
pub fn lock_file_name(config: &Config) -> PathBuf {
    PathBuf::from(config.dir.as_deref().unwrap_or(".")).join(".hm_lock")
}

/// Acquires an exclusive non-blocking lock on the archive directory.
#[allow(unsafe_code)]
pub fn try_lock(config: &Config) -> std::io::Result<fs::File> {
    let path = lock_file_name(config);
    let file = fs::OpenOptions::new().write(true).create(true).truncate(true).open(&path)?;

    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;

        let fd = file.as_raw_fd();

        // SAFETY: This is a safe FFI call to the POSIX flock() system call.
        // - `fd` is a valid file descriptor obtained via AsRawFd()
        // - flock() is a standard POSIX function that cannot cause UB with valid fd
        // - LOCK_EX | LOCK_NB requests exclusive non-blocking lock (safe flags)
        // - Return value is checked for errors
        // - The file is owned by this function and fd remains valid
        let ret = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
        if ret != 0 {
            return Err(std::io::Error::last_os_error());
        }
    }

    // Write PID on all platforms (best-effort advisory on non-Unix)
    use std::io::Write;
    let _ = write!(&file, "{}", std::process::id());

    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::message::EmailInfo;

    fn make_email(msgnum: i32, msgid: &str) -> EmailInfo {
        EmailInfo { msgnum, msgid: Some(msgid.to_string()), date: 1000000, ..Default::default() }
    }

    #[test]
    fn test_message_name_sequential() {
        let config = Config::default();
        let email = make_email(42, "<a@b>");
        assert_eq!(message_name(&email, &config), "0042");
    }

    #[test]
    fn test_message_name_nonsequential() {
        let mut config = Config::default();
        config.nonsequential = true;
        let email = make_email(42, "<hello@world.com>");
        let name = message_name(&email, &config);
        assert_eq!(name.len(), 16);
        assert!(name.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_message_filename() {
        let config = Config::default();
        let email = make_email(7, "<a@b>");
        assert_eq!(message_filename(&email, &config), "0007.html");
    }

    #[test]
    fn test_fnv32_consistency() {
        let h1 = fnv32(b"test", 0);
        let h2 = fnv32(b"test", 0);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_fnv32_different() {
        let h1 = fnv32(b"abc", 0);
        let h2 = fnv32(b"xyz", 0);
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_extract_comment() {
        let line = "<!-- name=\"Alice\" -->";
        assert_eq!(extract_comment(line, "name"), Some("Alice"));
        assert_eq!(extract_comment(line, "subject"), None);
    }

    #[test]
    fn test_matches_existing() {
        assert!(matches_existing("<a@b>", "<a@b>"));
        assert!(!matches_existing("<a@b>", "<c@d>"));
    }

    #[test]
    fn test_dirpath() {
        let result = dirpath("/archives/%y/%m");
        assert!(result.starts_with("/archives/20"));
    }

    #[test]
    fn test_message_url_str_sequential() {
        let config = Config::default();
        let email = make_email(42, "<a@b>");
        assert_eq!(message_url_str(&email, &config), "0042.html");
    }

    #[test]
    fn test_message_url_str_nonsequential() {
        let mut config = Config::default();
        config.nonsequential = true;
        let email = make_email(42, "<hello@world.com>");
        let url = message_url_str(&email, &config);
        assert_eq!(url.len(), 21); // 16 hex + ".html"
        assert!(url.ends_with(".html"));
    }

    #[test]
    fn test_message_url_str_with_subdir() {
        let mut config = Config::default();
        config.msgsperfolder = 100;
        let email = make_email(142, "<a@b>");
        let url = message_url_str(&email, &config);
        assert_eq!(url, "1/0142.html");
    }

    #[test]
    fn test_extract_msgnum_from_html_basic() {
        let html = r#"<html><body><article id="msg42"></article></body></html>"#;
        let msgnum = extract_msgnum_from_html(html, "msg");
        assert_eq!(msgnum, Some(42));
    }

    #[test]
    fn test_extract_msgnum_from_html_no_match() {
        let html = r#"<html><body>no anchor here</body></html>"#;
        let msgnum = extract_msgnum_from_html(html, "msg");
        assert_eq!(msgnum, None);
    }

    #[test]
    fn test_extract_msgnum_from_html_invalid_prefix() {
        let html = r#"<html><body><article id="HM_42"></article></body></html>"#;
        let msgnum = extract_msgnum_from_html(html, "XYZ");
        assert_eq!(msgnum, None);
    }

    #[test]
    fn test_extract_msgnum_from_html_large_msgnum() {
        let html = r#"<html><body><article id="msg9999999"></article></body></html>"#;
        let msgnum = extract_msgnum_from_html(html, "msg");
        assert_eq!(msgnum, Some(9999999));
    }

    #[test]
    fn test_extract_msgnum_from_html_legacy_a_name() {
        // Backward compatibility: old archives use <a name="msg42">
        let html = r#"<html><body><a name="msg42"></a></body></html>"#;
        let msgnum = extract_msgnum_from_html(html, "msg");
        assert_eq!(msgnum, Some(42));
    }

    #[test]
    fn test_checkdir_creates_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let new_dir = tmp.path().join("new_subdir").to_string_lossy().to_string();
        assert!(!std::path::Path::new(&new_dir).exists());
        checkdir(&new_dir).unwrap();
        assert!(std::path::Path::new(&new_dir).exists());
    }

    #[test]
    fn test_checkdir_existing_dir_is_ok() {
        let tmp = tempfile::tempdir().unwrap();
        let result = checkdir(tmp.path().to_str().unwrap());
        assert!(result.is_ok());
    }

    #[test]
    fn test_is_empty_archive_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let mut config = Config::default();
        config.dir = Some(tmp.path().to_str().unwrap().to_string());
        assert!(is_empty_archive(&config));
    }

    #[test]
    fn test_is_empty_archive_with_file() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("index.html"), "content").unwrap();
        let mut config = Config::default();
        config.dir = Some(tmp.path().to_str().unwrap().to_string());
        assert!(!is_empty_archive(&config));
    }

    #[test]
    fn test_is_empty_archive_hidden_file_ignored() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(".hidden"), "content").unwrap();
        let mut config = Config::default();
        config.dir = Some(tmp.path().to_str().unwrap().to_string());
        assert!(is_empty_archive(&config));
    }

    #[test]
    fn test_write_and_read_messageindex_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let mut config = Config::default();
        config.dir = Some(tmp.path().to_str().unwrap().to_string());

        let mut store = crate::structs::EmailStore::new();
        store.add_email(make_email(0, "<a@b>"));
        store.add_email(make_email(1, "<b@b>"));
        store.add_email(make_email(2, "<c@b>"));

        write_messageindex(&store, &config).unwrap();
        let table = read_messageindex(&config).unwrap();

        assert_eq!(table[0].as_deref(), Some("0000"));
        assert_eq!(table[1].as_deref(), Some("0001"));
        assert_eq!(table[2].as_deref(), Some("0002"));
    }

    #[test]
    fn test_msg_subdir_msgsperfolder() {
        let mut config = Config::default();
        config.dir = Some("/tmp".to_string());
        config.msgsperfolder = 100;

        let email = make_email(250, "<a@b>");
        let sub = msg_subdir(&email, &config);
        assert!(sub.is_some());
        let info = sub.unwrap();
        assert_eq!(info.subdir, "2/");
    }

    #[test]
    fn test_msg_subdir_none_by_default() {
        let config = Config::default();
        let email = make_email(42, "<a@b>");
        assert!(msg_subdir(&email, &config).is_none());
    }

    #[test]
    fn test_message_path_sequential() {
        let mut config = Config::default();
        config.dir = Some("/tmp".to_string());
        let email = make_email(7, "<a@b>");
        let path = message_path(&email, &config);
        assert!(path.to_string_lossy().ends_with("0007.html"));
    }

    #[test]
    fn test_latest_folder_target_msgsperfolder() {
        let mut config = Config::default();
        config.dir = Some("/tmp".to_string());
        config.msgsperfolder = 100;
        let mut store = crate::structs::EmailStore::new();
        let mut e1 = make_email(50, "<a@b>");
        e1.date = 1000;
        let mut e2 = make_email(250, "<c@d>");
        e2.date = 2000;
        store.add_email(e1);
        store.add_email(e2);
        let target = latest_folder_target(&store, &config);
        assert_eq!(target, "2"); // msgnum 250 / 100
    }

    #[test]
    fn test_latest_folder_target_flat() {
        let config = Config::default();
        let mut store = crate::structs::EmailStore::new();
        store.add_email(make_email(1, "<a@b>"));
        assert_eq!(latest_folder_target(&store, &config), ".");
    }

    #[test]
    fn test_symlink_latest_rejects_path_traversal() {
        let tmpdir = std::env::temp_dir().join(format!("hm_test_symlink_{}", std::process::id()));
        let _ = fs::create_dir_all(&tmpdir);
        let mut config = Config::default();
        config.dir = Some(tmpdir.to_string_lossy().to_string());
        config.latest_folder = Some("../../evil".to_string());
        let store = crate::structs::EmailStore::new();
        // Must not error (guard returns Ok early) and must not create anything outside tmpdir.
        assert!(symlink_latest(&store, &config).is_ok());
        let escaped = tmpdir.parent().unwrap().parent().unwrap().join("evil");
        assert!(!escaped.exists());
        let _ = fs::remove_dir_all(&tmpdir);
    }

    #[test]
    fn test_symlink_latest_rejects_absolute_path() {
        let tmpdir =
            std::env::temp_dir().join(format!("hm_test_symlink_abs_{}", std::process::id()));
        let _ = fs::create_dir_all(&tmpdir);
        let mut config = Config::default();
        config.dir = Some(tmpdir.to_string_lossy().to_string());
        config.latest_folder = Some("/tmp/hm_evil_absolute_link".to_string());
        let store = crate::structs::EmailStore::new();
        assert!(symlink_latest(&store, &config).is_ok());
        assert!(!PathBuf::from("/tmp/hm_evil_absolute_link").exists());
        let _ = fs::remove_dir_all(&tmpdir);
    }

    #[test]
    fn test_parse_comment_timestamp_rfc2822() {
        let t = parse_comment_timestamp("Mon, 15 Mar 2021 12:00:00 +0000");
        assert!(t.is_some());
        assert!(t.unwrap() > 0);
    }
}
