use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;

use crate::config::Config;
use crate::date::get_date_str;
use crate::error::Result;
use crate::file_utils::msg_subdir;
use crate::headers::decode_mime_words;
use crate::html::format_subject_for_index;
use crate::i18n::I18n;
use crate::message::{EmailInfo, IndexType};
use crate::structs::EmailStore;
use crate::templates::{
    default_footer_template, default_header_template, get_header_cookies, set_cookie,
    substitute_cookies, substitute_printfile, PrintfileData,
};
use crate::txt2html::escape_html;
use chrono::TimeZone;

/// Generates the date-sorted index page HTML.
pub fn print_date_index(store: &EmailStore, config: &Config) -> Result<String> {
    let i18n = I18n::new(&config.language);
    let title = config.label.as_deref().unwrap_or(i18n.get("Date Index"));
    let mut cookies = get_header_cookies(config, title);

    let indices = store.traverse_date_list();
    let list = if config.reverse {
        let rev: Vec<usize> = indices.iter().rev().cloned().collect();
        render_flat_index(&rev, store, config, IndexType::Date)
    } else {
        render_flat_index(&indices, store, config, IndexType::Date)
    };
    let top = render_archive_stats_top(store, config, IndexType::Date, &i18n);
    let bottom = render_archive_stats_bottom(store, config, IndexType::Date, &i18n);
    set_cookie(&mut cookies, "ARTICLE", &format!("{}{}{}", top, list, bottom));

    render_index_page(config, &cookies)
}

/// Generates the subject-sorted index page HTML.
pub fn print_subject_index(store: &EmailStore, config: &Config) -> Result<String> {
    let i18n = I18n::new(&config.language);
    let title = config.label.as_deref().unwrap_or(i18n.get("Subject Index"));
    let mut cookies = get_header_cookies(config, title);

    let indices = store.traverse_subject_list();
    let list = if config.reverse {
        let rev: Vec<usize> = indices.iter().rev().cloned().collect();
        render_flat_index(&rev, store, config, IndexType::Subject)
    } else {
        render_flat_index(&indices, store, config, IndexType::Subject)
    };
    let top = render_archive_stats_top(store, config, IndexType::Subject, &i18n);
    let bottom = render_archive_stats_bottom(store, config, IndexType::Subject, &i18n);
    set_cookie(&mut cookies, "ARTICLE", &format!("{}{}{}", top, list, bottom));

    render_index_page(config, &cookies)
}

/// Generates the author-sorted index page HTML.
pub fn print_author_index(store: &EmailStore, config: &Config) -> Result<String> {
    let i18n = I18n::new(&config.language);
    let title = config.label.as_deref().unwrap_or(i18n.get("Author Index"));
    let mut cookies = get_header_cookies(config, title);

    let indices = store.traverse_author_list();
    let list = if config.reverse {
        let rev: Vec<usize> = indices.iter().rev().cloned().collect();
        render_flat_index(&rev, store, config, IndexType::Author)
    } else {
        render_flat_index(&indices, store, config, IndexType::Author)
    };
    let top = render_archive_stats_top(store, config, IndexType::Author, &i18n);
    let bottom = render_archive_stats_bottom(store, config, IndexType::Author, &i18n);
    set_cookie(&mut cookies, "ARTICLE", &format!("{}{}{}", top, list, bottom));

    render_index_page(config, &cookies)
}

/// Generates the threaded discussion index page HTML.
pub fn print_thread_index(store: &EmailStore, config: &Config) -> Result<String> {
    let i18n = I18n::new(&config.language);
    let title = config.label.as_deref().unwrap_or(i18n.get("Thread Index"));
    let mut cookies = get_header_cookies(config, title);

    let list = render_thread_index(store, config);
    let top = render_archive_stats_top(store, config, IndexType::Thread, &i18n);
    let bottom = render_archive_stats_bottom(store, config, IndexType::Thread, &i18n);
    set_cookie(&mut cookies, "ARTICLE", &format!("{}{}{}", top, list, bottom));

    render_index_page(config, &cookies)
}

/// Returns (first_date, last_date, count) across all emails in the store.
fn archive_date_range(store: &EmailStore) -> (i64, i64, usize) {
    let count = store.emails.len();
    if count == 0 {
        return (0, 0, 0);
    }
    let first = store.emails.iter().map(|e| e.date).min().unwrap_or(0);
    let last = store.emails.iter().map(|e| e.date).max().unwrap_or(0);
    (first, last, count)
}

/// Renders the top summary block shown above the message list:
///   N messages sorted by: [author][date][subject][attachment]
///   Starting: <date>  Ending: <date>
///   About this archive  (if config.about is set)
/// Returns the href for a given index type, using `index.{suffix}` for whichever
/// type is the configured `defaultindex` and the canonical named file otherwise.
fn index_href(index_type: IndexType, config: &Config) -> String {
    let sfx = &config.htmlsuffix;
    let is_default = match index_type {
        IndexType::Date => config.defaultindex == "date",
        IndexType::Subject => config.defaultindex == "subject",
        IndexType::Author => config.defaultindex == "author",
        IndexType::Thread => config.defaultindex == "thread",
        IndexType::Attachment => config.defaultindex == "attachment",
        _ => false,
    };
    if is_default {
        return format!("index.{}", sfx);
    }
    match index_type {
        IndexType::Date => format!("date.{}", sfx),
        IndexType::Subject => format!("subject.{}", sfx),
        IndexType::Author => format!("author.{}", sfx),
        IndexType::Thread => format!("thread.{}", sfx),
        IndexType::Attachment => format!("attachment.{}", sfx),
        _ => format!("index.{}", sfx),
    }
}

/// Builds the nav link string: `[ Author ] [ Date ] [ Subject ] [ Thread ] [ Attachment ]`
/// The current page type is shown as plain text; others are hyperlinks using `index_href`.
fn render_nav_links(
    store: &EmailStore,
    config: &Config,
    current: IndexType,
    i18n: &I18n,
) -> String {
    let mk_link = |t: IndexType, label: &str| -> String {
        if current == t {
            format!("[ {} ]", label)
        } else {
            format!("[ <a href=\"{}\">{}</a> ]", index_href(t, config), label)
        }
    };
    let author = mk_link(IndexType::Author, i18n.get("Author Index"));
    let date = mk_link(IndexType::Date, i18n.get("Date Index"));
    let subject = mk_link(IndexType::Subject, i18n.get("Subject Index"));
    let thread = mk_link(IndexType::Thread, i18n.get("Thread Index"));

    let has_attachments = store.emails.iter().any(|e| e.bodylist.bodies.iter().any(|b| b.attached));
    if config.attachmentsindex && has_attachments {
        let label = i18n.get("Attachment").trim_end_matches(':');
        let attach = mk_link(IndexType::Attachment, label);
        format!(
            "<nav aria-label=\"Index navigation\">{} {} {} {} {}</nav>",
            author, date, subject, thread, attach
        )
    } else {
        format!(
            "<nav aria-label=\"Index navigation\">{} {} {} {}</nav>",
            author, date, subject, thread
        )
    }
}

fn render_archive_stats_top(
    store: &EmailStore,
    config: &Config,
    current: IndexType,
    i18n: &I18n,
) -> String {
    let (first, last, count) = archive_date_range(store);
    if count == 0 {
        return String::new();
    }

    let first_str = get_date_str(
        first,
        config.dateformat.as_deref(),
        config.gmtime,
        config.eurodate,
        config.isodate,
        &config.language,
    );
    let last_str = get_date_str(
        last,
        config.dateformat.as_deref(),
        config.gmtime,
        config.eurodate,
        config.isodate,
        &config.language,
    );

    let mut nav = format!("{} {} ", count, i18n.get("messages sorted by"));
    nav.push_str(&render_nav_links(store, config, current, i18n));

    let mut html = format!(
        "<div class=\"hm-archive-info\">\n\
         <p>{}</p>\n\
         <p>{} <em>{}</em><br>{} <em>{}</em></p>\n",
        nav,
        i18n.get("Starting"),
        escape_html(&first_str),
        i18n.get("Ending"),
        escape_html(&last_str),
    );

    if let Some(ref about) = config.about {
        html.push_str(&format!(
            "<p><a href=\"{}\">{}</a></p>\n",
            escape_html(about),
            i18n.get("About this archive")
        ));
    }

    html.push_str("</div>\n");
    html
}

/// Renders the bottom summary block shown below the message list:
///   Last message date: <date>
///   Archived on: <now>
///   N messages sorted by: [author][date][subject][attachment]
///   About this archive  (if config.about is set)
fn render_archive_stats_bottom(
    store: &EmailStore,
    config: &Config,
    current: IndexType,
    i18n: &I18n,
) -> String {
    let (_, last, count) = archive_date_range(store);
    if count == 0 {
        return String::new();
    }

    let last_str = get_date_str(
        last,
        config.dateformat.as_deref(),
        config.gmtime,
        config.eurodate,
        config.isodate,
        &config.language,
    );

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let now_str = get_date_str(
        now,
        config.dateformat.as_deref(),
        config.gmtime,
        config.eurodate,
        config.isodate,
        &config.language,
    );

    let mut nav = format!("{} {} ", count, i18n.get("messages sorted by"));
    nav.push_str(&render_nav_links(store, config, current, i18n));

    let mut html = format!(
        "<div class=\"hm-archive-info\">\n\
         <p>{} <em>{}</em></p>\n\
         <p>{} <em>{}</em></p>\n\
         <p>{}</p>\n",
        i18n.get("Last message date"),
        escape_html(&last_str),
        i18n.get("Archived on"),
        escape_html(&now_str),
        nav,
    );

    if let Some(ref about) = config.about {
        html.push_str(&format!(
            "<p><a href=\"{}\">{}</a></p>\n",
            escape_html(about),
            i18n.get("About this archive")
        ));
    }

    html.push_str("</div>\n");
    html
}

fn render_flat_index(
    indices: &[usize],
    store: &EmailStore,
    config: &Config,
    index_type: IndexType,
) -> String {
    let i18n = I18n::new(&config.language);
    let mut html = String::new();

    if config.indextable {
        html.push_str("<table class=\"hm-index\">\n<tbody>\n");
        for &idx in indices {
            let email = &store.emails[idx];
            html.push_str(&format!(
                "<tr>{}</tr>\n",
                render_index_row(email, config, index_type, &i18n)
            ));
        }
        html.push_str("</tbody>\n</table>\n");
    } else {
        html.push_str("<ul class=\"hm-index\">\n");
        for &idx in indices {
            let email = &store.emails[idx];
            html.push_str(&format!(
                "  <li>{}</li>\n",
                render_index_row(email, config, index_type, &i18n)
            ));
        }
        html.push_str("</ul>\n");
    }

    html
}

fn render_index_row(
    email: &EmailInfo,
    config: &Config,
    index_type: IndexType,
    i18n: &I18n,
) -> String {
    let decoded_author = {
        let raw = email
            .name
            .as_deref()
            .or(email.email_addr.as_deref())
            .unwrap_or(i18n.get("unknown author"));
        decode_mime_words(raw)
    };
    match index_type {
        IndexType::Date => {
            let date_str = get_date_str(
                email.date,
                config.dateformat.as_deref(),
                config.gmtime,
                config.eurodate,
                config.isodate,
                &config.language,
            );
            let subject = email.subject.as_deref().unwrap_or(i18n.get("no subject"));
            let decoded_subject = format_subject_for_index(subject, config);
            let filename = crate::file_utils::message_url_str(email, config);
            format!(
                "<a href=\"{}\"><strong>{}</strong></a> <span id=\"msg{}\"><em>{} <small>({})</small></em></span>",
                filename,
                escape_html(&decoded_subject),
                email.msgnum,
                escape_html(&decoded_author),
                date_str,
            )
        },
        IndexType::Subject => {
            let subject = email.subject.as_deref().unwrap_or(i18n.get("no subject"));
            let decoded = format_subject_for_index(subject, config);
            let filename = crate::file_utils::message_url_str(email, config);
            let date_str = get_date_str(
                email.date,
                config.dateformat.as_deref(),
                config.gmtime,
                config.eurodate,
                config.isodate,
                &config.language,
            );
            format!(
                "<a href=\"{}\"><strong>{}</strong></a> <span id=\"msg{}\"><em>{} <small>({})</small></em></span>",
                filename,
                escape_html(&decoded),
                email.msgnum,
                escape_html(&decoded_author),
                date_str,
            )
        },
        IndexType::Author => {
            let subject = email.subject.as_deref().unwrap_or(i18n.get("no subject"));
            let decoded = format_subject_for_index(subject, config);
            let filename = crate::file_utils::message_url_str(email, config);
            let date_str = get_date_str(
                email.date,
                config.dateformat.as_deref(),
                config.gmtime,
                config.eurodate,
                config.isodate,
                &config.language,
            );
            format!(
                "<em>{}</em> <a href=\"{}\"><strong>{}</strong></a> <span id=\"msg{}\"><small>({})</small></span>",
                escape_html(&decoded_author),
                filename,
                escape_html(&decoded),
                email.msgnum,
                date_str,
            )
        },
        IndexType::Thread => String::new(),
        IndexType::Attachment | IndexType::Folders | IndexType::NoIndex => String::new(),
    }
}

fn render_thread_index(store: &EmailStore, config: &Config) -> String {
    let i18n = I18n::new(&config.language);
    let mut html = String::new();
    let mut printed: HashSet<i32> = HashSet::new();

    // Walk ALL messages in date order (matching original hypermail threadlist behaviour).
    // For each message not yet printed, start a thread tree.  This ensures that
    // standalone messages (no parent, no replies) appear as single top-level items,
    // and that reply trees are only rendered once — rooted at their earliest ancestor.
    let indices = store.traverse_date_list();
    let ordered: Box<dyn Iterator<Item = &usize>> = if config.reverse {
        Box::new(indices.iter().rev())
    } else {
        Box::new(indices.iter())
    };

    for &idx in ordered {
        let email = &store.emails[idx];
        if printed.contains(&email.msgnum) {
            continue;
        }
        // Only start a tree from root messages (no parent in the archive).
        let has_parent =
            store.replylist.iter().any(|r| r.msgnum == email.msgnum && r.from_msgnum >= 0);
        if !has_parent {
            render_thread_tree(email, store, config, &mut html, &mut printed, 0, &i18n);
        }
    }

    // Catch any messages that were somehow missed (shouldn't normally happen).
    for &idx in &indices {
        let email = &store.emails[idx];
        if !printed.contains(&email.msgnum) {
            render_thread_tree(email, store, config, &mut html, &mut printed, 0, &i18n);
        }
    }

    if html.is_empty() {
        return format!("<p>{}</p>\n", i18n.get("No messages found."));
    }

    format!("<ul class=\"hm-index\">\n{}</ul>\n", html)
}

fn render_thread_tree(
    email: &EmailInfo,
    store: &EmailStore,
    config: &Config,
    html: &mut String,
    printed: &mut HashSet<i32>,
    depth: i32,
    i18n: &I18n,
) {
    if printed.contains(&email.msgnum) {
        return;
    }
    printed.insert(email.msgnum);

    let max_depth = if config.thrdlevels > 0 && config.thrdlevels < 100 {
        config.thrdlevels
    } else {
        50
    };

    if depth > max_depth {
        return;
    }

    let subject = format_subject_for_index(
        email.subject.as_deref().unwrap_or(i18n.get("no subject")),
        config,
    );
    let author = decode_mime_words(
        email
            .name
            .as_deref()
            .or(email.email_addr.as_deref())
            .unwrap_or(i18n.get("unknown author")),
    );
    let filename = crate::file_utils::message_url_str(email, config);

    // Date string: use indexdateformat if set (non-empty), else dateformat — matching
    // original hypermail's getindexdatestr().
    let date_fmt = config
        .indexdateformat
        .as_deref()
        .filter(|s| !s.is_empty())
        .or(config.dateformat.as_deref());
    let date_str = get_date_str(
        email.date,
        date_fmt,
        config.gmtime,
        config.eurodate,
        config.isodate,
        &config.language,
    );

    // Format matches flat index style: bold subject, em author, small date
    let entry = format!(
        "<a href=\"{}\"><strong>{}</strong></a> <span id=\"msg{}\"><em>{} <small>({})</small></em></span>",
        filename,
        escape_html(&subject),
        email.msgnum,
        escape_html(&author),
        date_str,
    );

    if depth == 0 {
        html.push_str(&format!("  <li>{}", entry));
    } else {
        html.push_str(&format!("<li>{}", entry));
    }

    // Find all direct replies to this message.
    let replies: Vec<_> = store
        .replylist
        .iter()
        .filter(|r| r.from_msgnum == email.msgnum)
        .filter_map(|r| store.find_by_msgnum(r.msgnum).map(|idx| &store.emails[idx]))
        .collect();

    if !replies.is_empty() {
        html.push_str("\n<ul class=\"hm-thread-children\">\n");
        for reply_email in replies {
            render_thread_tree(reply_email, store, config, html, printed, depth + 1, i18n);
        }
        html.push_str("</ul>\n");
    }

    html.push_str("</li>\n");
}

fn render_index_page(
    config: &Config,
    cookies: &std::collections::HashMap<String, String>,
) -> Result<String> {
    use crate::templates::default_article_template;

    if config.ihtmlheader.is_some() || config.ihtmlfooter.is_some() {
        // External templates: use printfile-style %x substitution.
        // The ARTICLE body content sits between the header and footer.
        let header_tpl = config
            .ihtmlheader
            .as_deref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .unwrap_or_default();
        let footer_tpl = config
            .ihtmlfooter
            .as_deref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .unwrap_or_default();

        let title = cookies.get("TITLE").map(|s| s.as_str()).unwrap_or("");
        let data = PrintfileData {
            label: config.label.as_deref().unwrap_or(""),
            subject: title,
            dir: config.dir.as_deref().unwrap_or("."),
            name: None,
            email: None,
            msgid: None,
            charset: None,
            date: None,
            display_date: None,
            filename: None,
            archives: config.archives.as_deref(),
            about: config.about.as_deref(),
            mailto: config.mailto.as_deref(),
            language: &config.language,
            rel_path_to_top: "",
        };

        let article_body = cookies.get("ARTICLE").map(|s| s.as_str()).unwrap_or("");
        let header_html = substitute_printfile(&header_tpl, &data);
        let footer_html = substitute_printfile(&footer_tpl, &data);
        let generator = if config.showgenerator {
            let i18n = I18n::new(&config.language);
            let gen_text = crate::txt2html::escape_html(i18n.get("Generated by"));
            format!(
                "\n<p class=\"hm-generator\">{} <a href=\"https://hypermail-rs.github.io\">hypermail-rs</a></p>\n",
                gen_text
            )
        } else {
            String::new()
        };
        Ok(format!("{}{}{}{}", header_html, article_body, footer_html, generator))
    } else {
        // Internal default templates: use %COOKIE_NAME% substitution.
        let header_template = load_template_or_default(None, default_header_template());
        let footer_template = load_template_or_default(None, default_footer_template());
        let article_template =
            load_template_or_default(config.ihtmlhead.as_deref(), default_article_template());

        let header_html = substitute_cookies(&header_template, cookies);
        let article_content = substitute_cookies(&article_template, cookies);
        let mut nav_cookies = cookies.clone();
        set_cookie(&mut nav_cookies, "NAVIGATION", "");
        let footer_html = substitute_cookies(&footer_template, &nav_cookies);

        Ok(format!("{}{}{}", header_html, article_content, footer_html))
    }
}

fn load_template_or_default(path: Option<&str>, default: &str) -> String {
    path.and_then(|p| std::fs::read_to_string(p).ok())
        .unwrap_or_else(|| default.to_string())
}

/// Generates `attachment.html` — an index of messages that have at least one
/// attached MIME part.  Mirrors the original C hypermail `writeattachments()`.
pub fn print_attachment_index(store: &EmailStore, config: &Config) -> Result<String> {
    let i18n = I18n::new(&config.language);
    let label = i18n.get("Attachment").trim_end_matches(':');
    let title = format!("{} — {}", config.label.as_deref().unwrap_or("Archive"), label);
    let mut cookies = get_header_cookies(config, &title);

    // Collect only messages that have at least one attached body part.
    let indices_with_attachments: Vec<usize> = store
        .traverse_date_list()
        .into_iter()
        .filter(|&idx| store.emails[idx].bodylist.bodies.iter().any(|b| b.attached))
        .collect();

    let list = if config.indextable {
        let mut html = String::from("<table class=\"hm-index\">\n<tbody>\n");
        for idx in &indices_with_attachments {
            let email = &store.emails[*idx];
            html.push_str(&format!(
                "<tr>{}</tr>\n",
                render_index_row(email, config, IndexType::Date, &i18n)
            ));
        }
        html.push_str("</tbody>\n</table>\n");
        html
    } else {
        let mut html = String::from("<ul class=\"hm-index\">\n");
        for idx in &indices_with_attachments {
            let email = &store.emails[*idx];
            html.push_str(&format!(
                "  <li>{}</li>\n",
                render_index_row(email, config, IndexType::Date, &i18n)
            ));
        }
        html.push_str("</ul>\n");
        html
    };

    let top = render_archive_stats_top(store, config, IndexType::Attachment, &i18n);
    let bottom = render_archive_stats_bottom(store, config, IndexType::Attachment, &i18n);
    set_cookie(&mut cookies, "ARTICLE", &format!("{}{}{}", top, list, bottom));
    render_index_page(config, &cookies)
}

/// Generates `folders.html` — a top-level directory listing used when
/// `folder_by_date` or `msgsperfolder` is configured.
/// Mirrors the C hypermail `write_toplevel_indices()` folders page.
pub fn print_folders_index(store: &EmailStore, config: &Config) -> Result<String> {
    let i18n = I18n::new(&config.language);
    let title =
        format!("{} — {}", config.label.as_deref().unwrap_or("Archive"), i18n.get("Folders"));
    let mut cookies = get_header_cookies(config, &title);

    // Group email indices by their subdirectory name (preserving insertion order).
    let mut folder_map: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (idx, email) in store.emails.iter().enumerate() {
        let subdir = msg_subdir(email, config)
            .map(|s| s.subdir.trim_end_matches('/').to_string())
            .unwrap_or_default();
        folder_map.entry(subdir).or_default().push(idx);
    }

    let suffix = &config.htmlsuffix;
    let mut body = String::from("<ul class=\"hm-folders\">\n");

    let ordered: Vec<_> = if config.reverse_folders {
        folder_map.iter().rev().collect()
    } else {
        folder_map.iter().collect()
    };

    for (folder, indices) in &ordered {
        let count = indices.len();
        let min_date = indices.iter().map(|&i| store.emails[i].date).min().unwrap_or(0);
        let max_date = indices.iter().map(|&i| store.emails[i].date).max().unwrap_or(0);
        let min_str = get_date_str(
            min_date,
            config.dateformat.as_deref(),
            config.gmtime,
            config.eurodate,
            config.isodate,
            &config.language,
        );
        let max_str = get_date_str(
            max_date,
            config.dateformat.as_deref(),
            config.gmtime,
            config.eurodate,
            config.isodate,
            &config.language,
        );
        let label = if folder.is_empty() {
            "(root)".to_string()
        } else {
            folder.to_string()
        };
        let index_href = if folder.is_empty() {
            format!("index.{}", suffix)
        } else {
            format!("{}/index.{}", folder, suffix)
        };
        body.push_str(&format!(
            "  <li><a href=\"{}\">{}</a> — {} messages ({} – {})</li>\n",
            escape_html(&index_href),
            escape_html(&label),
            count,
            escape_html(&min_str),
            escape_html(&max_str),
        ));
    }
    body.push_str("</ul>\n");

    let top = render_archive_stats_top(store, config, IndexType::Folders, &i18n);
    set_cookie(&mut cookies, "ARTICLE", &format!("{}{}", top, body));
    render_index_page(config, &cookies)
}

/// Generates per-folder index pages (date / subject / author / thread) for each
/// subdirectory created by `folder_by_date` or `msgsperfolder`.
///
/// Returns `(relative_path, html)` pairs — the caller writes them to disk.
pub fn print_folder_index_set(
    store: &EmailStore,
    config: &Config,
) -> Result<Vec<(String, String)>> {
    let mut folder_map: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (idx, email) in store.emails.iter().enumerate() {
        let subdir = msg_subdir(email, config)
            .map(|s| s.subdir.trim_end_matches('/').to_string())
            .unwrap_or_default();
        folder_map.entry(subdir).or_default().push(idx);
    }

    let mut results: Vec<(String, String)> = Vec::new();
    let suffix = &config.htmlsuffix;

    for (folder, indices) in &folder_map {
        // Build a mini-store containing only this folder's messages.
        let mut sub_store = EmailStore::new();
        for &idx in indices {
            let e = store.emails[idx].clone();
            let new_idx = sub_store.add_email(e);
            sub_store.insert_into_date_list(new_idx);
            sub_store.insert_into_subject_list(new_idx);
            sub_store.insert_into_author_list(new_idx);
        }
        // Copy reply relationships that are entirely within this folder.
        let msgnums: std::collections::HashSet<i32> =
            sub_store.emails.iter().map(|e| e.msgnum).collect();
        for r in &store.replylist {
            if msgnums.contains(&r.from_msgnum) && msgnums.contains(&r.msgnum) {
                sub_store.replylist.push(r.clone());
            }
        }

        let prefix = if folder.is_empty() {
            String::new()
        } else {
            format!("{}/", folder)
        };

        // date  → index.html  (the "default" for the folder)
        let date_html = print_date_index(&sub_store, config)?;
        results.push((format!("{}index.{}", prefix, suffix), date_html));

        let subj_html = print_subject_index(&sub_store, config)?;
        results.push((format!("{}subject.{}", prefix, suffix), subj_html));

        let auth_html = print_author_index(&sub_store, config)?;
        results.push((format!("{}author.{}", prefix, suffix), auth_html));

        let thread_html = print_thread_index(&sub_store, config)?;
        results.push((format!("{}thread.{}", prefix, suffix), thread_html));
    }

    Ok(results)
}

/// Returns the default index filename (e.g., `"index.html"`).
pub fn get_index_filename(config: &Config) -> String {
    format!("index.{}", config.htmlsuffix)
}

/// Returns the full filesystem path for the main index file.
pub fn get_index_path(config: &Config) -> PathBuf {
    let dir = config.dir.as_deref().unwrap_or(".");
    PathBuf::from(dir).join(get_index_filename(config))
}

/// Generates per-month index pages, returning `(filename, html)` pairs.
pub fn print_monthly_index(store: &EmailStore, config: &Config) -> Result<Vec<(String, String)>> {
    use std::collections::BTreeMap;
    let mut month_map: BTreeMap<String, Vec<usize>> = BTreeMap::new();

    for idx in store.traverse_date_list() {
        let email = &store.emails[idx];
        let key = if config.gmtime {
            match chrono::Utc.timestamp_opt(email.date, 0).single() {
                Some(ts) => ts.format("%Y-%m").to_string(),
                None => "0000-00".to_string(),
            }
        } else {
            match chrono::Local.timestamp_opt(email.date, 0).single() {
                Some(ts) => ts.format("%Y-%m").to_string(),
                None => "0000-00".to_string(),
            }
        };
        month_map.entry(key).or_default().push(idx);
    }

    let mut results = Vec::new();
    for (month, indices) in &month_map {
        let title = format!("{} - {}", config.label.as_deref().unwrap_or("Archive"), month);
        let mut cookies = get_header_cookies(config, &title);
        let body = if config.reverse {
            let rev: Vec<usize> = indices.iter().rev().cloned().collect();
            render_flat_index(&rev, store, config, IndexType::Date)
        } else {
            render_flat_index(indices, store, config, IndexType::Date)
        };
        set_cookie(&mut cookies, "ARTICLE", &body);
        let html = render_index_page(config, &cookies)?;
        let filename = format!("{}.{}", month, config.htmlsuffix);
        results.push((filename, html));
    }
    Ok(results)
}

/// Generates per-year index pages, returning `(filename, html)` pairs.
pub fn print_yearly_index(store: &EmailStore, config: &Config) -> Result<Vec<(String, String)>> {
    use std::collections::BTreeMap;
    let mut year_map: BTreeMap<String, Vec<usize>> = BTreeMap::new();

    for idx in store.traverse_date_list() {
        let email = &store.emails[idx];
        let key = if config.gmtime {
            match chrono::Utc.timestamp_opt(email.date, 0).single() {
                Some(ts) => ts.format("%Y").to_string(),
                None => "0000".to_string(),
            }
        } else {
            match chrono::Local.timestamp_opt(email.date, 0).single() {
                Some(ts) => ts.format("%Y").to_string(),
                None => "0000".to_string(),
            }
        };
        year_map.entry(key).or_default().push(idx);
    }

    let mut results = Vec::new();
    for (year, indices) in &year_map {
        let title = format!("{} - {}", config.label.as_deref().unwrap_or("Archive"), year);
        let mut cookies = get_header_cookies(config, &title);
        let body = if config.reverse {
            let rev: Vec<usize> = indices.iter().rev().cloned().collect();
            render_flat_index(&rev, store, config, IndexType::Date)
        } else {
            render_flat_index(indices, store, config, IndexType::Date)
        };
        set_cookie(&mut cookies, "ARTICLE", &body);
        let html = render_index_page(config, &cookies)?;
        let filename = format!("year-{}.{}", year, config.htmlsuffix);
        results.push((filename, html));
    }
    Ok(results)
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
            name: Some("Alice".to_string()),
            email_addr: Some("alice@example.com".to_string()),
            subject: Some("Hello".to_string()),
            date: 1000,
            ..Default::default()
        };
        let e2 = EmailInfo {
            msgnum: 2,
            name: Some("Bob".to_string()),
            email_addr: Some("bob@example.com".to_string()),
            subject: Some("Re: Hello".to_string()),
            date: 2000,
            ..Default::default()
        };
        store.add_email(e1);
        store.add_email(e2);
        store.insert_into_date_list(0);
        store.insert_into_date_list(1);
        store.insert_into_subject_list(0);
        store.insert_into_subject_list(1);
        store.insert_into_author_list(0);
        store.insert_into_author_list(1);
        store
    }

    #[test]
    fn test_date_index() {
        let store = make_store();
        let config = Config::default();
        let html = print_date_index(&store, &config).unwrap();
        assert!(html.contains("Alice"));
        assert!(html.contains("Hello"));
        // Subject must be in a <strong> inside the link; author in <em> with date in <small>
        assert!(html.contains("<strong>Hello</strong>"), "subject should be wrapped in <strong>");
        assert!(html.contains("<em>Alice"), "author should be inside <em>");
        assert!(html.contains("<small>"), "date should be inside <small>");
        assert!(html.contains("<span id="), "each row should have a named anchor");
        assert!(!html.contains("> - <"), "old dash-separated format should not appear");
    }

    #[test]
    fn test_subject_index() {
        let store = make_store();
        let config = Config::default();
        let html = print_subject_index(&store, &config).unwrap();
        assert!(html.contains("Alice"));
        assert!(html.contains("Hello"));
    }

    #[test]
    fn test_author_index() {
        let store = make_store();
        let config = Config::default();
        let html = print_author_index(&store, &config).unwrap();
        assert!(html.contains("Alice"));
        assert!(html.contains("Bob"));
    }

    #[test]
    fn test_get_index_filename() {
        let config = Config::default();
        assert_eq!(get_index_filename(&config), "index.html");
    }

    #[test]
    fn test_yearly_index_filename_prefixed() {
        let store = make_store();
        let config = Config::default();
        let results = print_yearly_index(&store, &config).unwrap();
        for (filename, _) in &results {
            assert!(
                filename.starts_with("year-"),
                "yearly filename should be prefixed: {}",
                filename
            );
            assert!(
                filename.ends_with(".html"),
                "yearly filename should end with .html: {}",
                filename
            );
        }
    }

    #[test]
    fn test_yearly_index_no_collision_with_msgnum() {
        let store = make_store();
        let config = Config::default();
        let results = print_yearly_index(&store, &config).unwrap();
        // Yearly index for "1970" should be "year-1970.html", not "1970.html"
        // which could collide with msgnum 1970's message file
        for (filename, _) in &results {
            assert!(
                !filename.chars().all(|c| c == '.' || c.is_ascii_digit()),
                "filename should not be just digits + suffix: {}",
                filename
            );
        }
    }

    #[test]
    fn test_archive_stats_top_contains_starting_ending() {
        let store = make_store();
        let config = Config::default();
        let i18n = I18n::new("en");
        let html = render_archive_stats_top(&store, &config, IndexType::Date, &i18n);
        assert!(html.contains("Starting:"), "should have Starting label");
        assert!(html.contains("Ending:"), "should have Ending label");
        assert!(html.contains("2 messages sorted by:"), "should show message count");
    }

    #[test]
    fn test_archive_stats_bottom_contains_last_and_archived() {
        let store = make_store();
        let config = Config::default();
        let i18n = I18n::new("en");
        let html = render_archive_stats_bottom(&store, &config, IndexType::Date, &i18n);
        assert!(html.contains("Last message date:"), "should have last message date label");
        assert!(html.contains("Archived on:"), "should have archived-on label");
        assert!(html.contains("2 messages sorted by:"), "should show message count");
    }

    #[test]
    fn test_archive_stats_contains_about_link() {
        let store = make_store();
        let mut config = Config::default();
        config.about = Some("https://example.com/about".to_string());
        let i18n = I18n::new("en");
        let top = render_archive_stats_top(&store, &config, IndexType::Date, &i18n);
        assert!(top.contains("https://example.com/about"), "should include about link");
        assert!(top.contains("About this archive"), "should include about label");
    }

    #[test]
    fn test_archive_stats_no_about_when_not_configured() {
        let store = make_store();
        let config = Config::default();
        let i18n = I18n::new("en");
        let top = render_archive_stats_top(&store, &config, IndexType::Date, &i18n);
        assert!(
            !top.contains("About this archive"),
            "should not include about when unconfigured"
        );
    }

    #[test]
    fn test_index_uses_email_addr_when_no_name() {
        let mut store = EmailStore::new();
        let e = EmailInfo {
            msgnum: 1,
            name: None,
            email_addr: Some("noreply@example.com".to_string()),
            subject: Some("Test".to_string()),
            date: 1000,
            ..Default::default()
        };
        store.add_email(e);
        store.insert_into_date_list(0);
        store.insert_into_subject_list(0);
        store.insert_into_author_list(0);
        let config = Config::default();
        let html = print_date_index(&store, &config).unwrap();
        assert!(
            html.contains("noreply@example.com"),
            "should show email address when name is absent"
        );
        assert!(
            !html.contains("Unknown"),
            "should not show 'Unknown' fallback when email is available"
        );
    }

    #[test]
    fn test_index_date_sorted_index_has_stats_blocks() {
        let store = make_store();
        let config = Config::default();
        let html = print_date_index(&store, &config).unwrap();
        // Both top stats and bottom stats should appear
        assert!(html.contains("Starting:"), "date index should contain Starting:");
        assert!(
            html.contains("Last message date:"),
            "date index should contain Last message date:"
        );
    }

    #[test]
    fn test_no_subject_shows_locale_fallback() {
        let mut store = EmailStore::new();
        let e = EmailInfo {
            msgnum: 1,
            name: Some("Alice".to_string()),
            email_addr: Some("alice@example.com".to_string()),
            subject: None,
            date: 1000,
            ..Default::default()
        };
        store.add_email(e);
        store.insert_into_date_list(0);
        store.insert_into_subject_list(0);
        store.insert_into_author_list(0);
        let config = Config::default();
        let html = print_date_index(&store, &config).unwrap();
        assert!(html.contains("(no subject)"), "None subject should render as '(no subject)'");
    }

    // -----------------------------------------------------------------------
    // print_attachment_index
    // -----------------------------------------------------------------------

    fn make_store_with_attachment() -> EmailStore {
        use crate::message::{Body, BodyChain};
        let mut store = EmailStore::new();
        let e1 = EmailInfo {
            msgnum: 1,
            name: Some("Alice".to_string()),
            subject: Some("Has attachment".to_string()),
            date: 1000,
            bodylist: BodyChain {
                bodies: vec![Body {
                    line: "file.pdf".to_string(),
                    html: false,
                    header: false,
                    parsed_header: false,
                    attached: true,
                    demimed: false,
                    msgnum: 1,
                }],
            },
            ..Default::default()
        };
        let e2 = EmailInfo {
            msgnum: 2,
            name: Some("Bob".to_string()),
            subject: Some("No attachment".to_string()),
            date: 2000,
            ..Default::default()
        };
        store.add_email(e1);
        store.add_email(e2);
        store.insert_into_date_list(0);
        store.insert_into_date_list(1);
        store.insert_into_subject_list(0);
        store.insert_into_subject_list(1);
        store.insert_into_author_list(0);
        store.insert_into_author_list(1);
        store
    }

    #[test]
    fn test_attachment_index_includes_message_with_attachment() {
        let store = make_store_with_attachment();
        let config = Config::default();
        let html = print_attachment_index(&store, &config).unwrap();
        assert!(html.contains("Has attachment"), "should list message with attachment");
    }

    #[test]
    fn test_attachment_index_excludes_plain_message() {
        let store = make_store_with_attachment();
        let config = Config::default();
        let html = print_attachment_index(&store, &config).unwrap();
        assert!(!html.contains("No attachment"), "should NOT list message without attachment");
    }

    #[test]
    fn test_attachment_index_empty_when_no_attachments() {
        let store = make_store(); // make_store() emails have no attached bodies
        let config = Config::default();
        let html = print_attachment_index(&store, &config).unwrap();
        // The message list should be an empty <ul> with no <li> items.
        assert!(html.contains("<ul class=\"hm-index\">"), "should render the list container");
        assert!(!html.contains("<li>"), "empty attachment index should have no list items");
    }

    #[test]
    fn test_attachment_index_indextable_mode() {
        let store = make_store_with_attachment();
        let mut config = Config::default();
        config.indextable = true;
        let html = print_attachment_index(&store, &config).unwrap();
        assert!(html.contains("<table"), "indextable mode should use <table>");
        assert!(html.contains("Has attachment"));
    }

    // -----------------------------------------------------------------------
    // print_folders_index
    // -----------------------------------------------------------------------

    fn make_foldered_store() -> (EmailStore, Config) {
        let mut store = EmailStore::new();
        // Two messages with different dates → different folders under %Y-%m
        let e1 = EmailInfo {
            msgnum: 1,
            name: Some("Alice".to_string()),
            subject: Some("Jan message".to_string()),
            date: 1706745600, // 2024-02-01 00:00:00 UTC
            ..Default::default()
        };
        let e2 = EmailInfo {
            msgnum: 2,
            name: Some("Bob".to_string()),
            subject: Some("Mar message".to_string()),
            date: 1709251200, // 2024-03-01 00:00:00 UTC
            ..Default::default()
        };
        store.add_email(e1);
        store.add_email(e2);
        store.insert_into_date_list(0);
        store.insert_into_date_list(1);
        store.insert_into_subject_list(0);
        store.insert_into_subject_list(1);
        store.insert_into_author_list(0);
        store.insert_into_author_list(1);
        let mut config = Config::default();
        config.folder_by_date = Some("%Y-%m".to_string());
        config.gmtime = true; // deterministic folder names in any TZ
        (store, config)
    }

    #[test]
    fn test_folders_index_contains_folder_links() {
        let (store, config) = make_foldered_store();
        let html = print_folders_index(&store, &config).unwrap();
        assert!(html.contains("2024-"), "should list year-month folder names");
    }

    #[test]
    fn test_folders_index_shows_message_count() {
        let (store, config) = make_foldered_store();
        let html = print_folders_index(&store, &config).unwrap();
        assert!(html.contains("1 messages"), "each folder should show its message count");
    }

    #[test]
    fn test_folders_index_links_to_subfolder_index() {
        let (store, config) = make_foldered_store();
        let html = print_folders_index(&store, &config).unwrap();
        assert!(html.contains("/index.html"), "folder link should point to subfolder index.html");
    }

    // -----------------------------------------------------------------------
    // print_folder_index_set
    // -----------------------------------------------------------------------

    #[test]
    fn test_folder_index_set_returns_four_pages_per_folder() {
        let (store, config) = make_foldered_store();
        let pages = print_folder_index_set(&store, &config).unwrap();
        // 2 folders × 4 pages (index, subject, author, thread) = 8
        assert_eq!(pages.len(), 8, "should generate 4 index pages per folder");
    }

    #[test]
    fn test_folder_index_set_path_contains_folder_name() {
        let (store, config) = make_foldered_store();
        let pages = print_folder_index_set(&store, &config).unwrap();
        let paths: Vec<&str> = pages.iter().map(|(p, _)| p.as_str()).collect();
        assert!(
            paths.iter().any(|p| p.contains("2024-") && p.contains("index.html")),
            "paths should include folder-prefixed index.html; got: {:?}",
            paths
        );
    }

    #[test]
    fn test_folder_index_set_each_contains_only_folder_messages() {
        let (store, config) = make_foldered_store();
        let pages = print_folder_index_set(&store, &config).unwrap();
        // Each folder's date index should contain only its own message, not both.
        let folder_indexes: Vec<_> =
            pages.iter().filter(|(p, _)| p.ends_with("index.html")).collect();
        for (path, html) in &folder_indexes {
            // Both messages should NOT appear in the same subfolder index.
            let has_jan = html.contains("Jan message");
            let has_mar = html.contains("Mar message");
            assert!(
                !(has_jan && has_mar),
                "folder index {} should not contain messages from both folders",
                path
            );
        }
    }

    // -----------------------------------------------------------------------
    // index_href / render_nav_links — nav link routing
    // -----------------------------------------------------------------------

    #[test]
    fn test_index_href_date_is_index_when_defaultindex_date() {
        let config = Config::default(); // defaultindex = "date"
        assert_eq!(index_href(IndexType::Date, &config), "index.html");
    }

    #[test]
    fn test_index_href_date_is_date_html_when_defaultindex_subject() {
        let mut config = Config::default();
        config.defaultindex = "subject".to_string();
        assert_eq!(index_href(IndexType::Date, &config), "date.html");
    }

    #[test]
    fn test_index_href_subject_is_index_when_defaultindex_subject() {
        let mut config = Config::default();
        config.defaultindex = "subject".to_string();
        assert_eq!(index_href(IndexType::Subject, &config), "index.html");
    }

    #[test]
    fn test_index_href_author_is_index_when_defaultindex_author() {
        let mut config = Config::default();
        config.defaultindex = "author".to_string();
        assert_eq!(index_href(IndexType::Author, &config), "index.html");
    }

    #[test]
    fn test_index_href_thread_is_index_when_defaultindex_thread() {
        let mut config = Config::default();
        config.defaultindex = "thread".to_string();
        assert_eq!(index_href(IndexType::Thread, &config), "index.html");
    }

    #[test]
    fn test_index_href_non_default_types_use_named_files() {
        let config = Config::default(); // defaultindex = "date"
        assert_eq!(index_href(IndexType::Subject, &config), "subject.html");
        assert_eq!(index_href(IndexType::Author, &config), "author.html");
        assert_eq!(index_href(IndexType::Thread, &config), "thread.html");
        assert_eq!(index_href(IndexType::Attachment, &config), "attachment.html");
    }

    #[test]
    fn test_nav_links_current_page_shown_as_plain_text() {
        let store = make_store();
        let config = Config::default();
        let i18n = I18n::new("en");
        let nav = render_nav_links(&store, &config, IndexType::Date, &i18n);
        // Current page (Date) must not be a link
        assert!(
            !nav.contains("<a href=\"index.html\">Date Index</a>"),
            "current page should not be a link"
        );
        assert!(nav.contains("[ Date Index ]"), "current page should appear as plain text");
    }

    #[test]
    fn test_nav_links_other_pages_are_links() {
        let store = make_store();
        let config = Config::default(); // defaultindex=date
        let i18n = I18n::new("en");
        let nav = render_nav_links(&store, &config, IndexType::Date, &i18n);
        // Non-current pages should be links
        assert!(nav.contains("href=\"subject.html\""), "subject link should be present");
        assert!(nav.contains("href=\"author.html\""), "author link should be present");
        assert!(nav.contains("href=\"thread.html\""), "thread link should be present");
    }

    #[test]
    fn test_nav_links_subject_defaultindex_uses_index_html() {
        let store = make_store();
        let mut config = Config::default();
        config.defaultindex = "subject".to_string();
        let i18n = I18n::new("en");
        // View from the Author page so both Date and Subject links are rendered as <a>.
        let nav = render_nav_links(&store, &config, IndexType::Author, &i18n);
        // Subject is the defaultindex so its link should be index.html
        assert!(
            nav.contains("href=\"index.html\""),
            "subject defaultindex should link to index.html"
        );
        // Date is not defaultindex so its link should be date.html
        assert!(
            nav.contains("href=\"date.html\""),
            "date link should be date.html when it is not defaultindex"
        );
    }

    #[test]
    fn test_nav_links_attachment_hidden_when_no_attachments() {
        let store = make_store(); // no attached bodies
        let mut config = Config::default();
        config.attachmentsindex = true;
        let i18n = I18n::new("en");
        let nav = render_nav_links(&store, &config, IndexType::Date, &i18n);
        assert!(
            !nav.contains("attachment"),
            "attachment nav link should be absent when no attachments exist"
        );
    }

    #[test]
    fn test_nav_links_attachment_shown_when_attachments_exist() {
        let store = make_store_with_attachment();
        let mut config = Config::default();
        config.attachmentsindex = true;
        let i18n = I18n::new("en");
        let nav = render_nav_links(&store, &config, IndexType::Date, &i18n);
        assert!(
            nav.contains("attachment.html"),
            "attachment nav link should be present when attachments exist"
        );
    }

    #[test]
    fn test_nav_links_attachment_hidden_when_attachmentsindex_off() {
        let store = make_store_with_attachment();
        let mut config = Config::default();
        config.attachmentsindex = false;
        let i18n = I18n::new("en");
        let nav = render_nav_links(&store, &config, IndexType::Date, &i18n);
        assert!(
            !nav.contains("attachment"),
            "attachment nav link should be absent when attachmentsindex=false"
        );
    }

    // -----------------------------------------------------------------------
    // Subject index sort order
    // -----------------------------------------------------------------------

    #[test]
    fn test_subject_index_re_replies_sort_with_originals() {
        let mut store = EmailStore::new();

        let mut alpha = EmailInfo {
            msgnum: 1,
            name: Some("Alice".to_string()),
            subject: Some("Alpha".to_string()),
            date: 1000,
            ..Default::default()
        };
        alpha.unre_subject = Some("alpha".to_string());

        let mut re_alpha = EmailInfo {
            msgnum: 2,
            name: Some("Bob".to_string()),
            subject: Some("Re: Alpha".to_string()),
            date: 2000,
            ..Default::default()
        };
        re_alpha.unre_subject = Some("alpha".to_string());

        let mut zebra = EmailInfo {
            msgnum: 3,
            name: Some("Carol".to_string()),
            subject: Some("Zebra".to_string()),
            date: 3000,
            ..Default::default()
        };
        zebra.unre_subject = Some("zebra".to_string());

        store.add_email(alpha);
        store.add_email(re_alpha);
        store.add_email(zebra);
        store.insert_into_subject_list(0);
        store.insert_into_subject_list(1);
        store.insert_into_subject_list(2);

        let config = Config::default();
        let html = print_subject_index(&store, &config).unwrap();

        // "Zebra" should appear after both "Alpha" entries
        let alpha_pos = html.find("Alpha").unwrap();
        let zebra_pos = html.find("Zebra").unwrap();
        assert!(zebra_pos > alpha_pos, "Zebra should sort after Alpha/Re:Alpha");
    }

    // -----------------------------------------------------------------------
    // Author index sort order
    // -----------------------------------------------------------------------

    #[test]
    fn test_author_index_no_name_sorted_by_email() {
        let mut store = EmailStore::new();

        let e_zoe = EmailInfo {
            msgnum: 1,
            name: Some("Zoe".to_string()),
            email_addr: Some("zoe@example.com".to_string()),
            subject: Some("From Zoe".to_string()),
            date: 1000,
            ..Default::default()
        };
        let e_no_name = EmailInfo {
            msgnum: 2,
            name: None,
            email_addr: Some("amy@example.com".to_string()),
            subject: Some("From Amy".to_string()),
            date: 2000,
            ..Default::default()
        };
        store.add_email(e_zoe);
        store.add_email(e_no_name);
        store.insert_into_author_list(0);
        store.insert_into_author_list(1);

        let config = Config::default();
        let html = print_author_index(&store, &config).unwrap();

        // "amy@example.com" sorts before "Zoe" → Amy's message should appear first
        let amy_pos = html.find("From Amy").unwrap();
        let zoe_pos = html.find("From Zoe").unwrap();
        assert!(amy_pos < zoe_pos, "amy@ should sort before Zoe");
    }
}
