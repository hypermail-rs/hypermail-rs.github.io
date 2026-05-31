use crate::error::{HypermailError, Result};
use std::io::{BufRead, BufReader, Read};

/// Maximum size for a single email message (100 MB).
///
/// This prevents denial-of-service attacks via extremely large messages
/// that could exhaust memory. Legitimate emails are typically < 50 MB
/// even with large attachments.
const MAX_MESSAGE_SIZE: usize = 100 * 1024 * 1024;

/// Maximum line length for email headers or body lines (10 MB).
///
/// RFC 2822 recommends lines < 998 characters, but we allow much larger
/// for compatibility with malformed messages and large MIME parts.
/// This prevents memory exhaustion from pathological single-line inputs.
const MAX_LINE_SIZE: usize = 10 * 1024 * 1024;

/// Supported mbox format variants for "From " line escaping strategies.
#[derive(Debug, Clone, PartialEq)]
pub enum MboxFormat {
    MboxO,
    MboxRd,
    MboxCl,
    MboxCl2,
}

/// A single raw email message split into envelope line, headers, and body bytes.
#[derive(Debug, Clone)]
pub struct RawMessage {
    pub from_line: String,
    pub headers: Vec<u8>,
    pub body: Vec<u8>,
}

/// Streaming iterator that splits an mbox file into individual messages.
///
/// # Security
///
/// Enforces per-message and per-line size limits to prevent memory exhaustion
/// from malicious or malformed input.
pub struct MboxReader<R: Read> {
    reader: BufReader<R>,
    format: MboxFormat,
    line_num: usize,
    buffer: Vec<u8>,
    eof: bool,
    max_message_size: usize,
}

impl<R: Read> MboxReader<R> {
    /// Creates a new mbox reader with the given format variant.
    pub fn new(reader: R, format: MboxFormat) -> Self {
        MboxReader {
            reader: BufReader::new(reader),
            format,
            line_num: 0,
            buffer: Vec::new(),
            eof: false,
            max_message_size: MAX_MESSAGE_SIZE,
        }
    }

    /// Overrides the default maximum message size limit.
    pub fn with_max_message_size(mut self, size: usize) -> Self {
        self.max_message_size = size;
        self
    }

    fn is_from_line(line: &[u8]) -> bool {
        line.starts_with(b"From ") && line.len() > 5
    }

    #[allow(dead_code)]
    fn is_from_line_mboxrd(line: &[u8]) -> bool {
        line.starts_with(b">From ") || Self::is_from_line(line)
    }

    fn unescape_mboxrd(line: &[u8]) -> Vec<u8> {
        // MboxRd: strip exactly one leading '>' from lines that start with
        // one or more '>' followed by "From ". This correctly handles
        // multi-level escaping: ">>From " → ">From ", ">>>From " → ">>From ".
        if line.len() > 1 && line[0] == b'>' {
            // Count leading '>' characters
            let gt_count = line.iter().take_while(|&&b| b == b'>').count();
            // Check if after the '>' sequence we have "From "
            if line[gt_count..].starts_with(b"From ") {
                return line[1..].to_vec();
            }
        }
        line.to_vec()
    }
}

impl<R: Read> Iterator for MboxReader<R> {
    type Item = Result<RawMessage>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.eof {
            return None;
        }

        loop {
            let mut line = Vec::new();
            self.line_num += 1;

            match self.reader.read_until(b'\n', &mut line) {
                Ok(0) => {
                    self.eof = true;
                    if self.buffer.is_empty() {
                        return None;
                    }
                    break;
                },
                Ok(_) => {},
                Err(e) => {
                    return Some(Err(HypermailError::MboxParse {
                        line: self.line_num,
                        message: format!("read error: {e}"),
                    }))
                },
            }

            // Security: Check line size to prevent memory exhaustion
            // from pathological inputs with extremely long lines
            if line.len() > MAX_LINE_SIZE {
                return Some(Err(HypermailError::MboxParse {
                    line: self.line_num,
                    message: format!(
                        "line exceeds maximum size ({} bytes > {} bytes)",
                        line.len(),
                        MAX_LINE_SIZE
                    ),
                }));
            }

            if line.last() == Some(&b'\n') {
                line.pop();
                if line.last() == Some(&b'\r') {
                    line.pop();
                }
            }

            if self.buffer.is_empty() {
                if Self::is_from_line(&line) {
                    self.buffer = line;
                    self.buffer.push(b'\n');
                    continue;
                }
                self.buffer = line;
                self.buffer.push(b'\n');
                continue;
            }

            if Self::is_from_line(&line) {
                let raw = self.buffer.split_off(0);
                self.buffer = line;
                self.buffer.push(b'\n');
                return Some(Ok(parse_raw_message(&raw)));
            }

            // Security: Check total message size before appending
            // Prevents denial-of-service via extremely large messages
            let new_size = self.buffer.len() + line.len() + 1;
            if new_size > self.max_message_size {
                self.buffer.clear();
                return Some(Err(HypermailError::MboxParse {
                    line: self.line_num,
                    message: format!(
                        "message exceeds maximum size ({} bytes > {} bytes)",
                        new_size, self.max_message_size
                    ),
                }));
            }

            if self.format == MboxFormat::MboxRd {
                let unescaped = Self::unescape_mboxrd(&line);
                self.buffer.extend_from_slice(&unescaped);
            } else {
                self.buffer.extend_from_slice(&line);
            }
            self.buffer.push(b'\n');
        }

        if self.buffer.is_empty() {
            return None;
        }
        let raw = std::mem::take(&mut self.buffer);
        Some(Ok(parse_raw_message(&raw)))
    }
}

fn parse_raw_message(data: &[u8]) -> RawMessage {
    let from_end = data.iter().position(|&b| b == b'\n').unwrap_or(data.len());
    let from_line = String::from_utf8_lossy(&data[..from_end]).trim_end().to_string();

    let rest = if from_end + 1 < data.len() {
        &data[from_end + 1..]
    } else {
        &[]
    };

    let sep = rest
        .windows(2)
        .position(|w| w == b"\n\n")
        .or_else(|| rest.windows(4).position(|w| w == b"\r\n\r\n").map(|p| p + 2));

    if let Some(headers_end) = sep {
        let header_bytes = rest[..headers_end].to_vec();
        let body_bytes = if headers_end + 2 < rest.len() {
            let skip = if rest[headers_end..].starts_with(b"\r\n") {
                4
            } else {
                2
            };
            if headers_end + skip < rest.len() {
                rest[headers_end + skip..].to_vec()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };
        RawMessage { from_line, headers: header_bytes, body: body_bytes }
    } else {
        RawMessage { from_line, headers: rest.to_vec(), body: Vec::new() }
    }
}

/// Reads an entire mbox file into a vector of raw messages.
pub fn read_mbox_file(path: &str, format: MboxFormat) -> Result<Vec<RawMessage>> {
    let file = std::fs::File::open(path).map_err(HypermailError::Io)?;
    let reader = MboxReader::new(file, format);
    let mut messages = Vec::new();
    for msg in reader {
        messages.push(msg?);
    }
    Ok(messages)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn create_mbox_data() -> Vec<u8> {
        b"From alice@example.com Mon Jan 01 12:00:00 2024\n\
          From: Alice <alice@example.com>\n\
          Subject: First message\n\
          Message-ID: <001@example.com>\n\
          Date: Mon, 01 Jan 2024 12:00:00 +0000\n\
          \n\
          This is the first message body.\n\
          \n\
          From bob@example.com Mon Jan 01 13:00:00 2024\n\
          From: Bob <bob@example.com>\n\
          Subject: Re: First message\n\
          Message-ID: <002@example.com>\n\
          In-Reply-To: <001@example.com>\n\
          Date: Mon, 01 Jan 2024 13:00:00 +0000\n\
          \n\
          This is a reply.\n\
          \n\
          From carol@example.com Mon Jan 01 14:00:00 2024\n\
          From: Carol <carol@example.com>\n\
          Subject: Another thread\n\
          Message-ID: <003@example.com>\n\
          Date: Mon, 01 Jan 2024 14:00:00 +0000\n\
          \n\
          A different conversation.\n"
            .to_vec()
    }

    #[test]
    fn test_parse_mbox_basic() {
        let data = create_mbox_data();
        let cursor = Cursor::new(data);
        let reader = MboxReader::new(cursor, MboxFormat::MboxO);
        let messages: Vec<Result<RawMessage>> = reader.collect();
        assert_eq!(messages.len(), 3);

        let msg0 = messages[0].as_ref().unwrap();
        assert!(msg0.from_line.contains("alice@example.com"));

        let headers = crate::headers::parse_headers(&msg0.headers);
        assert_eq!(crate::headers::find_header(&headers, "Subject"), Some("First message"));
    }

    #[test]
    fn test_from_line_parsing() {
        assert!(MboxReader::<std::io::Empty>::is_from_line(b"From alice@example.com Mon Jan 01"));
        assert!(!MboxReader::<std::io::Empty>::is_from_line(b"From: alice@example.com"));
        assert!(!MboxReader::<std::io::Empty>::is_from_line(b""));
    }

    #[test]
    fn test_empty_mbox() {
        let cursor = Cursor::new(b"");
        let reader = MboxReader::new(cursor, MboxFormat::MboxO);
        let count = reader.count();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_mbox_no_headers() {
        let data = b"From alice@example.com\n\nJust a body\n".to_vec();
        let cursor = Cursor::new(data);
        let reader = MboxReader::new(cursor, MboxFormat::MboxO);
        let messages: Vec<Result<RawMessage>> = reader.collect();
        assert_eq!(messages.len(), 1);
        assert!(messages[0].is_ok());
    }

    #[test]
    fn test_multipart_message_with_from_in_body() {
        let data = b"From alice@example.com\n\
                     From: Alice <alice@example.com>\n\
                     Subject: Test\n\
                     \n\
                     This line looks like\n\
                     >From someone else\n\
                     but shouldn't be split.\n"
            .to_vec();
        let cursor = Cursor::new(data);
        let reader = MboxReader::new(cursor, MboxFormat::MboxRd);
        let messages: Vec<Result<RawMessage>> = reader.collect();
        assert_eq!(messages.len(), 1);
        let msg = messages[0].as_ref().unwrap();
        assert_eq!(msg.from_line, "From alice@example.com");
        let body_str = std::str::from_utf8(&msg.body).unwrap();
        assert!(body_str.contains("From someone else"));
    }

    #[test]
    fn test_max_message_size_exceeded() {
        let data = b"From alice@example.com\n\
                     From: Alice <alice@example.com>\n\
                     \n\
                     This is a very long body line that exceeds our tiny limit.\n"
            .to_vec();
        let cursor = Cursor::new(data);
        let reader = MboxReader::new(cursor, MboxFormat::MboxO).with_max_message_size(10);
        let results: Vec<Result<RawMessage>> = reader.collect();
        assert!(
            results.iter().any(|r| r.is_err()),
            "Should fail when message exceeds size limit"
        );
    }

    #[test]
    fn test_mboxrd_unescape() {
        // MboxRd: ">From " at start of body line is an escaped "From " and must be unescaped
        let data = b"From alice@example.com\n\
                     Subject: Test\n\
                     \n\
                     >From someone we know\n"
            .to_vec();
        let cursor = Cursor::new(data);
        let reader = MboxReader::new(cursor, MboxFormat::MboxRd);
        let messages: Vec<Result<RawMessage>> = reader.collect();
        assert_eq!(messages.len(), 1);
        let msg = messages[0].as_ref().unwrap();
        let body = std::str::from_utf8(&msg.body).unwrap();
        assert!(body.contains("From someone"), "'>From' should be unescaped to 'From'");
        assert!(!body.contains(">From"), "unescaped body should not contain '>From'");
    }

    #[test]
    fn test_parse_mbox_three_messages_bodies() {
        let data = create_mbox_data();
        let cursor = Cursor::new(data);
        let reader = MboxReader::new(cursor, MboxFormat::MboxO);
        let messages: Vec<Result<RawMessage>> = reader.collect();
        assert_eq!(messages.len(), 3);
        let body0 = std::str::from_utf8(&messages[0].as_ref().unwrap().body).unwrap();
        assert!(body0.contains("first message body"));
        let body1 = std::str::from_utf8(&messages[1].as_ref().unwrap().body).unwrap();
        assert!(body1.contains("reply"));
    }
}
