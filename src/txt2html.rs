use crate::config::Config;
use crate::message::BodyChain;
use crate::quotes::{find_quote_class, is_sig_start, unquote};
use crate::string_utils::conv_urls;

pub fn txt2html(body_chain: &mut BodyChain, config: &Config) {
    let mut in_sig = false;
    for body in &mut body_chain.bodies {
        if body.attached || body.header || body.html {
            continue;
        }
        body.line = txt2html_line(&body.line, config, &mut in_sig);
    }
}

fn txt2html_line(line: &str, config: &Config, in_sig: &mut bool) -> String {
    let line = line.trim_end_matches('\n').trim_end_matches('\r');

    if line.is_empty() {
        // Emit a class-less marker that format_body() will collapse into a
        // bare newline within the surrounding run, producing a real
        // paragraph break inside one flowing pre-wrap block.
        return String::from("<div class=\"hm-blank\"></div>\n");
    }

    if is_sig_start(line) {
        *in_sig = true;
        return String::from("<hr class=\"hm-sig\">\n");
    }

    // Determine CSS class: monospace for signatures, proportional for body/quotes
    let pg_class = if *in_sig { "hm-sig-text" } else { "hm-pg" };

    // Check for inline image marker: [INLINE_IMAGE:mime/type:base64data]
    if line.starts_with("[INLINE_IMAGE:") && line.ends_with(']') {
        if let Some(marker_content) =
            line.strip_prefix("[INLINE_IMAGE:").and_then(|s| s.strip_suffix(']'))
        {
            if let Some((mime_type, base64_data)) = marker_content.split_once(':') {
                // SEC-1: Validate MIME type against allowlist before emitting data URI.
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
                if SAFE_IMAGE_TYPES.contains(&mime_type) {
                    // Convert marker to actual HTML img tag
                    return format!(
                        "<div class=\"{}\"><img src=\"data:{};base64,{}\" alt=\"Embedded image\" style=\"max-width:100%;height:auto\"></div>\n",
                        pg_class, mime_type, base64_data
                    );
                }
            }
        }
        // If parsing fails or MIME type not in allowlist, fall through to normal processing
    }

    let is_quote = line.starts_with('>');
    if is_quote {
        let unquoted = unquote(line);
        let quote_class = find_quote_class(line);
        let escaped = escape_html(&unquoted);
        let with_links = if config.href_detection {
            conv_urls(&escaped)
        } else {
            escaped
        };
        format!("<div class=\"{}\">{}</div>\n", quote_class, with_links)
    } else {
        let escaped = escape_html(line);
        let with_links = if config.href_detection {
            conv_urls(&escaped)
        } else {
            escaped
        };
        format!("<div class=\"{}\">{}</div>\n", pg_class, with_links)
    }
}

pub fn escape_html(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&#39;"),
            '\t' => result.push_str("        "),
            _ => result.push(c),
        }
    }
    result
}

pub fn conv_showhtml(body: &mut BodyChain, config: &Config) {
    let showhtml = config.showhtml;
    let mut in_sig = false;

    for b in &mut body.bodies {
        if b.attached || b.header {
            continue;
        }

        // Detect signature start
        let trimmed = b.line.trim_end_matches('\n').trim_end_matches('\r');
        if is_sig_start(trimmed) {
            in_sig = true;
            b.line = String::from("<hr class=\"hm-sig\">\n");
            continue;
        }

        let pg_class = if in_sig { "hm-sig-text" } else { "hm-pg" };

        if b.html {
            if showhtml >= 2 {
                continue;
            }
            if showhtml == 0 {
                let escaped = escape_html(&b.line);
                b.line = escaped;
                continue;
            }
            // showhtml == 1: escape HTML body text
            if showhtml == 1 || showhtml >= 4 {
                let escaped = escape_html(&b.line);
                b.line = if escaped.is_empty() {
                    String::new()
                } else {
                    format!("<div class=\"{}\">{}</div>\n", pg_class, escaped)
                };
                continue;
            }
            continue;
        }
        // !b.html (plain text body)
        if showhtml == 0 {
            let escaped = escape_html(&b.line);
            b.line = escaped;
            continue;
        }
        if showhtml == 1 {
            // Check for inline image marker first
            let line = b.line.trim_end_matches('\n').trim_end_matches('\r');
            if line.starts_with("[INLINE_IMAGE:") && line.ends_with(']') {
                if let Some(marker_content) =
                    line.strip_prefix("[INLINE_IMAGE:").and_then(|s| s.strip_suffix(']'))
                {
                    if let Some((mime_type, base64_data)) = marker_content.split_once(':') {
                        const SAFE_IMAGE_TYPES: &[&str] = &[
                            "image/gif",
                            "image/jpeg",
                            "image/jpg",
                            "image/png",
                            "image/webp",
                            "image/bmp",
                            "image/tiff",
                        ];
                        if SAFE_IMAGE_TYPES.contains(&mime_type) {
                            b.line = format!(
                                "<div class=\"{}\"><img src=\"data:{};base64,{}\" alt=\"Embedded image\" style=\"max-width:100%;height:auto\"></div>\n",
                                pg_class, mime_type, base64_data
                            );
                            continue;
                        }
                    }
                }
            }

            // Normal text processing
            let escaped = escape_html(&b.line);
            b.line = if escaped.is_empty() {
                String::from("<div class=\"hm-blank\"></div>\n")
            } else {
                format!("<div class=\"{}\">{}</div>\n", pg_class, escaped)
            };
            continue;
        }
        if showhtml == 2 || showhtml == 3 {
            b.line = txt2html_line(&b.line, config, &mut in_sig);
            continue;
        }
    }
}

pub fn conv_body_line(line: &str, config: &Config) -> String {
    if is_sig_start(line) {
        return String::from("<hr class=\"hm-sig\">\n");
    }

    let is_quote = line.starts_with('>');
    let escaped = escape_html(line);
    let with_links = if config.href_detection {
        conv_urls(&escaped)
    } else {
        escaped
    };

    if is_quote {
        format!("<div class=\"{}\">{}</div>\n", find_quote_class_with_fallback(line), with_links)
    } else {
        format!("<div class=\"hm-pg\">{}</div>\n", with_links)
    }
}

fn find_quote_class_with_fallback(line: &str) -> String {
    let class = find_quote_class(line);
    if class.is_empty() {
        "hm-quote-1".to_string()
    } else {
        class
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn make_config() -> Config {
        Config::default()
    }

    #[test]
    fn test_escape_html() {
        assert_eq!(escape_html("<test>"), "&lt;test&gt;");
        assert_eq!(escape_html("a&b"), "a&amp;b");
        assert_eq!(escape_html("hello"), "hello");
    }

    #[test]
    fn test_txt2html_line_normal() {
        let config = make_config();
        let mut in_sig = false;
        let result = txt2html_line("hello world", &config, &mut in_sig);
        assert_eq!(result, "<div class=\"hm-pg\">hello world</div>\n");
    }

    #[test]
    fn test_txt2html_line_quote() {
        let config = make_config();
        let mut in_sig = false;
        let result = txt2html_line("> quoted text", &config, &mut in_sig);
        assert!(result.contains("hm-quote-1"));
        assert!(result.contains("quoted text"));
    }

    #[test]
    fn test_txt2html_line_sig() {
        let config = make_config();
        let mut in_sig = false;
        let result = txt2html_line("-- ", &config, &mut in_sig);
        assert_eq!(result, "<hr class=\"hm-sig\">\n");
        assert!(in_sig);
    }

    #[test]
    fn test_txt2html_line_empty() {
        let config = make_config();
        let mut in_sig = false;
        let result = txt2html_line("", &config, &mut in_sig);
        assert_eq!(result, "<div class=\"hm-blank\"></div>\n");
    }

    #[test]
    fn test_txt2html_line_urls() {
        let config = make_config();
        let mut in_sig = false;
        let result = txt2html_line("Visit https://example.com", &config, &mut in_sig);
        assert!(result.contains("<a href=\"https://example.com\""));
        assert!(result.contains("rel=\"noopener noreferrer\""));
    }

    fn make_body_chain(text: &str) -> BodyChain {
        let mut chain = BodyChain { bodies: Vec::new() };
        chain.bodies.push(crate::message::Body {
            line: text.to_string(),
            html: false,
            header: false,
            parsed_header: false,
            attached: false,
            demimed: false,
            msgnum: 1,
        });
        chain
    }

    #[test]
    fn test_conv_showhtml_showhtml_0_escapes() {
        let mut config = make_config();
        config.showhtml = 0;
        let mut chain = make_body_chain("<script>alert('xss')</script>");
        conv_showhtml(&mut chain, &config);
        assert_eq!(chain.bodies[0].line, "&lt;script&gt;alert(&#39;xss&#39;)&lt;/script&gt;");
    }

    #[test]
    fn test_conv_showhtml_showhtml_0_plain_text() {
        let mut config = make_config();
        config.showhtml = 0;
        let mut chain = make_body_chain("Hello World");
        conv_showhtml(&mut chain, &config);
        assert_eq!(chain.bodies[0].line, "Hello World");
    }

    #[test]
    fn test_conv_showhtml_showhtml_1_wraps_in_div() {
        let mut config = make_config();
        config.showhtml = 1;
        let mut chain = make_body_chain("Hello World");
        conv_showhtml(&mut chain, &config);
        assert_eq!(chain.bodies[0].line, "<div class=\"hm-pg\">Hello World</div>\n");
    }

    #[test]
    fn test_conv_showhtml_showhtml_1_escapes_xss() {
        let mut config = make_config();
        config.showhtml = 1;
        let mut chain = make_body_chain("<script>bad</script>");
        conv_showhtml(&mut chain, &config);
        assert_eq!(
            chain.bodies[0].line,
            "<div class=\"hm-pg\">&lt;script&gt;bad&lt;/script&gt;</div>\n"
        );
    }

    #[test]
    fn test_conv_showhtml_showhtml_2_txt2html() {
        let mut config = make_config();
        config.showhtml = 2;
        let mut chain = make_body_chain("> quote");
        conv_showhtml(&mut chain, &config);
        assert!(chain.bodies[0].line.contains("hm-quote"));
    }

    #[test]
    fn test_conv_showhtml_attached_unchanged() {
        let mut config = make_config();
        config.showhtml = 0;
        let mut chain = BodyChain { bodies: Vec::new() };
        chain.bodies.push(crate::message::Body {
            line: "<script>attack</script>".to_string(),
            html: false,
            header: false,
            parsed_header: false,
            attached: true,
            demimed: false,
            msgnum: 1,
        });
        conv_showhtml(&mut chain, &config);
        // Attached bodies should not be processed
        assert_eq!(chain.bodies[0].line, "<script>attack</script>");
    }

    #[test]
    fn test_conv_showhtml_header_unchanged() {
        let mut config = make_config();
        config.showhtml = 0;
        let mut chain = BodyChain { bodies: Vec::new() };
        chain.bodies.push(crate::message::Body {
            line: "<script>attack</script>".to_string(),
            html: false,
            header: true,
            parsed_header: false,
            attached: false,
            demimed: false,
            msgnum: 1,
        });
        conv_showhtml(&mut chain, &config);
        assert_eq!(chain.bodies[0].line, "<script>attack</script>");
    }

    #[test]
    fn test_txt2html_line_inline_image() {
        let config = make_config();
        let mut in_sig = false;
        let line =
            "[INLINE_IMAGE:image/gif:R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7]";
        let result = txt2html_line(line, &config, &mut in_sig);

        // Should convert to actual HTML img tag
        assert!(
            result.contains("<img src=\"data:image/gif;base64,"),
            "Should convert marker to img tag"
        );
        assert!(
            result.contains("R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7"),
            "Should contain base64 data"
        );
        assert!(result.contains("alt=\"Embedded image\""), "Should have alt text");
        assert!(!result.contains("[INLINE_IMAGE:"), "Should not contain marker text");
    }

    #[test]
    fn test_txt2html_line_inline_image_jpeg() {
        let config = make_config();
        let mut in_sig = false;
        let line = "[INLINE_IMAGE:image/jpeg:abcd1234]";
        let result = txt2html_line(line, &config, &mut in_sig);

        assert!(
            result.contains("<img src=\"data:image/jpeg;base64,abcd1234\""),
            "Should create data URI with correct MIME type"
        );
    }

    #[test]
    fn test_txt2html_sig_then_body_uses_sig_class() {
        let config = make_config();
        let mut in_sig = false;
        let _ = txt2html_line("-- ", &config, &mut in_sig);
        assert!(in_sig);
        let result = txt2html_line("John Doe", &config, &mut in_sig);
        assert_eq!(result, "<div class=\"hm-sig-text\">John Doe</div>\n");
    }

    #[test]
    fn test_conv_body_line_plain() {
        let config = make_config();
        let result = conv_body_line("Hello world", &config);
        assert_eq!(result, "<div class=\"hm-pg\">Hello world</div>\n");
    }

    #[test]
    fn test_conv_body_line_quote() {
        let config = make_config();
        let result = conv_body_line("> quoted", &config);
        assert!(result.contains("hm-quote-1"));
        assert!(result.contains("quoted"));
    }

    #[test]
    fn test_conv_body_line_sig() {
        let config = make_config();
        let result = conv_body_line("-- ", &config);
        assert_eq!(result, "<hr class=\"hm-sig\">\n");
    }

    #[test]
    fn test_conv_body_line_escapes_html() {
        let config = make_config();
        let result = conv_body_line("<b>bold</b>", &config);
        assert!(result.contains("&lt;b&gt;"));
        assert!(!result.contains("<b>"));
    }

    #[test]
    fn test_escape_html_quote() {
        assert_eq!(escape_html("it's"), "it&#39;s");
    }

    #[test]
    fn test_escape_html_double_quote() {
        assert_eq!(escape_html(r#"say "hi""#), "say &quot;hi&quot;");
    }

    #[test]
    fn test_escape_html_tab_expanded() {
        let result = escape_html("a\tb");
        assert!(result.contains("        ")); // 8 spaces
        assert!(!result.contains('\t'));
    }

    #[test]
    fn test_inline_image_svg_blocked() {
        let config = make_config();
        let mut in_sig = false;
        let line = "[INLINE_IMAGE:image/svg+xml:PHN2Zy8+]";
        let result = txt2html_line(line, &config, &mut in_sig);
        // SVG is not in the allowlist; should NOT produce an <img> tag
        assert!(!result.contains("<img"), "SVG should be blocked from inline embedding");
    }
}
