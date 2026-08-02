use crate::error::Result;

/// Parsed MIME Content-Type header with type, subtype, and parameters.
#[derive(Debug, Clone)]
pub struct ContentType {
    pub type_: String,
    pub subtype: String,
    pub params: std::collections::HashMap<String, String>,
}

impl ContentType {
    /// Parses a Content-Type header value into structured components.
    pub fn parse(s: &str) -> Self {
        let s = s.trim();
        let mut params = std::collections::HashMap::new();

        let (base, param_str) = if let Some(semi) = s.find(';') {
            (s[..semi].trim(), Some(s[semi + 1..].trim()))
        } else {
            (s, None)
        };

        let (type_, subtype) = if let Some(slash) = base.find('/') {
            (base[..slash].trim().to_lowercase(), base[slash + 1..].trim().to_lowercase())
        } else {
            (base.to_lowercase(), "".to_string())
        };

        if let Some(pstr) = param_str {
            for part in pstr.split(';') {
                let part = part.trim();
                if let Some(eq) = part.find('=') {
                    let key = part[..eq].trim().to_lowercase();
                    let mut val = part[eq + 1..].trim().to_string();
                    if (val.starts_with('"') && val.ends_with('"'))
                        || (val.starts_with('\'') && val.ends_with('\''))
                    {
                        val = val[1..val.len() - 1].to_string();
                    }
                    params.insert(key, val);
                }
            }
        }

        ContentType { type_, subtype, params }
    }

    pub fn is_text(&self) -> bool {
        self.type_ == "text"
    }

    pub fn is_multipart(&self) -> bool {
        self.type_ == "multipart"
    }

    pub fn boundary(&self) -> Option<&str> {
        self.params.get("boundary").map(|s| s.as_str())
    }

    pub fn charset(&self) -> Option<&str> {
        self.params.get("charset").map(|s| s.as_str())
    }

    pub fn name(&self) -> Option<&str> {
        self.params.get("name").map(|s| s.as_str())
    }

    pub fn full_type(&self) -> String {
        format!("{}/{}", self.type_, self.subtype)
    }
}

/// Parsed MIME Content-Disposition header with disposition type and parameters.
#[derive(Debug, Clone)]
pub struct ContentDisposition {
    pub disposition: String,
    pub params: std::collections::HashMap<String, String>,
}

impl ContentDisposition {
    /// Parses a Content-Disposition header value into structured components.
    pub fn parse(s: &str) -> Self {
        let s = s.trim();
        let mut params = std::collections::HashMap::new();

        let (disp, param_str) = if let Some(semi) = s.find(';') {
            (s[..semi].trim().to_lowercase(), Some(s[semi + 1..].trim()))
        } else {
            (s.to_lowercase(), None)
        };

        if let Some(pstr) = param_str {
            for part in pstr.split(';') {
                let part = part.trim();
                if let Some(eq) = part.find('=') {
                    let key = part[..eq].trim().to_lowercase();
                    let mut val = part[eq + 1..].trim().to_string();
                    if (val.starts_with('"') && val.ends_with('"'))
                        || (val.starts_with('\'') && val.ends_with('\''))
                    {
                        val = val[1..val.len() - 1].to_string();
                    }
                    params.insert(key, val);
                }
            }
        }

        ContentDisposition { disposition: disp, params }
    }

    pub fn filename(&self) -> Option<&str> {
        self.params.get("filename").map(|s| s.as_str())
    }

    pub fn is_attachment(&self) -> bool {
        self.disposition == "attachment"
    }
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'A'..=b'F' => Some(b - b'A' + 10),
        b'a'..=b'f' => Some(b - b'a' + 10),
        _ => None,
    }
}

/// Decodes base64-encoded data, ignoring whitespace.
pub fn decode_base64(data: &[u8]) -> Result<Vec<u8>> {
    let text = std::str::from_utf8(data)
        .map_err(|e| crate::error::HypermailError::Parse(format!("Invalid base64 text: {e}")))?;

    let clean: String = text.chars().filter(|c| !c.is_whitespace()).collect();

    use base64::Engine as _;
    let engine = base64::engine::general_purpose::STANDARD;
    engine
        .decode(&clean)
        .map_err(|e| crate::error::HypermailError::Parse(format!("Base64 decode error: {e}")))
}

/// Decodes quoted-printable encoded data, handling soft line breaks and `_` as space.
pub fn decode_quoted_printable(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(data.len());
    let mut i = 0;
    while i < data.len() {
        if data[i] == b'=' {
            if i + 2 < data.len() && data[i + 1] == b'\r' && data[i + 2] == b'\n' {
                // Soft line break: =\r\n
                i += 3;
                continue;
            }
            if i + 1 < data.len() && data[i + 1] == b'\n' {
                // Soft line break: =\n (Unix-style, no \r)
                i += 2;
                continue;
            }
            if i + 2 < data.len() {
                if let (Some(h), Some(l)) = (hex_val(data[i + 1]), hex_val(data[i + 2])) {
                    result.push(h << 4 | l);
                    i += 3;
                    continue;
                }
            }
        }
        if data[i] == b'_' {
            result.push(b' ');
        } else if data[i] != b'\r' {
            result.push(data[i]);
        }
        i += 1;
    }
    result
}

/// Decodes uuencoded data, returning `None` if no valid uuencode block is found.
pub fn decode_uuencode(data: &[u8]) -> Option<Vec<u8>> {
    let text = std::str::from_utf8(data).ok()?;
    let mut result = Vec::new();
    let mut in_encoded = false;

    for line in text.lines() {
        let line = line.trim_end();
        if line.starts_with("begin ") {
            in_encoded = true;
            continue;
        }
        if line == "end" || line == "`" {
            in_encoded = false;
            continue;
        }
        if !in_encoded || line.is_empty() {
            continue;
        }

        let bytes = line.as_bytes();
        if bytes.is_empty() {
            continue;
        }

        let count = (bytes[0] as usize - 32) & 0x3f;
        if count == 0 {
            continue;
        }

        let mut buf = [0u8; 3];
        let mut j = 1;
        let mut out = 0;

        while j < bytes.len() && out < count {
            let mut chars = [0u8; 4];
            let mut n = 0;
            while n < 4 && j < bytes.len() {
                chars[n] = bytes[j].wrapping_sub(32) & 0x3f;
                j += 1;
                n += 1;
            }

            if n >= 2 {
                buf[0] = (chars[0] << 2) | (chars[1] >> 4);
            }
            if n >= 3 {
                buf[1] = (chars[1] << 4) | (chars[2] >> 2);
            }
            if n >= 4 {
                buf[2] = (chars[2] << 6) | chars[3];
            }

            let to_push = n.saturating_sub(1);
            result.extend_from_slice(&buf[..to_push]);
            out += to_push;
        }
    }

    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

/// Combined MIME content-type and transfer-encoding information for a message part.
#[derive(Debug, Clone)]
pub struct MimeInfo {
    pub content_type: ContentType,
    pub content_transfer_encoding: Option<String>,
}

/// Extracts MIME info (content-type and transfer-encoding) from parsed headers.
pub fn parse_mime_info(headers: &[(String, String)]) -> Option<MimeInfo> {
    let ct_str = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
        .map(|(_, val)| val.as_str())?;

    let content_type = ContentType::parse(ct_str);
    let cte = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-transfer-encoding"))
        .map(|(_, val)| val.trim().to_lowercase());

    Some(MimeInfo { content_type, content_transfer_encoding: cte })
}

fn find_multipart_charset(body: &[u8], boundary: &str) -> Option<String> {
    let boundary_tag = format!("--{}", boundary);
    let boundary_bytes = boundary_tag.as_bytes();
    let mut pos = 0;

    while pos < body.len() {
        // Find next boundary starting from pos
        let start =
            match body[pos..].windows(boundary_bytes.len()).position(|w| w == boundary_bytes) {
                Some(offset) => pos + offset,
                None => break, // No more boundaries found, exit loop
            };

        let after_boundary = &body[start + boundary_bytes.len()..];

        // Verify boundary is followed by newline (not just part of content)
        let after_eol = if after_boundary.starts_with(b"\r\n") {
            &after_boundary[2..]
        } else if after_boundary.starts_with(b"\n") {
            &after_boundary[1..]
        } else {
            // Not a valid boundary, continue searching
            pos = start + 1;
            continue;
        };

        // Find end of headers (empty line)
        let header_end = after_eol
            .windows(2)
            .position(|w| w == b"\n\n")
            .or_else(|| after_eol.windows(4).position(|w| w == b"\r\n\r\n").map(|p| p + 2));

        if let Some(header_end) = header_end {
            let part_headers = &after_eol[..header_end];
            if let Ok(header_block) = std::str::from_utf8(part_headers) {
                for line in header_block.lines() {
                    let lower = line.to_lowercase();
                    if lower.starts_with("content-type:") {
                        if let Some(charset_start) = lower.find("charset=") {
                            let after = &line[charset_start + 8..];
                            let charset = after.trim().trim_matches('"').trim_matches('\'');
                            let charset =
                                charset.split([';', ' ', '\r', '\n']).next().unwrap_or(charset);
                            if !charset.is_empty() {
                                return Some(charset.to_string());
                            }
                        }
                    }
                }
            }
        }

        // Move past this boundary to search for next part
        pos = start + boundary_bytes.len();
    }
    None
}

/// Decodes a MIME message body using its content-type and transfer-encoding.
///
/// Handles charset conversion, multipart boundaries, and format=flowed unwrapping.
pub fn decode_body(body: &[u8], mime_info: &MimeInfo) -> String {
    let decoded_bytes = match mime_info.content_transfer_encoding.as_deref() {
        Some("base64") => match decode_base64(body) {
            Ok(bytes) => bytes,
            Err(_) => body.to_vec(),
        },
        Some("quoted-printable") | Some("qp") => decode_quoted_printable(body),
        // 7bit, 8bit, binary → use raw bytes
        _ => body.to_vec(),
    };

    let charset: Option<String> =
        mime_info.content_type.charset().map(|s| s.to_string()).or_else(|| {
            if mime_info.content_type.is_multipart() {
                if let Some(boundary) = mime_info.content_type.boundary() {
                    find_multipart_charset(body, boundary)
                } else {
                    None
                }
            } else {
                None
            }
        });

    // Use smart charset detection that handles mislabeled charsets
    if let Some(ref charset) = charset {
        return crate::headers::decode_to_utf8(&decoded_bytes, charset);
    }

    // No charset specified: try UTF-8 first, then common fallbacks
    if let Ok(s) = std::str::from_utf8(&decoded_bytes) {
        return s.to_string();
    }

    // Try common Greek/European charsets as fallback
    for label in &["windows-1253", "iso-8859-7", "iso-8859-1", "windows-1252"] {
        if let Some(encoding) = encoding_rs::Encoding::for_label(label.as_bytes()) {
            let (cow, _, _) = encoding.decode(&decoded_bytes);
            if !cow.contains('\u{FFFD}') {
                return cow.into_owned();
            }
        }
    }

    String::from_utf8_lossy(&decoded_bytes).to_string()
}

/// QUAL-4: Variant of `decode_body` that skips internal charset resolution,
/// using the already-resolved `charset` string instead.
fn decode_body_with_charset(body: &[u8], mime_info: &MimeInfo, charset: Option<&str>) -> String {
    let decoded_bytes = match mime_info.content_transfer_encoding.as_deref() {
        Some("base64") => match decode_base64(body) {
            Ok(bytes) => bytes,
            Err(_) => body.to_vec(),
        },
        Some("quoted-printable") | Some("qp") => decode_quoted_printable(body),
        _ => body.to_vec(),
    };

    if let Some(cs) = charset {
        return crate::headers::decode_to_utf8(&decoded_bytes, cs);
    }

    if let Ok(s) = std::str::from_utf8(&decoded_bytes) {
        return s.to_string();
    }

    for label in &["windows-1253", "iso-8859-7", "iso-8859-1", "windows-1252"] {
        if let Some(encoding) = encoding_rs::Encoding::for_label(label.as_bytes()) {
            let (cow, _, _) = encoding.decode(&decoded_bytes);
            if !cow.contains('\u{FFFD}') {
                return cow.into_owned();
            }
        }
    }

    String::from_utf8_lossy(&decoded_bytes).to_string()
}

fn resolve_charset(body_raw: &[u8], mi: &MimeInfo) -> Option<String> {
    mi.content_type.charset().map(|s| s.to_string()).or_else(|| {
        if mi.content_type.is_multipart() {
            if let Some(boundary) = mi.content_type.boundary() {
                find_multipart_charset(body_raw, boundary)
            } else {
                None
            }
        } else {
            None
        }
    })
}

/// Maximum nested multipart depth to prevent stack overflow from crafted messages.
const MAX_MULTIPART_DEPTH: u32 = 16;

/// Processes a MIME message body, returning decoded text and detected charset.
///
/// Handles multipart messages, inline images, attachments, and charset detection.
///
/// # Security
///
/// Only safe image MIME types are embedded inline; SVG is excluded due to script risks.
/// Nested multiparts are limited to [`MAX_MULTIPART_DEPTH`] levels.
pub fn process_mime_body(
    headers: &[(String, String)],
    body_raw: &[u8],
) -> (String, Option<String>) {
    process_mime_body_depth(headers, body_raw, 0)
}

fn process_mime_body_depth(
    headers: &[(String, String)],
    body_raw: &[u8],
    depth: u32,
) -> (String, Option<String>) {
    let mi = parse_mime_info(headers);
    if let Some(ref mi) = mi {
        // Check if this is a multipart message
        if mi.content_type.is_multipart() {
            if depth >= MAX_MULTIPART_DEPTH {
                log::warn!(
                    "multipart nesting depth limit ({}) exceeded; treating as plain text",
                    MAX_MULTIPART_DEPTH
                );
                return (String::from_utf8_lossy(body_raw).to_string(), None);
            }
            if let Some(boundary) = mi.content_type.boundary() {
                return process_multipart_body(body_raw, boundary, mi, depth);
            }
        }

        let charset = resolve_charset(body_raw, mi);
        // QUAL-4: Use decode_body_with_charset to avoid resolving charset twice.
        let mut decoded = decode_body_with_charset(body_raw, mi, charset.as_deref());
        // RFC 3676: unwrap format=flowed text
        if mi
            .content_type
            .params
            .get("format")
            .map(|v| v.eq_ignore_ascii_case("flowed"))
            .unwrap_or(false)
        {
            decoded = unflow_text(&decoded);
        }
        (decoded, charset)
    } else {
        // No Content-Type header: try UTF-8 first, then fallback charsets
        if let Ok(s) = std::str::from_utf8(body_raw) {
            return (s.to_string(), None);
        }
        for label in &["windows-1253", "iso-8859-7", "iso-8859-1", "windows-1252"] {
            if let Some(encoding) = encoding_rs::Encoding::for_label(label.as_bytes()) {
                let (cow, _, _) = encoding.decode(body_raw);
                if !cow.contains('\u{FFFD}') {
                    return (cow.into_owned(), Some(label.to_string()));
                }
            }
        }
        (String::from_utf8_lossy(body_raw).to_string(), None)
    }
}

fn process_multipart_body(
    body: &[u8],
    boundary: &str,
    parent_mime: &MimeInfo,
    depth: u32,
) -> (String, Option<String>) {
    let is_alternative = parent_mime.content_type.subtype == "alternative";
    let boundary_tag = format!("--{}", boundary);
    let boundary_bytes = boundary_tag.as_bytes();
    let mut result = String::new();
    let mut detected_charset = None;
    let mut pos = 0;

    // For multipart/alternative: collect all text parts, then pick the best one.
    // Prefer text/plain over text/html to avoid rendering raw HTML.
    let mut alt_plain: Option<(String, Option<String>)> = None;
    let mut alt_html: Option<(String, Option<String>)> = None;

    while pos < body.len() {
        // Find next boundary
        let start =
            match body[pos..].windows(boundary_bytes.len()).position(|w| w == boundary_bytes) {
                Some(offset) => pos + offset,
                None => break,
            };

        // Check for end boundary
        let after_boundary = &body[start + boundary_bytes.len()..];
        if after_boundary.starts_with(b"--") {
            // End boundary found
            break;
        }

        // Skip to content after boundary line
        let after_eol = if after_boundary.starts_with(b"\r\n") {
            &after_boundary[2..]
        } else if after_boundary.starts_with(b"\n") {
            &after_boundary[1..]
        } else {
            pos = start + boundary_bytes.len();
            continue;
        };

        // Find end of part headers
        let header_end = after_eol
            .windows(2)
            .position(|w| w == b"\n\n")
            .or_else(|| after_eol.windows(4).position(|w| w == b"\r\n\r\n").map(|p| p + 2));

        if let Some(header_end) = header_end {
            let part_headers_bytes = &after_eol[..header_end];
            let part_body_start = header_end + 2;

            // Find next boundary to determine part body end
            let part_body = if let Some(next_boundary_pos) = after_eol[part_body_start..]
                .windows(boundary_bytes.len())
                .position(|w| w == boundary_bytes)
            {
                &after_eol[part_body_start..part_body_start + next_boundary_pos]
            } else {
                &after_eol[part_body_start..]
            };

            // Parse part headers
            if let Ok(headers_str) = std::str::from_utf8(part_headers_bytes) {
                let mut part_headers = Vec::new();
                for line in headers_str.lines() {
                    if let Some((name, value)) = line.split_once(':') {
                        part_headers.push((name.trim().to_lowercase(), value.trim().to_string()));
                    }
                }

                // Check if this part is an attachment, inline content, or has Content-ID
                let mut is_attachment = false;
                let mut _has_content_id = false;
                let mut content_type_main = String::new();
                let mut encoding = String::new();

                for (name, value) in &part_headers {
                    if name == "content-disposition" {
                        is_attachment = value.to_lowercase().starts_with("attachment");
                    }
                    if name == "content-id" {
                        _has_content_id = true;
                    }
                    if name == "content-type" {
                        if let Some(main_type) = value.split(';').next() {
                            content_type_main = main_type.trim().to_lowercase();
                        }
                    }
                    if name == "content-transfer-encoding" {
                        encoding = value.trim().to_lowercase();
                    }
                }

                // Allowlist of safe image MIME types for inline embedding.
                // image/svg+xml is excluded — SVG can contain scripts.
                const SAFE_IMAGE_TYPES: &[&str] = &[
                    "image/gif",
                    "image/jpeg",
                    "image/jpg",
                    "image/png",
                    "image/webp",
                    "image/bmp",
                    "image/tiff",
                ];

                // Determine how to handle this part
                if content_type_main.starts_with("image/")
                    && SAFE_IMAGE_TYPES.contains(&content_type_main.as_str())
                {
                    // Always embed images inline for the HTML archive — viewers browse,
                    // they don't download.  Content-Disposition: attachment is an email-client
                    // hint that does not apply here.  Only fall back to a link if the raw
                    // data is missing or decoding fails.
                    let image_data = if encoding == "base64" {
                        decode_base64(part_body.trim_ascii()).ok()
                    } else if !part_body.is_empty() {
                        Some(part_body.to_vec())
                    } else {
                        None
                    };

                    if let Some(data) = image_data {
                        use base64::Engine as _;
                        let engine = base64::engine::general_purpose::STANDARD;
                        let base64_data = engine.encode(&data);
                        if !result.is_empty() {
                            result.push('\n');
                        }
                        result.push_str(&format!(
                            "[INLINE_IMAGE:{}:{}]\n",
                            content_type_main, base64_data
                        ));
                    } else if let Some(filename) = extract_filename(&part_headers) {
                        // Decoding failed — fall back to a named attachment link
                        if !result.is_empty() {
                            result.push('\n');
                        }
                        result.push_str(&format!("[Attachment: {}]\n", filename));
                    }
                } else if content_type_main.starts_with("image/")
                    || is_attachment
                    || content_type_main.starts_with("application/")
                {
                    // Non-safe image or non-image attachment - just note it
                    if let Some(filename) = extract_filename(&part_headers) {
                        if !result.is_empty() {
                            result.push('\n');
                        }
                        result.push_str(&format!("[Attachment: {}]\n", filename));
                    }
                } else if content_type_main.starts_with("text/")
                    || content_type_main.starts_with("multipart/")
                    || content_type_main.is_empty()
                {
                    // Process text / nested multipart content (depth-limited)
                    let (decoded, charset) =
                        process_mime_body_depth(&part_headers, part_body, depth + 1);
                    if is_alternative {
                        // LOG-2: For multipart/alternative, collect parts separately.
                        if content_type_main == "text/plain" || content_type_main.is_empty() {
                            if alt_plain.is_none() {
                                alt_plain = Some((decoded, charset));
                            }
                        } else if content_type_main == "text/html" && alt_html.is_none() {
                            alt_html = Some((decoded, charset));
                        }
                        // other text/* subtypes ignored for alternative
                    } else {
                        if detected_charset.is_none() && charset.is_some() {
                            detected_charset = charset;
                        }
                        if !result.is_empty() && !decoded.is_empty() {
                            result.push('\n');
                        }
                        result.push_str(&decoded);
                    }
                }
            }
        }

        // Move to next part
        pos = start + boundary_bytes.len();
    }

    // LOG-2: For multipart/alternative, select the single best part.
    // Prefer text/plain; fall back to text/html if no plain part exists.
    if is_alternative {
        let chosen = alt_plain.or(alt_html);
        if let Some((text, charset)) = chosen {
            return (text, charset);
        }
        return (result, detected_charset);
    }

    (result, detected_charset)
}

fn extract_filename(headers: &[(String, String)]) -> Option<String> {
    for (name, value) in headers {
        if name == "content-disposition" || name == "content-type" {
            // Try RFC 2231 continuation first (filename*0=, filename*1=, ...)
            if let Some(f) = extract_rfc2231_filename(value) {
                return Some(f);
            }
            // Try RFC 2231 charset encoding (filename*=charset'lang'value)
            if let Some(f) = extract_rfc2231_encoded_filename(value) {
                return Some(f);
            }
            // Fall back to simple filename= or name=
            for param in value.split(';') {
                let param = param.trim();
                if let Some(filename_part) =
                    param.strip_prefix("filename=").or_else(|| param.strip_prefix("name="))
                {
                    let filename = filename_part.trim().trim_matches('"').trim_matches('\'');
                    if !filename.is_empty() {
                        return Some(filename.to_string());
                    }
                }
            }
        }
    }
    None
}

fn extract_rfc2231_filename(value: &str) -> Option<String> {
    // RFC 2231 allows arbitrarily many continuation segments (filename*0=, *1=, ...).
    // A pathological message could supply many huge segments that concatenate into
    // hundreds of MB of `String`. Cap the reassembled length to defend against
    // memory exhaustion; legitimate filenames are well under this.
    const MAX_FILENAME_LEN: usize = 8 * 1024;

    let mut parts: Vec<(usize, String)> = Vec::new();
    for param in value.split(';') {
        let param = param.trim();
        for prefix in &["filename*", "name*"] {
            if let Some(rest) = param.strip_prefix(prefix) {
                if let Some(eq_pos) = rest.find('=') {
                    let num_part = &rest[..eq_pos];
                    let val_part = &rest[eq_pos + 1..];
                    let num_str = num_part.trim_end_matches('*');
                    if let Ok(idx) = num_str.parse::<usize>() {
                        let val = val_part.trim().trim_matches('"').trim_matches('\'');
                        let decoded = if num_part.ends_with('*') {
                            decode_rfc2231_value(val)
                        } else {
                            val.to_string()
                        };
                        parts.push((idx, decoded));
                    }
                }
            }
        }
    }
    if parts.is_empty() {
        return None;
    }
    parts.sort_by_key(|(idx, _)| *idx);
    let mut result = String::new();
    for (_, v) in parts {
        if result.len().saturating_add(v.len()) > MAX_FILENAME_LEN {
            // Truncate rather than fail the whole parse — the partial filename
            // is still safer than `None` triggering downstream defaults that
            // might leak the original.
            let remaining = MAX_FILENAME_LEN.saturating_sub(result.len());
            if remaining > 0 {
                let take = v
                    .char_indices()
                    .take_while(|(i, _)| *i <= remaining)
                    .last()
                    .map(|(i, c)| i + c.len_utf8())
                    .unwrap_or(0);
                result.push_str(&v[..take.min(v.len())]);
            }
            break;
        }
        result.push_str(&v);
    }
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

fn extract_rfc2231_encoded_filename(value: &str) -> Option<String> {
    for param in value.split(';') {
        let param = param.trim();
        for prefix in &["filename*=", "name*="] {
            if let Some(rest) = param.strip_prefix(prefix) {
                let val = rest.trim().trim_matches('"');
                return Some(decode_rfc2231_value(val));
            }
        }
    }
    None
}

fn decode_rfc2231_value(value: &str) -> String {
    let parts: Vec<&str> = value.splitn(3, '\'').collect();
    if parts.len() == 3 {
        let charset = parts[0];
        let encoded = parts[2];
        let decoded_bytes = percent_decode_bytes(encoded);
        let encoding =
            encoding_rs::Encoding::for_label(charset.as_bytes()).unwrap_or(encoding_rs::UTF_8);
        let (result, _, _) = encoding.decode(&decoded_bytes);
        result.into_owned()
    } else {
        let decoded_bytes = percent_decode_bytes(value);
        String::from_utf8_lossy(&decoded_bytes).into_owned()
    }
}

fn percent_decode_bytes(input: &str) -> Vec<u8> {
    let mut result = Vec::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) =
                u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""), 16)
            {
                result.push(byte);
                i += 3;
                continue;
            }
        }
        result.push(bytes[i]);
        i += 1;
    }
    result
}

/// RFC 3676: Unwrap format=flowed text
/// Lines ending with a space (SP) are joined with the following line.
/// Lines beginning with "-- " are signature separators (never flowed).
pub fn unflow_text(text: &str) -> String {
    let mut result = String::with_capacity(text.len());

    for line in text.lines() {
        // Signature separator is never flowed
        if line == "-- " {
            result.push_str(line);
            result.push('\n');
            continue;
        }

        if line.ends_with(' ') {
            result.push_str(line.trim_end_matches(' '));
            result.push(' ');
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_type_parse() {
        let ct = ContentType::parse("text/plain; charset=utf-8");
        assert_eq!(ct.type_, "text");
        assert_eq!(ct.subtype, "plain");
        assert_eq!(ct.charset(), Some("utf-8"));
    }

    #[test]
    fn test_content_type_multipart() {
        let ct = ContentType::parse("multipart/mixed; boundary=\"----=_Part_123\"");
        assert!(ct.is_multipart());
        assert_eq!(ct.boundary(), Some("----=_Part_123"));
    }

    #[test]
    fn test_content_disposition() {
        let cd = ContentDisposition::parse("attachment; filename=\"test.pdf\"");
        assert!(cd.is_attachment());
        assert_eq!(cd.filename(), Some("test.pdf"));
    }

    #[test]
    fn test_content_disposition_inline() {
        let cd = ContentDisposition::parse("inline");
        assert!(!cd.is_attachment());
    }

    #[test]
    fn test_base64_decode() {
        let data = b"SGVsbG8gV29ybGQ=";
        let decoded = decode_base64(data).unwrap();
        assert_eq!(decoded, b"Hello World");
    }

    #[test]
    fn test_base64_decode_with_newlines() {
        let data = b"SGVs\nbG8g\nV29y\nbGQ=";
        let decoded = decode_base64(data).unwrap();
        assert_eq!(decoded, b"Hello World");
    }

    #[test]
    fn test_quoted_printable_decode() {
        let data = b"=48=C3=A5kan";
        let decoded = decode_quoted_printable(data);
        assert_eq!(std::str::from_utf8(&decoded).unwrap(), "Håkan");
    }

    #[test]
    fn test_quoted_printable_soft_break() {
        let data = b"line=\r\ncontinued";
        let decoded = decode_quoted_printable(data);
        assert_eq!(std::str::from_utf8(&decoded).unwrap(), "linecontinued");
    }

    #[test]
    fn test_quoted_printable_soft_break_unix_lf_only() {
        // Unix-style soft break: =\n without \r (common in Unix-originated emails)
        let data = b"line=\ncontinued";
        let decoded = decode_quoted_printable(data);
        assert_eq!(
            std::str::from_utf8(&decoded).unwrap(),
            "linecontinued",
            "=\\n soft break (without \\r) should be handled"
        );
    }

    #[test]
    fn test_quoted_printable_soft_break_mixed() {
        // Mix of Unix and DOS soft breaks
        let data = b"part1=\npart2=\r\npart3";
        let decoded = decode_quoted_printable(data);
        assert_eq!(std::str::from_utf8(&decoded).unwrap(), "part1part2part3");
    }

    #[test]
    fn test_uuencode_simple() {
        let data = b"begin 644 test.txt\n+5B5C(&%L9&%C\n`\nend\n";
        let decoded = decode_uuencode(data);
        assert!(decoded.is_some());
        assert!(!decoded.unwrap().is_empty());
    }

    #[test]
    fn test_parse_mime_info() {
        let headers = vec![
            ("content-type".to_string(), "text/plain; charset=iso-8859-1".to_string()),
            ("content-transfer-encoding".to_string(), "quoted-printable".to_string()),
        ];
        let mi = parse_mime_info(&headers).unwrap();
        assert_eq!(mi.content_type.charset(), Some("iso-8859-1"));
        assert_eq!(mi.content_transfer_encoding.as_deref(), Some("quoted-printable"));
    }

    #[test]
    fn test_parse_mime_info_no_cte() {
        let headers = vec![("content-type".to_string(), "text/plain; charset=utf-8".to_string())];
        let mi = parse_mime_info(&headers).unwrap();
        assert_eq!(mi.content_type.charset(), Some("utf-8"));
        assert!(mi.content_transfer_encoding.is_none());
    }

    #[test]
    fn test_parse_mime_info_no_ct() {
        let headers: Vec<(String, String)> = vec![("from".to_string(), "a@b.com".to_string())];
        assert!(parse_mime_info(&headers).is_none());
    }

    #[test]
    fn test_decode_body_base64() {
        let headers = vec![
            ("content-type".to_string(), "text/plain; charset=utf-8".to_string()),
            ("content-transfer-encoding".to_string(), "base64".to_string()),
        ];
        let mi = parse_mime_info(&headers).unwrap();
        let body = b"SGVsbG8gV29ybGQ=";
        let decoded = decode_body(body, &mi);
        assert_eq!(decoded, "Hello World");
    }

    #[test]
    fn test_decode_body_quoted_printable() {
        let headers = vec![
            ("content-type".to_string(), "text/plain; charset=utf-8".to_string()),
            ("content-transfer-encoding".to_string(), "quoted-printable".to_string()),
        ];
        let mi = parse_mime_info(&headers).unwrap();
        let body = b"Hello=20World=21";
        let decoded = decode_body(body, &mi);
        assert_eq!(decoded, "Hello World!");
    }

    #[test]
    fn test_decode_body_7bit_passthrough() {
        let headers = vec![("content-type".to_string(), "text/plain; charset=utf-8".to_string())];
        let mi = parse_mime_info(&headers).unwrap();
        let body = b"Hello World";
        let decoded = decode_body(body, &mi);
        assert_eq!(decoded, "Hello World");
    }

    #[test]
    fn test_decode_body_charset_iso8859_1() {
        let headers = vec![
            ("content-type".to_string(), "text/plain; charset=iso-8859-1".to_string()),
            ("content-transfer-encoding".to_string(), "quoted-printable".to_string()),
        ];
        let mi = parse_mime_info(&headers).unwrap();
        // "H=E5kan" with iso-8859-1: å = 0xE5 = 229
        let body = b"H=E5kan";
        let decoded = decode_body(body, &mi);
        assert_eq!(decoded, "Håkan");
    }

    #[test]
    fn test_process_mime_body_no_mime() {
        let headers = vec![("from".to_string(), "a@b.com".to_string())];
        let (body, charset) = process_mime_body(&headers, b"Hello World");
        assert_eq!(body, "Hello World");
        assert!(charset.is_none());
    }

    #[test]
    fn test_process_mime_body_with_charset() {
        let headers =
            vec![("content-type".to_string(), "text/plain; charset=iso-8859-1".to_string())];
        let (body, charset) = process_mime_body(&headers, b"Hello");
        assert_eq!(body, "Hello");
        assert_eq!(charset.as_deref(), Some("iso-8859-1"));
    }

    #[test]
    fn test_decode_body_iso_8859_7_kalimera() {
        let headers =
            vec![("content-type".to_string(), "text/plain; charset=iso-8859-7".to_string())];
        let mi = parse_mime_info(&headers).unwrap();
        // "Καλημερα" in ISO-8859-7: Κ=0xCA α=0xE1 λ=0xEB η=0xE7 μ=0xEC ε=0xE5 ρ=0xF1 α=0xE1
        let body = b"\xCA\xE1\xEB\xE7\xEC\xE5\xF1\xE1";
        let decoded = decode_body(body, &mi);
        assert_eq!(decoded, "Καλημερα");
    }

    #[test]
    fn test_decode_body_windows_1253_kalimera() {
        let headers =
            vec![("content-type".to_string(), "text/plain; charset=windows-1253".to_string())];
        let mi = parse_mime_info(&headers).unwrap();
        // "Καλημερα" in Windows-1253 (same code points for unaccented Greek)
        let body = b"\xCA\xE1\xEB\xE7\xEC\xE5\xF1\xE1";
        let decoded = decode_body(body, &mi);
        assert_eq!(decoded, "Καλημερα");
    }

    #[test]
    fn test_decode_body_iso_8859_7_tonos() {
        let headers =
            vec![("content-type".to_string(), "text/plain; charset=iso-8859-7".to_string())];
        let mi = parse_mime_info(&headers).unwrap();
        // "άνθρωπος" in ISO-8859-7: ά=0xDC ν=0xED θ=0xE8 ρ=0xF1 ω=0xF9 π=0xF0 ο=0xEF ς=0xF2
        let body = b"\xDC\xED\xE8\xF1\xF9\xF0\xEF\xF2";
        let decoded = decode_body(body, &mi);
        assert_eq!(decoded, "άνθρωπος");
    }

    #[test]
    fn test_decode_body_windows_1253_tonos() {
        let headers =
            vec![("content-type".to_string(), "text/plain; charset=windows-1253".to_string())];
        let mi = parse_mime_info(&headers).unwrap();
        // "άνθρωπος" in Windows-1253: ά=0xDC ν=0xED θ=0xE8 ρ=0xF1 ω=0xF9 π=0xF0 ο=0xEF ς=0xF2
        let body = b"\xDC\xED\xE8\xF1\xF9\xF0\xEF\xF2";
        let decoded = decode_body(body, &mi);
        assert_eq!(decoded, "άνθρωπος");
    }

    #[test]
    fn test_decode_body_no_charset_iso_8859_7_fallback() {
        let headers = vec![("content-type".to_string(), "text/plain".to_string())];
        let mi = parse_mime_info(&headers).unwrap();
        // "Καλημερα" in ISO-8859-7
        let body = b"\xCA\xE1\xEB\xE7\xEC\xE5\xF1\xE1";
        let decoded = decode_body(body, &mi);
        assert_eq!(decoded, "Καλημερα");
    }

    #[test]
    fn test_decode_body_iso_8859_7_quoted_printable() {
        let headers = vec![
            ("content-type".to_string(), "text/plain; charset=iso-8859-7".to_string()),
            ("content-transfer-encoding".to_string(), "quoted-printable".to_string()),
        ];
        let mi = parse_mime_info(&headers).unwrap();
        // QP-encoded "Καλημερα": Κ=CA α=E1 λ=EB η=E7 μ=EC ε=E5 ρ=F1 α=E1
        let body = b"\xCA=E1=EB=E7=EC=E5=F1=E1";
        let decoded = decode_body(body, &mi);
        assert_eq!(decoded, "Καλημερα");
    }

    #[test]
    fn test_decode_body_iso_8859_7_base64() {
        let headers = vec![
            ("content-type".to_string(), "text/plain; charset=iso-8859-7".to_string()),
            ("content-transfer-encoding".to_string(), "base64".to_string()),
        ];
        let mi = parse_mime_info(&headers).unwrap();
        // Base64 of ISO-8859-7 "Καλημερα" (bytes: CAE1EBE7ECE5F1E1)
        let body = b"yuHr5+zl8eE=";
        let decoded = decode_body(body, &mi);
        assert_eq!(decoded, "Καλημερα");
    }

    #[test]
    fn test_find_multipart_charset_second_part() {
        // Multipart where first part has no charset, second part does
        let boundary = "----=_NextPart_000_1234";
        let body = "------=_NextPart_000_1234\n\
             Content-Type: text/plain; format=flowed\n\
             \n\
             Some plain text\n\
             \n\
             ------=_NextPart_000_1234\n\
             Content-Type: text/html; charset=\"iso-8859-7\"\n\
             \n\
             <p>Some text</p>\n\
             \n\
             ------=_NextPart_000_1234--\n"
            .to_string();
        let result = find_multipart_charset(body.as_bytes(), boundary);
        assert_eq!(result.as_deref(), Some("iso-8859-7"));
    }

    #[test]
    fn test_find_multipart_charset_all_parts_no_charset() {
        // Multipart where NO part has a charset
        let boundary = "----=_NextPart_000_5678";
        let body = "------=_NextPart_000_5678\n\
             Content-Type: text/plain; format=flowed\n\
             \n\
             First part\n\
             \n\
             ------=_NextPart_000_5678\n\
             Content-Type: text/plain\n\
             \n\
             Second part\n\
             \n\
             ------=_NextPart_000_5678--\n"
            .to_string();
        let result = find_multipart_charset(body.as_bytes(), boundary);
        assert!(result.is_none());
    }

    #[test]
    fn test_process_mime_body_multipart_charset_in_second_part() {
        let headers = vec![(
            "content-type".to_string(),
            "multipart/mixed; boundary=\"----=_NextPart_000_9999\"".to_string(),
        )];
        let body = b"------=_NextPart_000_9999\n\
             Content-Type: text/plain; format=flowed\n\
             Content-Transfer-Encoding: 8bit\n\
             \n\
             Hello\n\
             \n\
             ------=_NextPart_000_9999\n\
             Content-Type: text/html; charset=\"iso-8859-7\"\n\
             Content-Transfer-Encoding: 8bit\n\
             \n\
             \xCB\xE1\xEC\xE7\xED\xE5\xF1\xE1\n\
             \n\
             ------=_NextPart_000_9999--\n";
        let charset = resolve_charset(body, &parse_mime_info(&headers).unwrap());
        assert_eq!(
            charset.as_deref(),
            Some("iso-8859-7"),
            "Should detect charset from second part when first part lacks it"
        );
    }

    #[test]
    fn test_decode_body_greek_utf8() {
        let headers = vec![("content-type".to_string(), "text/plain; charset=utf-8".to_string())];
        let mi = parse_mime_info(&headers).unwrap();
        let body = "Καλημερα".as_bytes();
        let decoded = decode_body(body, &mi);
        assert_eq!(decoded, "Καλημερα");
    }

    #[test]
    fn test_process_mime_body_no_ct_greek_fallback() {
        // No Content-Type header, but body has Greek ISO-8859-7 bytes
        let headers = vec![("from".to_string(), "a@b.com".to_string())];
        // "Γεια" in ISO-8859-7: Γ=0xC3 ε=0xE5 ι=0xE9 α=0xE1
        let body = b"\xC3\xE5\xE9\xE1";
        let (decoded, charset) = process_mime_body(&headers, body);
        assert!(!decoded.contains('\u{FFFD}'), "Should decode Greek without replacement chars");
        // Should have detected a charset from fallbacks
        assert!(charset.is_some(), "Should report a detected charset");
        assert_eq!(decoded, "Γεια");
    }

    // Additional comprehensive Greek charset tests

    #[test]
    fn test_decode_body_uppercase_tonos_iso_8859_7() {
        // Test uppercase Greek with tonos: "Άνθρωπος" (capital Ά)
        // ISO-8859-7: Ά=0xB6 ν=0xED θ=0xE8 ρ=0xF1 ω=0xF9 π=0xF0 ο=0xEF ς=0xF2
        let headers =
            vec![("content-type".to_string(), "text/plain; charset=iso-8859-7".to_string())];
        let mi = parse_mime_info(&headers).unwrap();
        let body = b"\xB6\xED\xE8\xF1\xF9\xF0\xEF\xF2";
        let decoded = decode_body(body, &mi);
        assert_eq!(decoded, "Άνθρωπος");
    }

    #[test]
    fn test_decode_body_uppercase_tonos_windows_1253() {
        // Test uppercase Greek with tonos: "Άνθρωπος" (capital Ά)
        // Windows-1253: Ά=0xA2 ν=0xED θ=0xE8 ρ=0xF1 ω=0xF9 π=0xF0 ο=0xEF ς=0xF2
        let headers =
            vec![("content-type".to_string(), "text/plain; charset=windows-1253".to_string())];
        let mi = parse_mime_info(&headers).unwrap();
        let body = b"\xA2\xED\xE8\xF1\xF9\xF0\xEF\xF2";
        let decoded = decode_body(body, &mi);
        assert_eq!(decoded, "Άνθρωπος");
    }

    #[test]
    fn test_decode_body_real_world_greek_phrase() {
        // Real-world phrase: "Καλό απόγευμα" (Good afternoon)
        // ISO-8859-7: Κ=0xCA α=0xE1 λ=0xEB ό=0xFC <space> α=0xE1 π=0xF0 ό=0xFC γ=0xE3 ε=0xE5 υ=0xF5 μ=0xEC α=0xE1
        let headers =
            vec![("content-type".to_string(), "text/plain; charset=iso-8859-7".to_string())];
        let mi = parse_mime_info(&headers).unwrap();
        let body = b"\xCA\xE1\xEB\xFC\x20\xE1\xF0\xFC\xE3\xE5\xF5\xEC\xE1";
        let decoded = decode_body(body, &mi);
        assert_eq!(decoded, "Καλό απόγευμα");
    }

    #[test]
    fn test_decode_body_mixed_greek_latin() {
        // Mixed text: "Hello Κόσμε!" (Hello World! in mixed Greek/Latin)
        // UTF-8 encoding for the Greek part
        let headers = vec![("content-type".to_string(), "text/plain; charset=utf-8".to_string())];
        let mi = parse_mime_info(&headers).unwrap();
        let body = "Hello Κόσμε!".as_bytes();
        let decoded = decode_body(body, &mi);
        assert_eq!(decoded, "Hello Κόσμε!");
    }

    #[test]
    fn test_decode_body_question_marks_greek() {
        // Greek semicolon (U+037E) looks like ";" and question mark is ";"
        // "Πώς είσαι;" (How are you?)
        // ISO-8859-7: Π=0xD0 ώ=0xFE ς=0xF2 <space> ε=0xE5 ί=0xDF σ=0xF3 α=0xE1 ι=0xE9 ;
        let headers =
            vec![("content-type".to_string(), "text/plain; charset=iso-8859-7".to_string())];
        let mi = parse_mime_info(&headers).unwrap();
        let body = b"\xD0\xFE\xF2\x20\xE5\xDF\xF3\xE1\xE9;";
        let decoded = decode_body(body, &mi);
        assert_eq!(decoded, "Πώς είσαι;");
    }

    #[test]
    fn test_find_multipart_charset_mixed_encodings() {
        // Multipart with first part in UTF-8 (no charset param), second in ISO-8859-7
        let boundary = "----=_Part_123";
        let body = "------=_Part_123\n\
             Content-Type: text/plain\n\
             \n\
             English text\n\
             \n\
             ------=_Part_123\n\
             Content-Type: text/html; charset=\"iso-8859-7\"\n\
             \n\
             <p>Greek text</p>\n\
             \n\
             ------=_Part_123--\n"
            .to_string();
        let result = find_multipart_charset(body.as_bytes(), boundary);
        assert_eq!(
            result.as_deref(),
            Some("iso-8859-7"),
            "Should find charset from second part even when first part has none"
        );
    }

    #[test]
    fn test_process_mime_body_multipart_with_greek_html() {
        // Real-world scenario: multipart/alternative with Greek HTML
        let headers = vec![(
            "content-type".to_string(),
            "multipart/alternative; boundary=\"----=_NextPart_000_1111\"".to_string(),
        )];
        let body = b"------=_NextPart_000_1111\n\
             Content-Type: text/plain; charset=\"iso-8859-7\"\n\
             \n\
             \xCA\xE1\xEB\xE7\xEC\xE5\xF1\xE1\n\
             \n\
             ------=_NextPart_000_1111\n\
             Content-Type: text/html; charset=\"iso-8859-7\"\n\
             \n\
             <html><body>\xCA\xE1\xEB\xE7\xEC\xE5\xF1\xE1</body></html>\n\
             \n\
             ------=_NextPart_000_1111--\n";
        let (decoded, charset) = process_mime_body(&headers, body);
        assert_eq!(charset.as_deref(), Some("iso-8859-7"));
        // Should decode Greek correctly from first text/plain part
        assert!(decoded.contains("Καλημερα"), "Should contain decoded Greek text");
        assert!(!decoded.contains('\u{FFFD}'), "Should not have replacement characters");
    }

    #[test]
    fn test_decode_body_all_greek_letters_iso_8859_7() {
        // Test basic Greek alphabet (lowercase): α β γ δ ε
        // ISO-8859-7: α=0xE1 β=0xE2 γ=0xE3 δ=0xE4 ε=0xE5
        let headers =
            vec![("content-type".to_string(), "text/plain; charset=iso-8859-7".to_string())];
        let mi = parse_mime_info(&headers).unwrap();
        let body = b"\xE1\xE2\xE3\xE4\xE5";
        let decoded = decode_body(body, &mi);
        assert_eq!(decoded, "αβγδε");
    }

    #[test]
    fn test_decode_body_all_greek_letters_windows_1253() {
        // Test basic Greek alphabet (uppercase): Α Β Γ Δ Ε
        // Windows-1253: Α=0xC1 Β=0xC2 Γ=0xC3 Δ=0xC4 Ε=0xC5
        let headers =
            vec![("content-type".to_string(), "text/plain; charset=windows-1253".to_string())];
        let mi = parse_mime_info(&headers).unwrap();
        let body = b"\xC1\xC2\xC3\xC4\xC5";
        let decoded = decode_body(body, &mi);
        assert_eq!(decoded, "ΑΒΓΔΕ");
    }

    #[test]
    fn test_decode_body_diaeresis_greek() {
        // Test Greek with diaeresis: "ϊδιος" (same, with diaeresis on iota)
        // ISO-8859-7: ϊ=0xFA δ=0xE4 ι=0xE9 ο=0xEF ς=0xF2
        let headers =
            vec![("content-type".to_string(), "text/plain; charset=iso-8859-7".to_string())];
        let mi = parse_mime_info(&headers).unwrap();
        let body = b"\xFA\xE4\xE9\xEF\xF2";
        let decoded = decode_body(body, &mi);
        assert_eq!(decoded, "ϊδιος");
    }

    #[test]
    fn test_multipart_inline_image() {
        // Test multipart message with inline image (Content-Disposition: inline)
        let headers = vec![(
            "content-type".to_string(),
            "multipart/mixed; boundary=\"----=_Part_123\"".to_string(),
        )];

        // Create a small 1x1 red pixel GIF
        let gif_bytes = b"R0lGODlhAQABAIAAAP8AAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7";

        let body = format!(
            "------=_Part_123\n\
             Content-Type: text/plain; charset=utf-8\n\
             \n\
             Hello world\n\
             \n\
             ------=_Part_123\n\
             Content-Type: image/gif; name=\"pixel.gif\"\n\
             Content-Disposition: inline; filename=\"pixel.gif\"\n\
             Content-Transfer-Encoding: base64\n\
             \n\
             {}\n\
             \n\
             ------=_Part_123--\n",
            std::str::from_utf8(gif_bytes).unwrap()
        );

        let (decoded, _charset) = process_mime_body(&headers, body.as_bytes());

        // Should contain the text part
        assert!(decoded.contains("Hello world"), "Should contain text content");

        // Should contain inline image marker (not escaped HTML)
        assert!(
            decoded.contains("[INLINE_IMAGE:image/gif:"),
            "Should contain inline image marker"
        );
        assert!(
            decoded.contains("R0lGODlhAQABAIAAAP8AAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7"),
            "Should contain base64 data in marker"
        );

        // Should NOT show MIME boundaries
        assert!(!decoded.contains("------=_Part_123"), "Should not contain MIME boundaries");
    }

    #[test]
    fn test_multipart_attachment_image() {
        // Images are always embedded inline in the HTML archive regardless of
        // Content-Disposition: attachment — browsers browse, they don't download.
        let headers = vec![(
            "content-type".to_string(),
            "multipart/mixed; boundary=\"----=_Part_456\"".to_string(),
        )];

        let gif_bytes = b"R0lGODlhAQABAIAAAP8AAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7";

        let body = format!(
            "------=_Part_456\n\
             Content-Type: text/plain; charset=utf-8\n\
             \n\
             See attached image\n\
             \n\
             ------=_Part_456\n\
             Content-Type: image/gif; name=\"chart.gif\"\n\
             Content-Disposition: attachment; filename=\"chart.gif\"\n\
             Content-Transfer-Encoding: base64\n\
             \n\
             {}\n\
             \n\
             ------=_Part_456--\n",
            std::str::from_utf8(gif_bytes).unwrap()
        );

        let (decoded, _charset) = process_mime_body(&headers, body.as_bytes());

        // Should contain the text part
        assert!(decoded.contains("See attached image"), "Should contain text content");

        // Image must be embedded inline regardless of Content-Disposition: attachment
        assert!(
            decoded.contains("[INLINE_IMAGE:image/gif:"),
            "Should embed image inline even when Content-Disposition is attachment"
        );
        assert!(
            !decoded.contains("[Attachment: chart.gif]"),
            "Should NOT show attachment notation for images"
        );
    }

    #[test]
    fn test_multipart_image_with_content_id() {
        // Test multipart message with image referenced by Content-ID (for HTML email)
        let headers = vec![(
            "content-type".to_string(),
            "multipart/related; boundary=\"----=_Part_789\"".to_string(),
        )];

        let gif_bytes = b"R0lGODlhAQABAIAAAP8AAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7";

        let body = format!(
            "------=_Part_789\n\
             Content-Type: text/html; charset=utf-8\n\
             \n\
             <html><body>Logo: <img src=\"cid:logo@example.com\"></body></html>\n\
             \n\
             ------=_Part_789\n\
             Content-Type: image/gif; name=\"logo.gif\"\n\
             Content-ID: <logo@example.com>\n\
             Content-Transfer-Encoding: base64\n\
             \n\
             {}\n\
             \n\
             ------=_Part_789--\n",
            std::str::from_utf8(gif_bytes).unwrap()
        );

        let (decoded, _charset) = process_mime_body(&headers, body.as_bytes());

        // Should contain HTML part
        assert!(decoded.contains("<html>"), "Should contain HTML content");

        // Should contain inline image marker (since it has Content-ID)
        assert!(
            decoded.contains("[INLINE_IMAGE:image/gif:"),
            "Should contain inline image marker for Content-ID image"
        );
    }

    #[test]
    fn test_multipart_pdf_attachment() {
        // Test multipart message with PDF attachment
        let headers = vec![(
            "content-type".to_string(),
            "multipart/mixed; boundary=\"----=_Part_PDF\"".to_string(),
        )];

        let pdf_bytes = b"JVBERi0xLjQKJeLjz9M="; // Minimal PDF header in base64

        let body = format!(
            "------=_Part_PDF\n\
             Content-Type: text/plain; charset=utf-8\n\
             \n\
             Please review the attached document.\n\
             \n\
             ------=_Part_PDF\n\
             Content-Type: application/pdf; name=\"report.pdf\"\n\
             Content-Disposition: attachment; filename=\"report.pdf\"\n\
             Content-Transfer-Encoding: base64\n\
             \n\
             {}\n\
             \n\
             ------=_Part_PDF--\n",
            std::str::from_utf8(pdf_bytes).unwrap()
        );

        let (decoded, _charset) = process_mime_body(&headers, body.as_bytes());

        // Should contain the text part
        assert!(decoded.contains("Please review"), "Should contain text content");

        // Should show attachment notation for PDF
        assert!(
            decoded.contains("[Attachment: report.pdf]"),
            "Should show PDF attachment notation"
        );

        // Should NOT show raw base64 PDF data
        assert!(!decoded.contains("JVBERi0xLjQKJeLjz9M="), "Should not contain raw PDF base64");
        assert!(!decoded.contains("application/pdf"), "Should not show content-type in output");
    }

    #[test]
    fn test_multipart_mixed_inline_and_attachment() {
        // Both inline and attachment-disposition images are now always embedded inline.
        let headers = vec![(
            "content-type".to_string(),
            "multipart/mixed; boundary=\"----=_Part_MIX\"".to_string(),
        )];

        let gif_bytes = b"R0lGODlhAQABAIAAAP8AAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7";

        let body = format!(
            "------=_Part_MIX\n\
             Content-Type: text/plain; charset=utf-8\n\
             \n\
             Email body text\n\
             \n\
             ------=_Part_MIX\n\
             Content-Type: image/gif; name=\"inline.gif\"\n\
             Content-Disposition: inline; filename=\"inline.gif\"\n\
             Content-Transfer-Encoding: base64\n\
             \n\
             {}\n\
             \n\
             ------=_Part_MIX\n\
             Content-Type: image/jpeg; name=\"photo.jpg\"\n\
             Content-Disposition: attachment; filename=\"photo.jpg\"\n\
             Content-Transfer-Encoding: base64\n\
             \n\
             {}\n\
             \n\
             ------=_Part_MIX--\n",
            std::str::from_utf8(gif_bytes).unwrap(),
            std::str::from_utf8(gif_bytes).unwrap()
        );

        let (decoded, _charset) = process_mime_body(&headers, body.as_bytes());

        // Should contain text
        assert!(decoded.contains("Email body text"), "Should contain text content");

        // Both images must be embedded inline — disposition is irrelevant for archives
        let inline_count = decoded.matches("[INLINE_IMAGE:").count();
        assert_eq!(inline_count, 2, "Both images (inline + attachment) should be embedded");

        assert!(!decoded.contains("[Attachment: "), "No image should remain as attachment link");
    }

    #[test]
    fn test_multipart_greek_text_with_inline_image() {
        // Real-world test: Greek text with inline image
        let headers = vec![(
            "content-type".to_string(),
            "multipart/mixed; boundary=\"----=_Part_GR\"".to_string(),
        )];

        // "Γεια σου" in ISO-8859-7: Γ=0xC3 ε=0xE5 ι=0xE9 α=0xE1 space σ=0xF3 ο=0xEF υ=0xF5
        let greek_text = vec![0xC3u8, 0xE5, 0xE9, 0xE1, 0x20, 0xF3, 0xEF, 0xF5];
        let gif_bytes = b"R0lGODlhAQABAIAAAP8AAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7";

        let mut body =
            b"------=_Part_GR\nContent-Type: text/plain; charset=iso-8859-7\n\n".to_vec();
        body.extend_from_slice(&greek_text);
        body.extend_from_slice(b"\n\n------=_Part_GR\n");
        body.extend_from_slice(b"Content-Type: image/gif; name=\"icon.gif\"\n");
        body.extend_from_slice(b"Content-Disposition: inline; filename=\"icon.gif\"\n");
        body.extend_from_slice(b"Content-Transfer-Encoding: base64\n\n");
        body.extend_from_slice(gif_bytes);
        body.extend_from_slice(b"\n\n------=_Part_GR--\n");

        let (decoded, charset) = process_mime_body(&headers, &body);

        // Should detect ISO-8859-7 charset
        assert_eq!(charset.as_deref(), Some("iso-8859-7"), "Should detect Greek charset");

        // Should contain decoded Greek text
        assert!(decoded.contains("Γεια σου"), "Should contain decoded Greek text");

        // Should contain inline image marker
        assert!(
            decoded.contains("[INLINE_IMAGE:image/gif:"),
            "Should contain inline image marker"
        );

        // Should NOT have mojibake or replacement characters
        assert!(!decoded.contains('\u{FFFD}'), "Should not have replacement characters");
    }

    #[test]
    fn test_decode_body_mislabeled_iso_8859_1_as_greek() {
        // Real-world case: Body labeled as iso-8859-1 but contains Greek (iso-8859-7)
        // Greek text: "Σωστά όλα αυτά" (Correct, all that)
        // In iso-8859-7: Σ=0xD3 ω=0xF9 σ=0xF3 τ=0xF4 ά=0xDC space=0x20 ό=0xFC λ=0xEB α=0xE1
        let headers =
            vec![("content-type".to_string(), "text/plain; charset=iso-8859-1".to_string())];
        let mi = parse_mime_info(&headers).unwrap();
        let body = b"\xD3\xF9\xF3\xF4\xDC\x20\xFC\xEB\xE1\x20\xE1\xF5\xF4\xDC";

        let decoded = decode_body(body, &mi);

        // Should auto-detect Greek despite iso-8859-1 label
        assert!(
            decoded.contains("Σωστά") || decoded.contains("ωστά"),
            "Should detect Greek in mislabeled iso-8859-1 body: got '{}'",
            decoded
        );

        // Should NOT have mojibake
        assert!(!decoded.contains("ÓùóôÜ"), "Should not have mojibake: got '{}'", decoded);
    }

    #[test]
    fn test_rfc2231_continuation_filename() {
        let headers = vec![(
            "content-disposition".to_string(),
            "attachment; filename*0=\"very_long_\"; filename*1=\"filename.pdf\"".to_string(),
        )];
        assert_eq!(extract_filename(&headers), Some("very_long_filename.pdf".to_string()));
    }

    #[test]
    fn test_rfc2231_encoded_filename() {
        let headers = vec![(
            "content-disposition".to_string(),
            "attachment; filename*=utf-8''%C3%A9tude.pdf".to_string(),
        )];
        assert_eq!(extract_filename(&headers), Some("étude.pdf".to_string()));
    }

    #[test]
    fn test_format_flowed_unwrap() {
        let input = "This is a long \nline that was wrapped.\n\nNew paragraph.\n";
        let expected = "This is a long line that was wrapped.\n\nNew paragraph.\n";
        assert_eq!(unflow_text(input), expected);
    }

    #[test]
    fn test_format_flowed_signature_not_unwrapped() {
        let input = "Hello \nworld.\n-- \nSignature\n";
        let expected = "Hello world.\n-- \nSignature\n";
        assert_eq!(unflow_text(input), expected);
    }

    #[test]
    fn test_decode_body_correct_iso_8859_1_latin_preserved() {
        // Verify that actual Latin-1 content is NOT incorrectly "fixed" to Greek
        // French: "Café résumé"
        let headers =
            vec![("content-type".to_string(), "text/plain; charset=iso-8859-1".to_string())];
        let mi = parse_mime_info(&headers).unwrap();
        let body = b"Caf\xE9 r\xE9sum\xE9";

        let decoded = decode_body(body, &mi);

        // Should preserve correct Latin-1
        assert_eq!(
            decoded, "Café résumé",
            "Should preserve correct Latin-1 text: got '{}'",
            decoded
        );
    }

    #[test]
    fn test_content_type_is_text() {
        let ct = ContentType::parse("text/html; charset=utf-8");
        assert!(ct.is_text());
        assert!(!ct.is_multipart());
    }

    #[test]
    fn test_content_type_full_type() {
        let ct = ContentType::parse("application/pdf");
        assert_eq!(ct.full_type(), "application/pdf");
    }

    #[test]
    fn test_content_type_name_param() {
        let ct = ContentType::parse("image/jpeg; name=\"photo.jpg\"");
        assert_eq!(ct.name(), Some("photo.jpg"));
    }

    #[test]
    fn test_content_type_no_subtype() {
        let ct = ContentType::parse("text");
        assert_eq!(ct.type_, "text");
        assert_eq!(ct.subtype, "");
    }

    #[test]
    fn test_content_disposition_no_filename() {
        let cd = ContentDisposition::parse("inline");
        assert_eq!(cd.filename(), None);
        assert!(!cd.is_attachment());
    }

    #[test]
    fn test_decode_quoted_printable_underscore_as_space() {
        let data = b"Hello_World";
        let decoded = decode_quoted_printable(data);
        assert_eq!(std::str::from_utf8(&decoded).unwrap(), "Hello World");
    }

    #[test]
    fn test_decode_uuencode_no_begin() {
        let data = b"not a uuencoded block";
        let result = decode_uuencode(data);
        assert!(result.is_none());
    }

    #[test]
    fn test_unflow_text_no_trailing_space() {
        let input = "Line one.\nLine two.\n";
        let result = unflow_text(input);
        assert_eq!(result, "Line one.\nLine two.\n");
    }

    #[test]
    fn test_process_mime_body_format_flowed() {
        let headers = vec![(
            "content-type".to_string(),
            "text/plain; charset=utf-8; format=flowed".to_string(),
        )];
        let body = b"This is a long \nline that flows.\n";
        let (decoded, _) = process_mime_body(&headers, body);
        assert!(decoded.contains("This is a long line that flows."), "got: {}", decoded);
    }
}
