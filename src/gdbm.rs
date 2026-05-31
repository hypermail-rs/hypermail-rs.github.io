use crate::config::Config;
use crate::date::secs_to_iso;
use crate::message::EmailInfo;
use crate::structs::EmailStore;
use std::fs;
use std::path::PathBuf;

pub fn gdbm_index_name(config: &Config) -> PathBuf {
    let dir = config.dir.as_deref().unwrap_or(".");
    PathBuf::from(dir).join(".hm2index")
}

pub fn togdbm(store: &EmailStore, config: &Config) -> std::io::Result<()> {
    let path = gdbm_index_name(config);
    let mut data = Vec::new();

    for email in &store.emails {
        let mut entry = Vec::new();
        push_field(&mut entry, email.from_date_str.as_deref().unwrap_or(""));
        push_field(&mut entry, email.date_str.as_deref().unwrap_or(""));
        push_field(&mut entry, email.name.as_deref().unwrap_or(""));
        push_field(&mut entry, email.email_addr.as_deref().unwrap_or(""));
        push_field(&mut entry, email.subject.as_deref().unwrap_or(""));
        push_field(&mut entry, email.msgid.as_deref().unwrap_or(""));
        push_field(&mut entry, email.inreplyto.as_deref().unwrap_or(""));
        push_field(&mut entry, email.charset.as_deref().unwrap_or(""));
        push_field(&mut entry, &secs_to_iso(email.from_date));
        push_field(&mut entry, &secs_to_iso(email.date));
        push_field(&mut entry, &email.exp_time.to_string());
        push_field(&mut entry, &email.is_deleted.to_string());

        let msgnum_bytes = email.msgnum.to_le_bytes();
        data.extend_from_slice(&msgnum_bytes);
        data.extend_from_slice(&(entry.len() as u32).to_le_bytes());
        data.extend_from_slice(&entry);
    }

    let max_msgnum = store.max_msgnum;
    let max_key = (-1i32).to_le_bytes();
    let max_val = max_msgnum.to_string();
    data.extend_from_slice(&max_key);
    data.extend_from_slice(&(max_val.len() as u32).to_le_bytes());
    data.extend_from_slice(max_val.as_bytes());

    let dl_key = "delete_level\0";
    let dl_val = config.delete_level.to_string();
    data.extend_from_slice(dl_key.as_bytes());
    data.extend_from_slice(&(dl_val.len() as u32).to_le_bytes());
    data.extend_from_slice(dl_val.as_bytes());

    fs::write(&path, &data)
}

fn push_field(data: &mut Vec<u8>, s: &str) {
    data.extend_from_slice(s.as_bytes());
    data.push(0);
}

pub fn load_from_gdbm(store: &mut EmailStore, config: &Config) -> std::io::Result<i32> {
    let path = gdbm_index_name(config);
    let data = fs::read(&path)?;
    let mut pos = 0;
    let mut count = 0;

    while pos < data.len() {
        if pos + 4 > data.len() {
            break;
        }
        let key_int = i32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
        pos += 4;

        if pos + 4 > data.len() {
            break;
        }
        let val_len =
            u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
        pos += 4;

        if pos + val_len > data.len() {
            break;
        }

        if key_int == -1 {
            // max_msgnum
            if let Ok(s) = String::from_utf8(data[pos..pos + val_len].to_vec()) {
                if let Ok(n) = s.trim().parse::<i32>() {
                    store.max_msgnum = n;
                }
            }
            pos += val_len;
            continue;
        }

        if key_int >= 0 {
            // Parse value fields (null-separated) - regular email entry
            let fields: Vec<&[u8]> = data[pos..pos + val_len].split(|&b| b == 0).collect();
            let get_field = |idx: usize| -> Option<String> {
                fields.get(idx).and_then(|f| {
                    if f.is_empty() {
                        None
                    } else {
                        String::from_utf8(f.to_vec()).ok()
                    }
                })
            };

            let email = EmailInfo {
                msgnum: key_int,
                from_date_str: get_field(0),
                date_str: get_field(1),
                name: get_field(2),
                email_addr: get_field(3),
                subject: get_field(4),
                msgid: get_field(5),
                inreplyto: get_field(6),
                charset: get_field(7),
                from_date: get_field(8).and_then(|s| s.parse().ok()).unwrap_or(0),
                date: get_field(9).and_then(|s| s.parse().ok()).unwrap_or(0),
                exp_time: get_field(10).and_then(|s| s.parse().ok()).unwrap_or(0),
                is_deleted: get_field(11).and_then(|s| s.parse().ok()).unwrap_or(0),
                ..Default::default()
            };

            if email.msgid.is_some() {
                let idx = store.add_email(email);
                store.insert_into_date_list(idx);
                store.insert_into_subject_list(idx);
                store.insert_into_author_list(idx);
                count += 1;
            }
        }

        pos += val_len;
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::message::EmailInfo;

    fn make_store() -> EmailStore {
        let mut store = EmailStore::new();
        let e1 = EmailInfo {
            msgnum: 1,
            msgid: Some("<a@b>".to_string()),
            name: Some("Alice".to_string()),
            subject: Some("Hello".to_string()),
            email_addr: Some("alice@e.com".to_string()),
            date_str: Some("Mon, 1 Jan 2024 12:00:00 +0000".to_string()),
            from_date_str: Some("Mon, 1 Jan 2024 12:00:00 +0000".to_string()),
            date: 1704110400,
            from_date: 1704110400,
            ..Default::default()
        };
        let e2 = EmailInfo {
            msgnum: 2,
            msgid: Some("<c@d>".to_string()),
            name: Some("Bob".to_string()),
            subject: Some("Re: Hello".to_string()),
            email_addr: Some("bob@e.com".to_string()),
            date_str: Some("Tue, 2 Jan 2024 12:00:00 +0000".to_string()),
            from_date_str: Some("Tue, 2 Jan 2024 12:00:00 +0000".to_string()),
            date: 1704196800,
            from_date: 1704196800,
            ..Default::default()
        };
        store.add_email(e1);
        store.add_email(e2);
        store
    }

    #[test]
    fn test_gdbm_roundtrip() {
        let store = make_store();
        let dir = tempfile::tempdir().unwrap();
        let mut config = Config::default();
        config.dir = Some(dir.path().to_string_lossy().to_string());

        togdbm(&store, &config).unwrap();

        let mut loaded = EmailStore::new();
        let count = load_from_gdbm(&mut loaded, &config).unwrap();
        assert!(count > 0);
        assert!(loaded.max_msgnum >= 2);
    }

    #[test]
    fn test_gdbm_index_name() {
        let config = Config::default();
        let path = gdbm_index_name(&config);
        assert!(path.to_string_lossy().contains(".hm2index"));
    }

    #[test]
    fn test_gdbm_msgnum_over_1m() {
        let mut store = EmailStore::new();
        let e1 = EmailInfo {
            msgnum: 2000000,
            msgid: Some("<big@num>".to_string()),
            name: Some("Big".to_string()),
            subject: Some("Large Msgnum".to_string()),
            email_addr: Some("big@e.com".to_string()),
            date_str: Some("Mon, 1 Jan 2024 12:00:00 +0000".to_string()),
            from_date_str: Some("Mon, 1 Jan 2024 12:00:00 +0000".to_string()),
            date: 1704110400,
            from_date: 1704110400,
            ..Default::default()
        };
        store.add_email(e1);

        let dir = tempfile::tempdir().unwrap();
        let mut config = Config::default();
        config.dir = Some(dir.path().to_string_lossy().to_string());

        togdbm(&store, &config).unwrap();

        let mut loaded = EmailStore::new();
        let count = load_from_gdbm(&mut loaded, &config).unwrap();
        assert_eq!(count, 1, "should have loaded exactly 1 email");
        assert_eq!(loaded.emails[0].msgnum, 2000000, "msgnum > 1M should survive roundtrip");
        assert_eq!(loaded.emails[0].subject.as_deref(), Some("Large Msgnum"));
    }
}
