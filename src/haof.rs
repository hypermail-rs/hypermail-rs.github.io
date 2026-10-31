use std::path::Path;

use crate::config::Config;
use crate::date::secs_to_iso;
use crate::error::Result;
use crate::headers::decode_mime_words;
use crate::structs::EmailStore;

pub fn write_haof(store: &EmailStore, config: &Config) -> Result<String> {
    let dir = config.dir.as_deref().unwrap_or(".");
    let haof_path = Path::new(dir).join("haof.xml");

    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<haof version=\"1.0\">\n");
    xml.push_str(&format!(
        "  <title>{}</title>\n",
        escape_xml(config.label.as_deref().unwrap_or("Archive"))
    ));
    xml.push_str("  <generator>hypermail-rs</generator>\n");
    xml.push_str(&format!("  <count>{}</count>\n", store.emails.len()));

    for email in &store.emails {
        xml.push_str("  <message>\n");
        xml.push_str(&format!("    <id>{}</id>\n", escape_xml(&email.msgnum.to_string())));
        xml.push_str(&format!("    <date>{}</date>\n", secs_to_iso(email.date)));
        xml.push_str(&format!(
            "    <subject>{}</subject>\n",
            escape_xml(&decode_mime_words(email.subject.as_deref().unwrap_or("(no subject)")))
        ));
        xml.push_str(&format!(
            "    <from>{}</from>\n",
            escape_xml(&decode_mime_words(email.name.as_deref().unwrap_or("Unknown")))
        ));
        if let Some(ref addr) = email.email_addr {
            xml.push_str(&format!("    <email>{}</email>\n", escape_xml(addr)));
        }
        if let Some(ref msgid) = email.msgid {
            xml.push_str(&format!("    <msgid>{}</msgid>\n", escape_xml(msgid)));
        }
        if let Some(ref inreplyto) = email.inreplyto {
            xml.push_str(&format!("    <inreplyto>{}</inreplyto>\n", escape_xml(inreplyto)));
        }
        xml.push_str("  </message>\n");
    }

    xml.push_str("</haof>\n");

    std::fs::write(&haof_path, &xml)?;
    crate::file_utils::apply_permissions(&haof_path, config.filemode);
    Ok(xml)
}

fn escape_xml(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&apos;"),
            c if !matches!(c, '\t' | '\n' | '\r' | '\u{20}'..='\u{d7ff}'
                | '\u{e000}'..='\u{fffd}' | '\u{10000}'..='\u{10ffff}') => {},
            c => result.push(c),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::message::EmailInfo;

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("<test&>"), "&lt;test&amp;&gt;");
        assert_eq!(escape_xml("plain"), "plain");
        assert_eq!(escape_xml("a\u{1}b\u{1b}c\td"), "abc\td");
        assert_eq!(escape_xml("a\u{fffe}\u{ffff}b"), "ab");
    }

    #[cfg(unix)]
    #[test]
    fn test_haof_uses_private_filemode() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let config = Config {
            dir: Some(dir.path().to_string_lossy().into_owned()),
            filemode: 0o600,
            ..Default::default()
        };
        write_haof(&EmailStore::new(), &config).unwrap();
        let mode = std::fs::metadata(dir.path().join("haof.xml")).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn test_write_haof_basic() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = EmailStore::new();
        let email = EmailInfo {
            msgnum: 1,
            name: Some("=?utf-8?Q?J=C3=BCrgen?=".to_string()),
            email_addr: Some("alice@example.com".to_string()),
            subject: Some("Hello".to_string()),
            date: 1000000,
            msgid: Some("<abc@e.com>".to_string()),
            ..Default::default()
        };
        store.add_email(email);
        let config =
            Config { dir: Some(dir.path().to_string_lossy().into_owned()), ..Default::default() };
        let xml = write_haof(&store, &config).unwrap();
        assert!(xml.contains("<subject>Hello</subject>"));
        assert!(xml.contains("<from>Jürgen</from>"), "{xml}");
        assert!(xml.contains("<count>1</count>"));
    }
}
