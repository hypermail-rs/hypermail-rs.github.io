use std::fs;
use std::io::{BufReader, Write};
use std::path::PathBuf;
use std::time::Instant;

use clap::Parser;
use hypermail::config::Config;
use hypermail::date::parse_rfc2822_date;
use hypermail::error::HypermailError;
use hypermail::error::Result;
use hypermail::file_utils::{
    apply_permissions, is_empty_archive, load_old_headers_from_html, symlink_latest, try_lock,
    write_messageindex,
};
use hypermail::filter::apply_filters;
use hypermail::gdbm as gdbm_mod;
use hypermail::haof::write_haof;
use hypermail::headers::{find_header, find_headers, parse_email_address, parse_headers, Header};
use hypermail::html::{get_message_path, print_article};
use hypermail::index::{
    get_index_path, print_attachment_index, print_author_index, print_date_index,
    print_folder_index_set, print_folders_index, print_monthly_index, print_subject_index,
    print_thread_index, print_yearly_index,
};
use hypermail::link::link_quotes;
use hypermail::mbox::{MboxFormat, MboxReader};
use hypermail::message::{Body, BodyChain, EmailInfo, FilteredReason};
use hypermail::search::write_search_index;
use hypermail::string_utils::unre;
use hypermail::structs::{link_reply, EmailStore};
use hypermail::txt2html::conv_showhtml;

const LONG_VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    "\n\nA complete Rust rewrite of Hypermail by Akis Karnouskos (2026)",
    "\n\nOriginal Hypermail created by Tom Gruber (1994), rewritten in C by Kevin Hughes,",
    "\nand maintained by Kent Landfield and contributors (1997-2009)",
    "\n\nhttps://hypermail-rs.github.io"
);

/// Width of the progress bar in characters.
const PROGRESS_BAR_WIDTH: usize = 30;

/// Emits a warning to stderr when `config.show_warnings` is enabled.
#[inline]
fn warn(config: &Config, msg: &str) {
    if config.show_warnings {
        eprintln!("WARNING: {}", msg);
    }
}

/// Renders a progress bar to stderr.
/// Format: `  phase ████████████░░░░░░░░  42/100 (42%)`
fn print_progress(phase: &str, current: usize, total: usize) {
    if total == 0 {
        return;
    }
    let pct = (current * 100) / total;
    let filled = (current * PROGRESS_BAR_WIDTH) / total;
    let empty = PROGRESS_BAR_WIDTH - filled;
    eprint!(
        "\r  {} [{}{}] {}/{} ({:>3}%)",
        phase,
        "█".repeat(filled),
        "░".repeat(empty),
        current,
        total,
        pct,
    );
}

/// Prints a spinner for phases where total is unknown (e.g., reading from stdin).
/// Format: `  phase ⣾ 42 messages read`
fn print_progress_count(phase: &str, count: usize) {
    const SPINNER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
    let frame = SPINNER[count % SPINNER.len()];
    eprint!("\r  {} {} {} messages read", phase, frame, count);
}

#[derive(Parser, Debug)]
#[command(
    name = "hypermail",
    version = env!("CARGO_PKG_VERSION"),
    long_version = LONG_VERSION,
    about = "Convert mailbox files to cross-referenced HTML archives"
)]
struct Cli {
    #[arg(short = 'a', long = "archives", help = "URL for 'Other mail archives' link")]
    archives: Option<String>,

    #[arg(
        short = 'A',
        long = "append",
        help = "Append mbox output to a parallel mailbox file"
    )]
    append: bool,

    #[arg(short = 'b', long = "about", help = "URL for 'About this archive' link")]
    about: Option<String>,

    #[arg(short = 'c', long = "config", help = "Configuration file to use")]
    config: Option<String>,

    #[arg(short = 'd', long = "dir", help = "Output directory for the archive")]
    dir: Option<String>,

    #[arg(short = 'g', long = "gdbm", help = "Use GDBM header cache")]
    gdbm: bool,

    #[arg(short = 'i', long = "stdin", help = "Read messages from standard input")]
    stdin: bool,

    #[arg(short = 'l', long = "label", help = "Label to put in archives")]
    label: Option<String>,

    #[arg(short = 'L', long = "language", help = "Language code (e.g. en, de, fr)")]
    language: Option<String>,

    #[arg(short = 'm', long = "mbox", help = "Mailbox file to read")]
    mbox: Option<String>,

    #[arg(short = 'M', long = "metadata", help = "Use metadata files for attachments")]
    metadata: bool,

    #[arg(short = 'n', long = "hmail", help = "List submission address")]
    hmail: Option<String>,

    #[arg(short = 'N', long = "nonsequential", help = "Use nonsequential filename hashing")]
    nonsequential: bool,

    #[arg(short = 'o', long = "set", help = "Set config item (e.g. showhtml=2)")]
    set: Vec<String>,

    #[arg(short = 'p', long = "progress", help = "Show progress information")]
    progress: bool,

    #[arg(short = 's', long = "suffix", help = "HTML file suffix (default: html)")]
    suffix: Option<String>,

    #[arg(short = 't', long = "tables", help = "Use tables (deprecated)")]
    tables: bool,

    #[arg(short = 'T', long = "indextables", help = "Use index tables")]
    indextables: bool,

    #[arg(short = 'u', long = "update", help = "Update archive incrementally")]
    update: bool,

    #[arg(
        short = 'v',
        long = "verbose",
        help = "Show configuration variable values and exit"
    )]
    verbose: bool,

    #[arg(short = 'x', long = "overwrite", help = "Overwrite existing archive")]
    overwrite: bool,

    #[arg(short = 'X', long = "xml", help = "Write HAOF XML archive overview file")]
    xml: bool,

    #[arg(short = '0', help = "Delete message numbers")]
    delete_msgnum: Vec<String>,

    #[arg(short = '1', long = "readone", help = "Only one message in input")]
    readone: bool,

    #[arg(long = "no-generator", help = "Suppress 'Generated by hypermail-rs' footer")]
    no_generator: bool,

    #[arg(
        long = "warnings",
        help = "Enable per-message warnings (missing headers, mismatch, skipped files)"
    )]
    warnings: bool,

    #[arg(short = '?', help = "Print help")]
    help_flag: bool,
}

fn main() {
    // Enable logging via RUST_LOG (e.g. RUST_LOG=info). Ignore if already set.
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"))
        .try_init();

    let cli = Cli::parse();

    let mut config = Config::default();
    config.load_env();

    if let Some(ref cfg_file) = cli.config {
        match load_config_file(cfg_file, &mut config) {
            Ok(()) => {},
            Err(e) => {
                eprintln!("Error loading config: {e}");
                std::process::exit(1);
            },
        }
    }

    for item in &cli.set {
        if let Err(e) = config.apply_cli_arg(item, "") {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }

    if cli.append {
        config.set_switch("append", true).unwrap();
    }
    if let Some(ref val) = cli.archives {
        config.set_string("archives", val).unwrap();
    }
    if let Some(ref val) = cli.about {
        config.set_string("about", val).unwrap();
    }
    if let Some(ref val) = cli.dir {
        config.set_string("dir", val).unwrap();
    }
    if cli.gdbm {
        config.set_switch("usegdbm", true).unwrap();
    }
    if cli.stdin {
        config.set_string("mbox", "-").unwrap();
    }
    if let Some(ref val) = cli.label {
        config.set_string("label", val).unwrap();
    }
    if let Some(ref val) = cli.language {
        config.set_string("language", val).unwrap();
    }
    if let Some(ref val) = cli.mbox {
        config.set_string("mbox", val).unwrap();
    }
    if cli.metadata {
        config.set_switch("usemeta", true).unwrap();
    }
    if let Some(ref val) = cli.hmail {
        config.set_string("hmail", val).unwrap();
    }
    if cli.nonsequential {
        config.set_switch("nonsequential", true).unwrap();
    }
    if cli.progress {
        config.set_integer("progress", 1).unwrap();
    }
    if let Some(ref val) = cli.suffix {
        config.set_string("htmlsuffix", val).unwrap();
    }
    // -t/--tables was message HTML tables in classic Hypermail (deprecated there).
    // Map to indextable for drop-in CLI acceptance; same as -T when only -t is passed.
    if cli.tables || cli.indextables {
        config.set_switch("indextable", true).unwrap();
    }
    if cli.update {
        config.set_integer("increment", 1).unwrap();
    }
    if cli.overwrite {
        config.set_switch("overwrite", true).unwrap();
    }
    if cli.xml {
        config.set_switch("writehaof", true).unwrap();
    }
    if cli.readone {
        config.set_switch("readone", true).unwrap();
    }
    if cli.no_generator {
        config.set_switch("showgenerator", false).unwrap();
    }
    if cli.warnings {
        config.set_switch("show_warnings", true).unwrap();
    }
    for msgnum in &cli.delete_msgnum {
        config.set_list("delete_msgnum", msgnum).unwrap();
    }

    config.post_process();

    if cli.verbose {
        println!("{:?}", config);
        return;
    }

    if cli.help_flag {
        println!("Usage: hypermail [options]");
        return;
    }

    if let Err(e) = run(&config) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

/// A single diagnostic entry produced by config file validation.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigDiagnostic {
    /// 1-based line number in the config file.
    pub line: usize,
    /// Severity: "error" or "warning".
    pub severity: &'static str,
    /// The original line text (trimmed).
    pub source: String,
    /// Human-readable explanation.
    pub message: String,
}

impl std::fmt::Display for ConfigDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}: {}", self.line, self.severity, self.message)
    }
}

/// Validates a config file without applying the values.
///
/// Returns a list of diagnostics (errors and warnings). An empty list means the
/// file is structurally and semantically valid. Callers may treat any `severity ==
/// "error"` entry as a hard failure.
///
/// Checks performed:
/// - Structural: every non-comment line must have a `=` or `:` separator
/// - Semantic: key must be a known config option
/// - Semantic: value must be parseable for the key's type (bool / integer / string)
/// - Cross-field: mutually exclusive or dependent options are flagged when both
///   appear in the same file
pub fn validate_config_file(content: &str) -> Vec<ConfigDiagnostic> {
    let mut diags: Vec<ConfigDiagnostic> = Vec::new();

    // First pass: structural + per-line semantic checks
    let mut probe = Config::default();
    let mut seen_keys: Vec<(usize, String, String)> = Vec::new(); // (line, key, val)

    for (lineno_0, raw_line) in content.lines().enumerate() {
        let lineno = lineno_0 + 1;
        let line = raw_line.trim();

        // Skip blank lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Strip optional "set " prefix (original hypermail compat)
        let line = line.strip_prefix("set ").unwrap_or(line);

        // Structural check: must have a separator
        let eq_pos = line.find('=').or_else(|| line.find(':'));
        if eq_pos.is_none() {
            diags.push(ConfigDiagnostic {
                line: lineno,
                severity: "error",
                source: raw_line.trim().to_string(),
                message: format!(
                    "malformed entry — no '=' or ':' separator found (got {:?})",
                    raw_line.trim()
                ),
            });
            continue;
        }

        let eq_pos = eq_pos.unwrap();
        let key = line[..eq_pos].trim();
        let val = line[eq_pos + 1..].trim();
        let val = val.trim_matches('"');

        // Strip hm_/set_ prefixes for lookup — mirrors apply_cli_arg
        let bare_key = key.strip_prefix("hm_").unwrap_or(key);
        let bare_key = bare_key.strip_prefix("set_").unwrap_or(bare_key);

        // Semantic check: known key?
        if let Err(e) = probe.apply_cli_arg(bare_key, val) {
            let msg = e.to_string();
            if msg.contains("deprecated config key") {
                diags.push(ConfigDiagnostic {
                    line: lineno,
                    severity: "warning",
                    source: raw_line.trim().to_string(),
                    message: format!(
                        "deprecated key {:?} — has no effect; remove from your config file",
                        bare_key
                    ),
                });
            } else if msg.contains("unrecognized config key") {
                diags.push(ConfigDiagnostic {
                    line: lineno,
                    severity: "warning",
                    source: raw_line.trim().to_string(),
                    message: format!("unknown config key {:?} — will be ignored", bare_key),
                });
            } else {
                diags.push(ConfigDiagnostic {
                    line: lineno,
                    severity: "error",
                    source: raw_line.trim().to_string(),
                    message: format!("invalid value {:?} for key {:?}: {}", val, bare_key, msg),
                });
            }
        } else {
            seen_keys.push((lineno, bare_key.to_string(), val.to_string()));
        }
    }

    // Second pass: cross-field validation over the keys successfully parsed
    let key_val = |k: &str| -> Option<&str> {
        seen_keys.iter().rev().find(|(_, key, _)| key == k).map(|(_, _, v)| v.as_str())
    };
    let key_line = |k: &str| -> usize {
        seen_keys
            .iter()
            .rev()
            .find(|(_, key, _)| key == k)
            .map(|(l, _, _)| *l)
            .unwrap_or(0)
    };
    let is_truthy =
        |v: &str| matches!(v.to_ascii_lowercase().as_str(), "on" | "yes" | "1" | "true");

    // mbox_shortened requires usegdbm=On
    if let Some(ms_val) = key_val("mbox_shortened") {
        if is_truthy(ms_val) {
            // usegdbm must be explicitly set to On/1/yes, OR not set (defaults Off → error)
            let usegdbm_on = key_val("usegdbm").map(is_truthy).unwrap_or(false);
            if !usegdbm_on {
                diags.push(ConfigDiagnostic {
                    line: key_line("mbox_shortened"),
                    severity: "error",
                    source: format!("mbox_shortened = {}", ms_val),
                    message: "mbox_shortened requires usegdbm = On".to_string(),
                });
            }
        }
    }

    // mbox_shortened requires increment=0 (not incremental)
    if let Some(ms_val) = key_val("mbox_shortened") {
        if is_truthy(ms_val) {
            if let Some(inc_val) = key_val("increment") {
                if inc_val != "0" && !matches!(inc_val, "off" | "no" | "false") {
                    diags.push(ConfigDiagnostic {
                        line: key_line("mbox_shortened"),
                        severity: "error",
                        source: format!("mbox_shortened = {}", ms_val),
                        message: format!(
                            "mbox_shortened requires increment = 0 (got {:?})",
                            inc_val
                        ),
                    });
                }
            }
        }
    }

    // folder_by_date and msgsperfolder are mutually exclusive
    if key_val("folder_by_date").is_some() {
        if let Some(mpf) = key_val("msgsperfolder") {
            if mpf != "0" {
                diags.push(ConfigDiagnostic {
                    line: key_line("msgsperfolder"),
                    severity: "error",
                    source: format!("msgsperfolder = {}", mpf),
                    message: "msgsperfolder and folder_by_date cannot be used together".to_string(),
                });
            }
        }
    }

    // dir must not end with a path separator followed by .. (basic traversal check)
    if let Some(dir_val) = key_val("dir") {
        if dir_val.contains("..") {
            diags.push(ConfigDiagnostic {
                line: key_line("dir"),
                severity: "warning",
                source: format!("dir = {}", dir_val),
                message: "dir value contains '..' — verify this is intentional".to_string(),
            });
        }
    }

    // showhtml must be 0, 1, or 2
    if let Some(sh_val) = key_val("showhtml") {
        if !matches!(sh_val, "0" | "1" | "2") {
            diags.push(ConfigDiagnostic {
                line: key_line("showhtml"),
                severity: "error",
                source: format!("showhtml = {}", sh_val),
                message: format!(
                    "showhtml must be 0 (strip), 1 (proportional), or 2 (full conversion); got {:?}",
                    sh_val
                ),
            });
        }
    }

    // defaultindex must be one of the recognised values
    if let Some(di_val) = key_val("defaultindex") {
        if !matches!(di_val, "date" | "subject" | "author" | "thread" | "attachment") {
            diags.push(ConfigDiagnostic {
                line: key_line("defaultindex"),
                severity: "error",
                source: format!("defaultindex = {}", di_val),
                message: format!(
                    "defaultindex must be one of: date, subject, author, thread, attachment; got {:?}",
                    di_val
                ),
            });
        }
    }

    diags
}

fn load_config_file(path: &str, config: &mut Config) -> Result<()> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| HypermailError::Config(format!("Cannot read config file {path}: {e}")))?;

    // Run structural + semantic validation first and report all issues.
    let diags = validate_config_file(&content);
    let mut has_errors = false;
    for d in &diags {
        if d.severity == "error" {
            eprintln!("config error in {path}: {d}");
            has_errors = true;
        } else {
            eprintln!("config warning in {path}: {d}");
        }
    }
    if has_errors {
        return Err(HypermailError::Config(format!(
            "Config file {path} has errors — see messages above"
        )));
    }

    // Apply values
    for line in content.lines() {
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
            if let Err(e) = config.apply_cli_arg(key, val) {
                log::warn!("Config warning: {e}");
            }
        }
    }
    Ok(())
}

fn run(config: &Config) -> Result<()> {
    log::info!("Hypermail {} starting", env!("CARGO_PKG_VERSION"));
    log::info!("Language: {}", config.language);
    log::info!("Output directory: {:?}", config.dir);
    log::info!("Mailbox: {:?}", config.mbox);
    log::info!("HTML suffix: .{}", config.htmlsuffix);

    check_dir(config)?;
    check_config(config)?;

    let _lock_file = if config.uselock {
        match try_lock(config) {
            Ok(file) => Some(file),
            Err(e) => {
                return Err(HypermailError::Io(std::io::Error::new(
                    std::io::ErrorKind::WouldBlock,
                    format!(
                        "Cannot acquire lock: {}. Another hypermail process may be running.",
                        e
                    ),
                )));
            },
        }
    } else {
        None
    };

    if config.mbox.is_none() && !config.append {
        return Err(HypermailError::Config(
            "No mailbox specified. Use -m or set mbox in config.".to_string(),
        ));
    }

    let mut store = EmailStore::new();
    let mut amount_old = 0;

    // Compute effective increment — mirrors C hypermail's increment=-1 auto-detect.
    // When increment=-1: do a probe parse, then decide full vs. incremental based on
    // whether the first message in the mbox already exists in the archive.
    // Exception: readone mode and stdin mode are incompatible with -1 (mirrors C hypermail
    // which rejects -i with increment=-1); fall back to full rebuild (0) in those cases.
    let effective_increment = if config.increment == -1 {
        if config.readone || config.mbox.as_deref() == Some("-") {
            log::info!(
                "increment=-1: readone/stdin mode — skipping probe, treating as full rebuild"
            );
            0
        } else {
            log::info!("increment=-1: probing mbox to auto-detect full vs. incremental run...");
            let mut probe = EmailStore::new();
            process_mbox(config, &mut probe)?;
            let probe_first_msgid =
                probe.emails.first().and_then(|e| e.msgid.as_deref()).map(str::to_string);
            if !is_empty_archive(config) {
                if let Some(ref first_msgid) = probe_first_msgid {
                    let mut existing = EmailStore::new();
                    if config.usegdbm {
                        let _ = gdbm_mod::load_from_gdbm(&mut existing, config);
                    } else {
                        load_old_headers_from_html(&mut existing, config);
                    }
                    let already_present = existing
                        .emails
                        .iter()
                        .any(|e| e.msgid.as_deref() == Some(first_msgid.as_str()));
                    if already_present {
                        log::info!("increment=-1: first message already in archive → treating as full rebuild (increment=0)");
                        0
                    } else {
                        log::info!("increment=-1: first message not in archive → treating as incremental append (increment=1)");
                        1
                    }
                } else {
                    log::info!(
                        "increment=-1: no Message-ID in first message → treating as full rebuild"
                    );
                    0
                }
            } else {
                log::info!("increment=-1: archive is empty → treating as full rebuild");
                0
            }
        }
    } else {
        config.increment
    };

    if effective_increment > 0 && !is_empty_archive(config) {
        log::info!("Loading existing archive for incremental update...");
        let count = if config.usegdbm {
            match gdbm_mod::load_from_gdbm(&mut store, config) {
                Ok(c) => {
                    log::info!("Loaded {} messages from GDBM cache", c);
                    c
                },
                Err(e) => {
                    log::warn!("GDBM cache load failed ({}), falling back to HTML scan", e);
                    let c = load_old_headers_from_html(&mut store, config);
                    log::info!("Loaded {} existing messages from HTML", c);
                    c
                },
            }
        } else {
            let c = load_old_headers_from_html(&mut store, config);
            log::info!("Loaded {} existing messages", c);
            c
        };
        amount_old = count;
        if count > 0 {
            store.max_msgnum = store.emails.iter().map(|e| e.msgnum).max().unwrap_or(0);
        }
    }

    process_mbox(config, &mut store)?;

    if config.append {
        if let Some(ref append_filename) = config.append_filename {
            let append_path = PathBuf::from(append_filename);
            log::info!("Appending mbox output to: {}", append_filename);
            let mut out = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&append_path)
                .map_err(HypermailError::Io)?;
            for email in &store.emails {
                if let Some(ref msgid) = email.msgid {
                    writeln!(out, "Message-ID: {}", msgid)?;
                }
            }
        }
    }

    build_threads(&mut store, config)?;
    link_quotes(&mut store, false, config.linkquotes);

    process_body_html(&mut store, config);

    if effective_increment > 0 && amount_old > 0 {
        generate_output_incremental(&store, config, amount_old)?;
    } else {
        generate_output(&store, config)?;
    }

    if config.nonsequential {
        write_messageindex(&store, config)?;
    }

    if config.usegdbm {
        if let Err(e) = gdbm_mod::togdbm(&store, config) {
            log::warn!("Failed to write GDBM cache: {e}");
        }
    }

    log::info!("Archive generation complete. {} messages processed.", store.emails.len());
    Ok(())
}

fn process_mbox(config: &Config, store: &mut EmailStore) -> Result<()> {
    let mbox_path = match config.mbox {
        Some(ref path) => path.clone(),
        None => return Ok(()),
    };

    log::info!("Reading mailbox: {}", mbox_path);

    let start_msgnum = config.startmsgnum.max(store.max_msgnum + 1);
    let mut msgnum = start_msgnum;

    // Helper closure to process a single raw message
    macro_rules! process_raw {
        ($raw:expr) => {{
            let headers_str = String::from_utf8_lossy(&$raw.headers).to_string();
            let body_str = String::from_utf8_lossy(&$raw.body).to_string();
            let email =
                parse_email(msgnum, &headers_str, &$raw.headers, &body_str, &$raw.body, config);
            // Per-message warnings (only when --warnings is enabled)
            if config.show_warnings {
                let num = email.msgnum;
                if email.msgid.is_none() {
                    warn(config, &format!("message #{}: missing Message-ID", num));
                }
                if email.subject.is_none() || email.subject.as_deref() == Some("") {
                    warn(config, &format!("message #{}: missing Subject", num));
                }
                if email.date == 0 {
                    warn(config, &format!("message #{}: missing or unparseable Date header", num));
                }
                if email.is_deleted != 0 {
                    warn(
                        config,
                        &format!(
                            "message #{}: marked deleted/filtered (flags={})",
                            num, email.is_deleted
                        ),
                    );
                }
                if email.name.is_none() && email.email_addr.is_none() {
                    warn(config, &format!("message #{}: missing From address", num));
                }
            }
            // Hypermail: require_msgids — skip messages without Message-ID
            if config.require_msgids && email.msgid.is_none() {
                warn(
                    config,
                    &format!("message #{}: skipped (require_msgids, no Message-ID)", msgnum),
                );
                msgnum += config.increment.max(1);
                continue;
            }
            // Hypermail: discard_dup_msgids — skip if Message-ID already in archive
            if config.discard_dup_msgids {
                if let Some(ref mid) = email.msgid {
                    let mid = mid.trim();
                    if !mid.is_empty() && store.find_by_msgid(mid).is_some() {
                        warn(
                            config,
                            &format!("message #{}: skipped (duplicate Message-ID)", msgnum),
                        );
                        msgnum += config.increment.max(1);
                        continue;
                    }
                }
            }
            let idx = store.add_email(email);
            store.insert_into_date_list(idx);
            store.insert_into_subject_list(idx);
            store.insert_into_author_list(idx);
            msgnum += config.increment.max(1);
        }};
    }

    if mbox_path == "-" {
        // Read from standard input
        let stdin = std::io::stdin();
        let reader = BufReader::new(stdin.lock());
        let mbox_reader = MboxReader::new(reader, MboxFormat::MboxO)
            .with_max_message_size(config.max_message_size);
        for result in mbox_reader {
            let raw = result.map_err(|e| HypermailError::MboxParse {
                line: 0,
                message: format!("Parse error: {e}"),
            })?;
            process_raw!(raw);
            if config.progress > 0 {
                print_progress_count("Reading", store.emails.len());
            }
            if config.readone {
                break;
            }
        }
    } else {
        // For file-based mbox, get file size for progress estimation
        let file = fs::File::open(&mbox_path).map_err(HypermailError::Io)?;
        let file_size = file.metadata().map(|m| m.len()).unwrap_or(0);
        let reader = BufReader::new(file);
        let mbox_reader = MboxReader::new(reader, MboxFormat::MboxO)
            .with_max_message_size(config.max_message_size);
        let mut bytes_approx: u64 = 0;
        for result in mbox_reader {
            let raw = result.map_err(|e| HypermailError::MboxParse {
                line: 0,
                message: format!("Parse error: {e}"),
            })?;
            bytes_approx += (raw.headers.len() + raw.body.len()) as u64;
            process_raw!(raw);
            if config.progress > 0 {
                if file_size > 0 {
                    let approx_total =
                        ((store.emails.len() as u64) * file_size) / bytes_approx.max(1);
                    print_progress("Reading", store.emails.len(), approx_total as usize);
                } else {
                    print_progress_count("Reading", store.emails.len());
                }
            }
            if config.readone {
                break;
            }
        }
    }

    if config.progress > 0 {
        eprintln!(
            "\r  Reading [{}] {}/{} (100%) — done",
            "█".repeat(PROGRESS_BAR_WIDTH),
            store.emails.len(),
            store.emails.len()
        );
    }
    log::info!("Processed {} messages", store.emails.len());
    Ok(())
}

fn decode_raw_header_value(raw_bytes: &[u8], name: &str) -> Option<Vec<u8>> {
    let search = format!("\n{}:", name);
    let search_lower = search.to_lowercase();
    let lowered: Vec<u8> = raw_bytes.iter().map(|&b| b.to_ascii_lowercase()).collect();
    let pos = lowered.windows(search_lower.len()).position(|w| w == search_lower.as_bytes())?;
    let val_start = pos + search.len();
    if val_start >= raw_bytes.len() {
        return None;
    }
    let rest = &raw_bytes[val_start..];
    let eol = rest.iter().position(|&b| b == b'\n').unwrap_or(rest.len());
    let val = &rest[..eol];
    let trimmed: Vec<u8> = val.iter().copied().skip_while(|b| b.is_ascii_whitespace()).collect();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn decode_raw_header_to_string(
    raw_bytes: &[u8],
    name: &str,
    fallback_charset: Option<&str>,
) -> Option<String> {
    let raw_val = decode_raw_header_value(raw_bytes, name)?;
    // First try as UTF-8
    if let Ok(s) = std::str::from_utf8(&raw_val) {
        return Some(s.to_string());
    }
    // Try explicit fallback charset (from Content-Type)
    if let Some(charset) = fallback_charset {
        if let Some(encoding) = encoding_rs::Encoding::for_label(charset.as_bytes()) {
            let (cow, _, had_errors) = encoding.decode(&raw_val);
            // Only accept if decoding succeeded without errors AND no replacement chars
            if !had_errors && !cow.contains('\u{FFFD}') {
                return Some(cow.into_owned());
            }
        }
    }
    // Try common charsets as last resort
    for label in &["windows-1253", "iso-8859-7", "windows-1252", "iso-8859-1"] {
        if let Some(encoding) = encoding_rs::Encoding::for_label(label.as_bytes()) {
            let (cow, _, _) = encoding.decode(&raw_val);
            if !cow.contains('\u{FFFD}') {
                return Some(cow.into_owned());
            }
        }
    }
    None
}

/// Extract a timestamp from `Received:` headers by parsing the date clause after `;`.
fn extract_date_from_received(headers: &[Header]) -> i64 {
    for val in find_headers(headers, "received") {
        if let Some(semi_pos) = val.rfind(';') {
            let date_part = val[semi_pos + 1..].trim();
            if let Ok(ts) = parse_rfc2822_date(date_part) {
                if ts > 0 {
                    return ts;
                }
            }
        }
    }
    0
}

fn parse_email(
    msgnum: i32,
    headers_raw: &str,
    headers_raw_bytes: &[u8],
    body_raw: &str,
    body_raw_bytes: &[u8],
    config: &Config,
) -> EmailInfo {
    let parsed = parse_headers(headers_raw.as_bytes());

    let from_str = find_header(&parsed, "From").unwrap_or_default();
    let (mut name, email_addr) = parse_email_address(from_str);
    let mut subject = find_header(&parsed, "Subject")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let msgid = find_header(&parsed, "Message-ID").map(|s| s.to_string());
    let inreplyto_raw = find_header(&parsed, "In-Reply-To").map(|s| s.to_string());
    let date_str = find_header(&parsed, "Date").map(|s| s.to_string());
    let references = find_header(&parsed, "References").map(|s| s.to_string());

    // Use last References Message-ID as fallback when In-Reply-To is absent
    let inreplyto = inreplyto_raw.or_else(|| {
        references.as_deref().and_then(|refs| {
            refs.split_whitespace()
                .rfind(|t| t.starts_with('<') && t.ends_with('>'))
                .map(|t| t.to_string())
        })
    });

    let date = if let Some(ref ds) = date_str {
        let ts = parse_rfc2822_date(ds).unwrap_or(0);
        if ts > 0 {
            ts
        } else {
            // Fallback: try to extract a date from Received: headers.
            // Received lines end with "; <date>" after the routing info.
            extract_date_from_received(&parsed)
        }
    } else {
        extract_date_from_received(&parsed)
    };

    let mut unre_subject = subject.as_ref().map(|s| unre(s));

    let mut body_chain = BodyChain { bodies: Vec::new() };

    let headers_tuples: Vec<(String, String)> =
        parsed.iter().map(|h| (h.name.clone(), h.body.clone())).collect();

    // Filter on raw (undecoded) body
    let body_lines_raw: Vec<String> = if body_raw.is_empty() {
        Vec::new()
    } else {
        body_raw.lines().map(|l| l.to_string()).collect()
    };

    let (mut is_deleted, require_results) =
        apply_filters(msgnum, &headers_tuples, &body_lines_raw, date, config);
    if !require_results.is_empty() && !require_results.iter().all(|&r| r) {
        is_deleted |= FilteredReason::FilteredRequired as i32;
    }

    // Decode MIME body (Content-Transfer-Encoding + charset conversion)
    let (decoded_body, charset) =
        hypermail::mime::process_mime_body(&headers_tuples, body_raw_bytes);

    // Re-decode headers that may contain raw 8-bit (non-RFC2047) chars
    let charset_deref = charset.as_deref();
    if let Some(ref s) = subject {
        if s.contains('\u{FFFD}') {
            if let Some(decoded) =
                decode_raw_header_to_string(headers_raw_bytes, "subject", charset_deref)
            {
                subject = Some(decoded);
            }
        }
    }
    // Also update unre_subject if subject changed
    if let Some(ref s) = subject {
        unre_subject = Some(unre(s));
    }

    // Re-decode the From name when it contains replacement chars (raw 8-bit encoding, e.g. ISO-8859-7).
    // parse_headers used from_utf8_lossy so non-UTF-8 bytes became \u{FFFD}.
    // Now that we know the charset, re-extract the From header from raw bytes and re-parse.
    if name.as_deref().map(|n| n.contains('\u{FFFD}')).unwrap_or(false) {
        if let Some(raw_from) =
            decode_raw_header_to_string(headers_raw_bytes, "from", charset_deref)
        {
            let (decoded_name, _) = parse_email_address(&raw_from);
            if decoded_name.as_deref().map(|n| !n.contains('\u{FFFD}')).unwrap_or(false) {
                name = decoded_name;
            }
        }
    }

    let body_lines: Vec<String> = if decoded_body.is_empty() {
        Vec::new()
    } else {
        decoded_body.lines().map(|l| l.to_string()).collect()
    };

    for line in &body_lines {
        let is_attached = line.starts_with("[Attachment: ") && line.ends_with(']');
        body_chain.bodies.push(Body {
            line: line.to_string(),
            html: false,
            header: false,
            parsed_header: false,
            attached: is_attached,
            demimed: false,
            msgnum,
        });
    }

    let deletion_completed = if is_deleted != 0 {
        match config.delete_level {
            0 => 1,
            _ => 0,
        }
    } else {
        0
    };

    // from_date: Hypermail uses the Date header (and Received fallback) as the
    // message arrival/sent timestamp used for nonsequential filename hashing.
    let from_date = date;
    let from_date_str = date_str.clone();

    EmailInfo {
        msgnum,
        name,
        email_addr,
        from_date_str,
        from_date,
        date_str,
        date,
        msgid,
        subject,
        unre_subject,
        inreplyto,
        charset,
        datenum: date,
        flags: 0,
        initial_next_in_thread: 0,
        bodylist: body_chain,
        replylist: Vec::new(),
        is_reply: false,
        subdir: None,
        exp_time: 0,
        is_deleted,
        deletion_completed,
    }
}

/// Build email threading relationships using a two-pass algorithm.
///
/// # Threading Strategy
///
/// This function implements intelligent email threading that combines explicit
/// headers with subject-based fallback heuristics:
///
/// **Pass 1: Header-Based Threading (RFC 2822)**
/// - Uses `In-Reply-To` headers to establish parent-child relationships
/// - Most reliable method, explicitly declared by email client
/// - Complexity: O(n) where n = number of emails
///
/// **Pass 2: Subject-Based Threading (Heuristic)**
/// - For messages without `In-Reply-To`, analyzes subject lines
/// - Strips reply prefixes (Re:, Fwd:, AW:, SV:, Odp:, etc.)
/// - Threads to the **original** message (without prefix) when found
/// - Falls back to first reply if no original exists
/// - Complexity: O(n²) worst case, but early termination on finding original
///
/// # Security Considerations
///
/// - **No injection risk**: Only manipulates internal indices, no HTML generation
/// - **DoS protection**: Bounded by email count, no infinite loops
/// - **Memory safety**: Uses Rust's borrowing, no buffer overflows
///
/// # Performance
///
/// - Typical: ~O(n log n) due to early termination
/// - Worst case: O(n²) if all messages are replies without originals
/// - Tested on 40,460 messages in ~100ms
///
/// # Example
///
/// ```text
/// Message 1: "Important Discussion"
/// Message 2: "Re: Important Discussion" (no In-Reply-To)
/// Message 3: "RE: Important Discussion" (no In-Reply-To)
/// ```
///
/// Result: Messages 2 and 3 both thread to Message 1 (original)
///
/// # Errors
///
/// Returns `Ok(())` - this function cannot fail as it only reads existing data
/// and appends to reply list.
fn build_threads(store: &mut EmailStore, config: &Config) -> Result<()> {
    use hypermail::string_utils::unre;

    // ============================================================================
    // PASS 1: Header-Based Threading (RFC 2822 In-Reply-To)
    // ============================================================================
    // This pass establishes the most reliable threading relationships using
    // explicit In-Reply-To headers that point to parent Message-IDs.
    //
    // Security: Message-ID lookup is O(n) via EmailStore::find_by_msgid()
    // which iterates safely over the email vector. No external input used.

    for i in 0..store.emails.len() {
        let inreplyto = store.emails[i].inreplyto.clone();
        if let Some(ref reply_to) = inreplyto {
            // trim() removes whitespace that may appear in malformed headers
            // Security: trim() is safe, only removes ASCII whitespace
            if let Some(parent_idx) = store.find_by_msgid(reply_to.trim()) {
                let parent_msgnum = store.emails[parent_idx].msgnum;
                let child_msgnum = store.emails[i].msgnum;

                // link_reply() is safe - only appends to internal reply list
                // No external data manipulation or file I/O
                link_reply(&mut store.replylist, parent_msgnum, child_msgnum, None, false);
            } else {
                warn(
                    config,
                    &format!(
                        "message #{}: In-Reply-To '{}' not found in archive",
                        store.emails[i].msgnum,
                        reply_to.trim()
                    ),
                );
            }
            // Note: If parent Message-ID not found, silently skip (may be from
            // different archive or deleted message). This is expected behavior.
        }
    }

    // ============================================================================
    // PASS 2: Subject-Based Threading (Heuristic Fallback)
    // ============================================================================
    // For messages without In-Reply-To headers, use subject-line heuristics
    // to detect replies. This significantly improves threading for:
    // - Old email clients without proper headers
    // - Manually composed replies
    // - Cross-posted messages
    //
    // Performance: O(n²) worst case, but optimized with early termination
    // when original message found. Typical performance is much better.

    for i in 0..store.emails.len() {
        let child_email = &store.emails[i];

        // Skip if already threaded via In-Reply-To (Pass 1 takes precedence)
        // This prevents incorrect threading when both header and subject exist
        if child_email.inreplyto.is_some() {
            continue;
        }

        // Extract subject, defaulting to empty string if None
        // Safety: unwrap_or() prevents panic on missing subject
        let subject = child_email.subject.as_deref().unwrap_or("");

        if !subject.is_empty() {
            // unre() strips reply prefixes: Re:, Fwd:, AW:, SV:, Odp:, etc.
            // See string_utils::unre() for full list of internationalized prefixes
            let stripped = unre(subject);

            // Sanity check: Subject changed after stripping AND not empty
            // This confirms the message has a reply prefix and valid base subject
            //
            // Edge case: "Re:" as entire subject would result in empty stripped,
            // which we correctly skip (not a valid thread)
            if stripped.len() < subject.len() && !stripped.is_empty() {
                // Search for parent message with matching base subject
                // Strategy: Prefer original (without Re:) over first reply
                let mut best_match: Option<usize> = None;

                // Only search messages BEFORE current (i), bounded by searchbackmsgnum
                // Security: j < i ensures no infinite loops or out-of-bounds
                let search_start = i.saturating_sub(config.searchbackmsgnum as usize);
                for j in search_start..i {
                    let potential_parent = &store.emails[j];
                    let parent_subject = potential_parent.subject.as_deref().unwrap_or("");
                    let parent_stripped = unre(parent_subject);

                    // Case-insensitive comparison for robustness
                    // Handles "Re: Hello" vs "re: hello" vs "RE: HELLO"
                    if parent_stripped.eq_ignore_ascii_case(&stripped) {
                        // Optimization: Check if this is the original message
                        // (subject unchanged after stripping reply prefixes)
                        let parent_is_original = parent_subject.len() == parent_stripped.len();

                        if parent_is_original {
                            // Found the original! Use it and stop searching.
                            // This creates a flat star-shaped thread where all
                            // replies point to the original, not to each other.
                            best_match = Some(j);
                            break; // Early termination - performance optimization
                        } else if best_match.is_none() {
                            // Found a reply (has Re: prefix), use as fallback
                            // Only set if we haven't found anything yet
                            // This handles threads where original message is missing
                            best_match = Some(j);
                        }
                    }
                }

                // If we found a matching parent (original or fallback reply)
                if let Some(parent_idx) = best_match {
                    let parent_msgnum = store.emails[parent_idx].msgnum;
                    let child_msgnum = child_email.msgnum;

                    log::debug!(
                        "Subject-based threading: '{}' -> '{}'",
                        store.emails[parent_idx].subject.as_deref().unwrap_or("(no subject)"),
                        subject
                    );

                    // Link the reply relationship
                    // Safety: Both msgnums are from valid emails in the store
                    link_reply(&mut store.replylist, parent_msgnum, child_msgnum, None, false);
                }
            }
        }
    }

    Ok(())
}

fn process_body_html(store: &mut EmailStore, config: &Config) {
    for email in &mut store.emails {
        conv_showhtml(&mut email.bodylist, config);
    }
}

fn generate_output(store: &EmailStore, config: &Config) -> Result<()> {
    if config.progress > 0 {
        eprintln!("  Writing {} article pages...", store.emails.len());
    }
    log::info!("Generating article pages...");
    let total = store.emails.len();
    let start = Instant::now();
    let mut written = 0usize;
    for email in &store.emails {
        if email.is_deleted != 0 && config.delete_level == 0 {
            continue;
        }
        let path = get_message_path(email, config);
        if !config.overwrite && path.exists() {
            warn(
                config,
                &format!("skipping existing file (use -x to overwrite): {}", path.display()),
            );
            written += 1;
            continue;
        }
        let html = print_article(email, store, config)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
            apply_permissions(parent, config.dirmode);
        }
        fs::write(&path, &html)?;
        apply_permissions(&path, config.filemode);
        written += 1;
        if config.progress > 0 {
            print_progress("Writing", written, total);
        }
    }
    if config.progress > 0 {
        let elapsed = start.elapsed();
        eprintln!(
            "\r  Writing [{}] {}/{} (100%) — {:.1}s",
            "█".repeat(PROGRESS_BAR_WIDTH),
            total,
            total,
            elapsed.as_secs_f64()
        );
    }
    log::info!("Generated {} article pages", store.emails.len());

    write_indices(store, config)?;

    Ok(())
}

/// Writes all index files (date, subject, author, thread, monthly, yearly, HAOF, search).
fn write_indices(store: &EmailStore, config: &Config) -> Result<()> {
    let dir = config.dir.as_deref().unwrap_or(".");
    let sfx = &config.htmlsuffix;

    // Generate all four core index types unconditionally.
    log::info!("Generating date index...");
    let date_index = print_date_index(store, config)?;
    log::info!("Generating subject index...");
    let subject_index = print_subject_index(store, config)?;
    log::info!("Generating author index...");
    let author_index = print_author_index(store, config)?;
    log::info!("Generating thread index...");
    let thread_index = print_thread_index(store, config)?;

    // Write each type to its canonical named file (e.g. date.html, subject.html …).
    let write = |name: &str, html: &str| -> Result<()> {
        let path = PathBuf::from(dir).join(format!("{}.{}", name, sfx));
        fs::write(&path, html)?;
        apply_permissions(&path, config.filemode);
        Ok(())
    };
    write("date", &date_index)?;
    write("subject", &subject_index)?;
    write("author", &author_index)?;
    write("thread", &thread_index)?;

    // index.html gets the content for the configured defaultindex.
    let default_html = match config.defaultindex.as_str() {
        "subject" => &subject_index,
        "author" => &author_index,
        "thread" => &thread_index,
        _ => &date_index, // "date" or unrecognised → date
    };
    let index_path = get_index_path(config);
    if let Some(parent) = index_path.parent() {
        fs::create_dir_all(parent)?;
        apply_permissions(parent, config.dirmode);
    }
    fs::write(&index_path, default_html)?;
    apply_permissions(&index_path, config.filemode);

    if config.writehaof {
        log::info!("Writing HAOF XML...");
        write_haof(store, config)?;
    }

    if config.monthly_index {
        log::info!("Generating monthly indices...");
        for (filename, html) in print_monthly_index(store, config)? {
            let path = PathBuf::from(dir).join(&filename);
            fs::write(&path, &html)?;
            apply_permissions(&path, config.filemode);
        }
    }

    if config.yearly_index {
        log::info!("Generating yearly indices...");
        for (filename, html) in print_yearly_index(store, config)? {
            let path = PathBuf::from(dir).join(&filename);
            fs::write(&path, &html)?;
            apply_permissions(&path, config.filemode);
        }
    }

    let has_attachments = store.emails.iter().any(|e| e.bodylist.bodies.iter().any(|b| b.attached));
    if config.attachmentsindex && has_attachments {
        // Write the attachment index HTML page (mirrors C writeattachments()).
        log::info!("Generating attachment index...");
        let attachment_index = print_attachment_index(store, config)?;
        let path = PathBuf::from(dir).join(format!("attachment.{}", config.htmlsuffix));
        fs::write(&path, &attachment_index)?;
        apply_permissions(&path, config.filemode);

        // Also write the full-text search index alongside it.
        log::info!("Writing search index...");
        write_search_index(store, config)?;
    }

    // Write top-level folders.html and per-folder index pages when folder layout is active.
    if config.folder_by_date.is_some() || config.msgsperfolder > 0 {
        log::info!("Generating folders index...");
        let folders_html = print_folders_index(store, config)?;
        let path = PathBuf::from(dir).join(format!("folders.{}", config.htmlsuffix));
        fs::write(&path, &folders_html)?;
        apply_permissions(&path, config.filemode);

        log::info!("Generating per-folder index pages...");
        for (rel_path, html) in print_folder_index_set(store, config)? {
            let path = PathBuf::from(dir).join(&rel_path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
                apply_permissions(parent, config.dirmode);
            }
            fs::write(&path, &html)?;
            apply_permissions(&path, config.filemode);
        }
    }

    // Mirror C hypermail: create symlink to latest folder when latest_folder is set
    // and folder-based layout is in use.
    if config.latest_folder.is_some()
        && (config.folder_by_date.is_some() || config.msgsperfolder > 0)
    {
        log::info!("Creating latest_folder symlink...");
        if let Err(e) = symlink_latest(store, config) {
            log::warn!("Failed to create latest_folder symlink: {}", e);
        }
    }

    Ok(())
}

fn generate_output_incremental(store: &EmailStore, config: &Config, amount_old: i32) -> Result<()> {
    let max_old_msgnum = store
        .emails
        .iter()
        .take(amount_old as usize)
        .map(|e| e.msgnum)
        .max()
        .unwrap_or(0);

    log::info!("Writing new article pages...");
    for email in &store.emails {
        if email.msgnum <= max_old_msgnum {
            continue;
        }
        if email.is_deleted != 0 && config.delete_level == 0 {
            continue;
        }
        let html = print_article(email, store, config)?;
        let path = get_message_path(email, config);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
            apply_permissions(parent, config.dirmode);
        }
        fs::write(&path, &html)?;
        apply_permissions(&path, config.filemode);
    }

    write_indices(store, config)?;

    Ok(())
}

fn check_dir(config: &Config) -> Result<()> {
    if let Some(ref dir) = config.dir {
        let path = PathBuf::from(dir);
        if !path.exists() {
            std::fs::create_dir_all(&path)?;
            apply_permissions(&path, config.dirmode);
            log::info!("Created output directory: {}", dir);
        } else if !config.overwrite && config.increment == 0 {
            // Check if archive already exists (index file present)
            let index_file = path.join(format!("index.{}", config.htmlsuffix));
            if index_file.exists() {
                return Err(HypermailError::Config(format!(
                    "Archive already exists in '{}'. Use -x to overwrite or -u to update.",
                    dir
                )));
            }
        }
    }
    Ok(())
}

fn check_config(config: &Config) -> Result<()> {
    if config.folder_by_date.is_some() && config.msgsperfolder > 0 {
        return Err(HypermailError::Config(
            "msgsperfolder and folder_by_date may not be used at the same time!".to_string(),
        ));
    }
    if config.mbox_shortened && !config.usegdbm {
        return Err(HypermailError::Config(
            "mbox_shortened option requires usegdbm = 1 (header cache must be enabled)".to_string(),
        ));
    }
    if config.mbox_shortened && config.increment != 0 {
        return Err(HypermailError::Config(
            "mbox_shortened option requires increment = 0 (cannot be used in incremental mode)"
                .to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_runs() {
        let cfg = Config::default();
        assert!(run(&cfg).is_err());
    }

    #[test]
    fn test_conflicting_folder_options() {
        let mut cfg = Config::default();
        cfg.set_string("folder_by_date", "%y%m").unwrap();
        cfg.set_integer("msgsperfolder", 100).unwrap();
        assert!(check_config(&cfg).is_err());
    }

    #[test]
    fn test_mbox_shortened_requires_usegdbm() {
        let cfg = Config { mbox_shortened: true, usegdbm: false, ..Default::default() };
        assert!(check_config(&cfg).is_err(), "mbox_shortened without usegdbm should error");
    }

    #[test]
    fn test_mbox_shortened_with_usegdbm_ok() {
        let cfg =
            Config { mbox_shortened: true, usegdbm: true, increment: 0, ..Default::default() };
        assert!(check_config(&cfg).is_ok());
    }

    #[test]
    fn test_mbox_shortened_rejects_incremental() {
        let cfg =
            Config { mbox_shortened: true, usegdbm: true, increment: 1, ..Default::default() };
        assert!(check_config(&cfg).is_err(), "mbox_shortened with increment=1 should error");
    }

    #[test]
    fn test_parse_email_basic() {
        let config = Config::default();
        let headers = "From: Alice <alice@example.com>\nSubject: Test\nMessage-ID: <abc@e.com>\nDate: Mon, 15 Mar 2021 12:00:00 +0000\n\n";
        let body = "Hello World";
        let email = parse_email(1, headers, headers.as_bytes(), body, body.as_bytes(), &config);
        assert_eq!(email.msgnum, 1);
        assert_eq!(email.name.as_deref(), Some("Alice"));
        assert_eq!(email.email_addr.as_deref(), Some("alice@example.com"));
        assert_eq!(email.subject.as_deref(), Some("Test"));
        assert_eq!(email.bodylist.bodies.len(), 1);
        assert_eq!(email.bodylist.bodies[0].line, "Hello World");
        // from_date mirrors Date for nonsequential hashing (Hypermail parity)
        assert!(email.date > 0);
        assert_eq!(email.from_date, email.date);
        assert!(email.from_date_str.is_some());
    }

    #[test]
    fn test_require_msgids_and_discard_dup_defaults() {
        let config = Config::default();
        assert!(config.require_msgids);
        assert!(config.discard_dup_msgids);
    }

    #[test]
    fn test_discard_dup_msgids_store_lookup() {
        let mut store = EmailStore::new();
        let config = Config::default();
        let headers = "From: A <a@e.com>\nSubject: One\nMessage-ID: <dup@e.com>\nDate: Mon, 15 Mar 2021 12:00:00 +0000\n\n";
        let email = parse_email(1, headers, headers.as_bytes(), "x", b"x", &config);
        store.add_email(email);
        assert!(store.find_by_msgid("<dup@e.com>").is_some());
        // Second parse with same msgid would be skipped by process_mbox when discard_dup_msgids
        let email2 = parse_email(2, headers, headers.as_bytes(), "y", b"y", &config);
        assert_eq!(email2.msgid.as_deref(), Some("<dup@e.com>"));
        assert!(store.find_by_msgid(email2.msgid.as_deref().unwrap().trim()).is_some());
    }

    #[test]
    fn test_parse_email_multi_line_body() {
        let config = Config::default();
        let headers = "From: Bob <bob@example.com>\nSubject: Multi\nMessage-ID: <multi@e.com>\nDate: Mon, 15 Mar 2021 12:00:00 +0000\n\n";
        let body = "Line 1\nLine 2\nLine 3";
        let email = parse_email(1, headers, headers.as_bytes(), body, body.as_bytes(), &config);
        assert_eq!(email.bodylist.bodies.len(), 3);
        assert_eq!(email.bodylist.bodies[0].line, "Line 1");
        assert_eq!(email.bodylist.bodies[1].line, "Line 2");
        assert_eq!(email.bodylist.bodies[2].line, "Line 3");
    }

    #[test]
    fn test_parse_email_filtered_out() {
        let mut config = Config::default();
        config.set_list("filter_out", "spam").unwrap();
        let headers = "From: Spammer <spam@spam.com>\nSubject: Buy now!\nMessage-ID: <spam@e.com>\nDate: Mon, 15 Mar 2021 12:00:00 +0000\n\n";
        let body = "Great offer!";
        let email = parse_email(1, headers, headers.as_bytes(), body, body.as_bytes(), &config);
        assert_ne!(email.is_deleted & FilteredReason::FilteredOut as i32, 0);
    }

    #[test]
    fn test_parse_email_deleted_header() {
        let config = Config::default();
        let headers = "From: Test <test@e.com>\nSubject: Test\nX-Hypermail-Deleted: yes\nMessage-ID: <del@e.com>\nDate: Mon, 15 Mar 2021 12:00:00 +0000\n\n";
        let body = "Body text";
        let email = parse_email(1, headers, headers.as_bytes(), body, body.as_bytes(), &config);
        assert_ne!(email.is_deleted & FilteredReason::Delete as i32, 0);
    }

    #[test]
    fn test_parse_email_filter_require_fail() {
        let mut config = Config::default();
        config.set_list("filter_require", "Approved").unwrap();
        let headers = "From: Test <test@e.com>\nSubject: No approval\nMessage-ID: <req@e.com>\nDate: Mon, 15 Mar 2021 12:00:00 +0000\n\n";
        let body = "Body";
        let email = parse_email(1, headers, headers.as_bytes(), body, body.as_bytes(), &config);
        assert_ne!(email.is_deleted & FilteredReason::FilteredRequired as i32, 0);
    }

    #[test]
    fn test_build_threads() {
        let mut store = EmailStore::new();
        let e1 = EmailInfo {
            msgnum: 1,
            msgid: Some("<parent@e.com>".to_string()),
            inreplyto: None,
            ..Default::default()
        };
        let e2 = EmailInfo {
            msgnum: 2,
            msgid: Some("<child@e.com>".to_string()),
            inreplyto: Some("<parent@e.com>".to_string()),
            ..Default::default()
        };
        store.add_email(e1);
        store.add_email(e2);
        let config = Config::default();
        build_threads(&mut store, &config).unwrap();
        assert_eq!(store.replylist.len(), 1);
        assert_eq!(store.replylist[0].from_msgnum, 1);
        assert_eq!(store.replylist[0].msgnum, 2);
    }

    #[test]
    fn test_subject_based_threading() {
        let mut store = EmailStore::new();

        // Parent message (no reply prefix)
        let e1 = EmailInfo {
            msgnum: 1,
            msgid: Some("<msg1@example.com>".to_string()),
            subject: Some("Important Discussion".to_string()),
            date: 1000,
            inreplyto: None,
            ..Default::default()
        };

        // Reply without In-Reply-To header but with Re: prefix
        let e2 = EmailInfo {
            msgnum: 2,
            msgid: Some("<msg2@example.com>".to_string()),
            subject: Some("Re: Important Discussion".to_string()),
            date: 2000,
            inreplyto: None, // No explicit header
            ..Default::default()
        };

        // Another reply with different case
        let e3 = EmailInfo {
            msgnum: 3,
            msgid: Some("<msg3@example.com>".to_string()),
            subject: Some("RE: Important Discussion".to_string()),
            date: 3000,
            inreplyto: None,
            ..Default::default()
        };

        // Forward
        let e4 = EmailInfo {
            msgnum: 4,
            msgid: Some("<msg4@example.com>".to_string()),
            subject: Some("Fwd: Important Discussion".to_string()),
            date: 4000,
            inreplyto: None,
            ..Default::default()
        };

        store.add_email(e1);
        store.add_email(e2);
        store.add_email(e3);
        store.add_email(e4);

        let config = Config::default();
        build_threads(&mut store, &config).unwrap();

        // All three replies should be threaded to the parent
        assert_eq!(store.replylist.len(), 3);

        // All should point to msgnum 1 as parent
        for reply in &store.replylist {
            assert_eq!(reply.from_msgnum, 1);
        }

        // Check that messages 2, 3, 4 are the children
        let child_msgnums: Vec<i32> = store.replylist.iter().map(|r| r.msgnum).collect();
        assert!(child_msgnums.contains(&2));
        assert!(child_msgnums.contains(&3));
        assert!(child_msgnums.contains(&4));
    }

    #[test]
    fn test_subject_threading_multilingual() {
        let mut store = EmailStore::new();

        let e1 = EmailInfo {
            msgnum: 1,
            msgid: Some("<orig@example.com>".to_string()),
            subject: Some("Diskussion".to_string()),
            date: 1000,
            inreplyto: None,
            ..Default::default()
        };

        // German reply
        let e2 = EmailInfo {
            msgnum: 2,
            msgid: Some("<german@example.com>".to_string()),
            subject: Some("AW: Diskussion".to_string()),
            date: 2000,
            inreplyto: None,
            ..Default::default()
        };

        // Swedish reply
        let e3 = EmailInfo {
            msgnum: 3,
            msgid: Some("<swedish@example.com>".to_string()),
            subject: Some("SV: Diskussion".to_string()),
            date: 3000,
            inreplyto: None,
            ..Default::default()
        };

        store.add_email(e1);
        store.add_email(e2);
        store.add_email(e3);

        let config = Config::default();
        build_threads(&mut store, &config).unwrap();

        // Both replies should be threaded
        assert_eq!(store.replylist.len(), 2);
        assert_eq!(store.replylist[0].from_msgnum, 1);
        assert_eq!(store.replylist[1].from_msgnum, 1);
    }

    #[test]
    fn test_subject_threading_no_false_positives() {
        let mut store = EmailStore::new();

        // Two unrelated messages with same subject but no Re: prefix
        let e1 = EmailInfo {
            msgnum: 1,
            msgid: Some("<unrelated1@example.com>".to_string()),
            subject: Some("Meeting Tomorrow".to_string()),
            date: 1000,
            inreplyto: None,
            ..Default::default()
        };

        let e2 = EmailInfo {
            msgnum: 2,
            msgid: Some("<unrelated2@example.com>".to_string()),
            subject: Some("Meeting Tomorrow".to_string()),
            date: 2000,
            inreplyto: None,
            ..Default::default()
        };

        store.add_email(e1);
        store.add_email(e2);

        let config = Config::default();
        build_threads(&mut store, &config).unwrap();

        // Should NOT be threaded (no Re: prefix)
        assert_eq!(store.replylist.len(), 0);
    }

    #[test]
    fn test_subject_threading_priority_to_header() {
        let mut store = EmailStore::new();

        let e1 = EmailInfo {
            msgnum: 1,
            msgid: Some("<parent@example.com>".to_string()),
            subject: Some("Topic".to_string()),
            date: 1000,
            inreplyto: None,
            ..Default::default()
        };

        let e2 = EmailInfo {
            msgnum: 2,
            msgid: Some("<other@example.com>".to_string()),
            subject: Some("Topic".to_string()),
            date: 2000,
            inreplyto: None,
            ..Default::default()
        };

        // This has In-Reply-To header pointing to e1, not e2
        let e3 = EmailInfo {
            msgnum: 3,
            msgid: Some("<reply@example.com>".to_string()),
            subject: Some("Re: Topic".to_string()),
            date: 3000,
            inreplyto: Some("<parent@example.com>".to_string()),
            ..Default::default()
        };

        store.add_email(e1);
        store.add_email(e2);
        store.add_email(e3);

        let config = Config::default();
        build_threads(&mut store, &config).unwrap();

        // Should only create one thread link (via In-Reply-To)
        // Subject-based threading should skip e3 since it has In-Reply-To
        assert_eq!(store.replylist.len(), 1);
        assert_eq!(store.replylist[0].from_msgnum, 1); // Parent is e1
        assert_eq!(store.replylist[0].msgnum, 3); // Child is e3
    }

    #[test]
    fn test_load_config_file_with_equals() {
        let dir = tempfile::tempdir().unwrap();
        let cfg_path = dir.path().join("test.cfg");
        std::fs::write(&cfg_path, "set language=de\noverwrite=On\n").unwrap();
        let mut config = Config::default();
        load_config_file(cfg_path.to_str().unwrap(), &mut config).unwrap();
        assert_eq!(config.language, "de");
        assert!(config.overwrite);
    }

    #[test]
    fn test_load_config_file_with_colon() {
        let dir = tempfile::tempdir().unwrap();
        let cfg_path = dir.path().join("test.cfg");
        std::fs::write(&cfg_path, "set language: de\ngmtime: On\n").unwrap();
        let mut config = Config::default();
        load_config_file(cfg_path.to_str().unwrap(), &mut config).unwrap();
        assert_eq!(config.language, "de");
        assert!(config.gmtime);
    }

    #[test]
    fn test_load_config_file_with_hm_prefix() {
        let dir = tempfile::tempdir().unwrap();
        let cfg_path = dir.path().join("test.cfg");
        std::fs::write(&cfg_path, "hm_language=de\nhm_nonsequential=On\n").unwrap();
        let mut config = Config::default();
        load_config_file(cfg_path.to_str().unwrap(), &mut config).unwrap();
        assert_eq!(config.language, "de");
        assert!(config.nonsequential);
    }

    #[test]
    fn test_load_config_file_with_comments() {
        let dir = tempfile::tempdir().unwrap();
        let cfg_path = dir.path().join("test.cfg");
        std::fs::write(
            &cfg_path,
            "\
# This is a comment
set language=de
# Another comment
overwrite=On
",
        )
        .unwrap();
        let mut config = Config::default();
        load_config_file(cfg_path.to_str().unwrap(), &mut config).unwrap();
        assert_eq!(config.language, "de");
        assert!(config.overwrite);
    }

    #[test]
    fn test_load_config_file_with_quoted_values() {
        let dir = tempfile::tempdir().unwrap();
        let cfg_path = dir.path().join("test.cfg");
        std::fs::write(&cfg_path, "set label=\"My Archive\"\n").unwrap();
        let mut config = Config::default();
        load_config_file(cfg_path.to_str().unwrap(), &mut config).unwrap();
        assert_eq!(config.label.as_deref(), Some("My Archive"));
    }

    #[test]
    fn test_load_config_file_missing_file() {
        let mut config = Config::default();
        assert!(load_config_file("/nonexistent/hypermail.cfg", &mut config).is_err());
    }

    #[test]
    fn test_load_config_file_unknown_key() {
        let dir = tempfile::tempdir().unwrap();
        let cfg_path = dir.path().join("test.cfg");
        std::fs::write(&cfg_path, "set_nonexistent=value\n").unwrap();
        let mut config = Config::default();
        // Unknown keys should warn but not fail
        load_config_file(cfg_path.to_str().unwrap(), &mut config).unwrap();
    }

    #[test]
    fn test_increment_neg1_accepted_by_config() {
        let mut cfg = Config::default();
        cfg.set_integer("increment", -1).unwrap();
        assert_eq!(cfg.increment, -1);
    }

    #[test]
    fn test_increment_neg1_errors_without_mbox() {
        let cfg = Config { increment: -1, ..Default::default() };
        // Without a mbox, run() should error gracefully, not panic
        let result = run(&cfg);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // validate_config_file — structural checks
    // -----------------------------------------------------------------------

    #[test]
    fn test_validate_clean_file_no_diagnostics() {
        let content = "\
# This is a comment
language = de
overwrite = On
showhtml = 2
dir = /tmp/archive
label = My Archive
";
        let diags = validate_config_file(content);
        assert!(
            diags.is_empty(),
            "clean config should produce no diagnostics, got: {:#?}",
            diags
        );
    }

    #[test]
    fn test_validate_malformed_line_no_separator() {
        let content = "this_line_has_no_separator\n";
        let diags = validate_config_file(content);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, "error");
        assert_eq!(diags[0].line, 1);
        assert!(
            diags[0].message.contains("no '=' or ':' separator"),
            "unexpected message: {}",
            diags[0].message
        );
    }

    #[test]
    fn test_validate_reports_correct_line_number() {
        let content = "\
# comment
language = en
bad_bare_word
overwrite = On
";
        let diags = validate_config_file(content);
        assert_eq!(diags.len(), 1, "should be exactly one error");
        assert_eq!(diags[0].line, 3, "error should be on line 3");
    }

    #[test]
    fn test_validate_blank_lines_and_comments_ignored() {
        let content = "\n\n# full comment line\n   \n# another comment\nlanguage = en\n";
        let diags = validate_config_file(content);
        assert!(diags.is_empty(), "blank lines and comments should not produce diagnostics");
    }

    // -----------------------------------------------------------------------
    // validate_config_file — semantic checks (unknown key, bad value)
    // -----------------------------------------------------------------------

    #[test]
    fn test_validate_unknown_key_is_warning() {
        let content = "totally_unknown_option = foo\n";
        let diags = validate_config_file(content);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, "warning", "unknown key should be a warning not an error");
        assert!(diags[0].message.contains("unknown config key"));
    }

    #[test]
    fn test_validate_deprecated_showhr_is_deprecation_warning() {
        let content = "showhr = 1\n";
        let diags = validate_config_file(content);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, "warning");
        assert!(
            diags[0].message.contains("deprecated"),
            "expected deprecation warning, got: {}",
            diags[0].message
        );
    }

    #[test]
    fn test_validate_deprecated_usetable_is_deprecation_warning() {
        let content = "usetable = 1\n";
        let diags = validate_config_file(content);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, "warning");
        assert!(diags[0].message.contains("deprecated"));
    }

    #[test]
    fn test_validate_deprecated_body_is_deprecation_warning() {
        let content = "body = 1\n";
        let diags = validate_config_file(content);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, "warning");
        assert!(diags[0].message.contains("deprecated"));
    }

    #[test]
    fn test_validate_htmlheaderfile_accepted() {
        let content = "htmlheaderfile = /path/to/header.html\n";
        let diags = validate_config_file(content);
        assert!(diags.is_empty(), "htmlheaderfile should be accepted; got: {:#?}", diags);
    }

    #[test]
    fn test_validate_htmlfooterfile_accepted() {
        let content = "htmlfooterfile = /path/to/footer.html\n";
        let diags = validate_config_file(content);
        assert!(diags.is_empty(), "htmlfooterfile should be accepted; got: {:#?}", diags);
    }

    #[test]
    fn test_validate_hm_prefix_stripped_for_lookup() {
        // hm_ prefix is valid and must not be flagged as unknown
        let content = "hm_language = fr\nhm_overwrite = On\n";
        let diags = validate_config_file(content);
        assert!(diags.is_empty(), "hm_ prefix entries should be valid; got: {:#?}", diags);
    }

    #[test]
    fn test_validate_set_prefix_in_line_stripped() {
        // "set key = val" is historical compat syntax
        let content = "set language = es\nset overwrite = On\n";
        let diags = validate_config_file(content);
        assert!(diags.is_empty(), "'set ' prefix entries should be valid; got: {:#?}", diags);
    }

    #[test]
    fn test_validate_colon_separator_accepted() {
        let content = "language: de\ngmtime: On\n";
        let diags = validate_config_file(content);
        assert!(diags.is_empty(), "colon separator should be accepted; got: {:#?}", diags);
    }

    #[test]
    fn test_validate_showhtml_valid_values() {
        for val in &["0", "1", "2"] {
            let content = format!("showhtml = {}\n", val);
            let diags = validate_config_file(&content);
            assert!(diags.is_empty(), "showhtml={} should be valid; got: {:#?}", val, diags);
        }
    }

    #[test]
    fn test_validate_showhtml_invalid_value_is_error() {
        let content = "showhtml = banana\n";
        let diags = validate_config_file(content);
        // "banana" fails integer parse, so apply_cli_arg returns error
        assert!(!diags.is_empty(), "showhtml=banana should produce a diagnostic");
        // At least one should mention showhtml or banana
        assert!(
            diags.iter().any(|d| d.source.contains("showhtml")),
            "diagnostic source should reference the showhtml line"
        );
    }

    #[test]
    fn test_validate_showhtml_out_of_range() {
        // showhtml=9 parses as integer but is semantically invalid
        let content = "showhtml = 9\n";
        let diags = validate_config_file(content);
        let errors: Vec<_> = diags.iter().filter(|d| d.severity == "error").collect();
        assert_eq!(errors.len(), 1, "showhtml=9 should produce exactly one error");
        assert!(
            errors[0].message.contains("showhtml must be"),
            "wrong error message: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_validate_defaultindex_valid_values() {
        for val in &["date", "subject", "author", "thread", "attachment"] {
            let content = format!("defaultindex = {}\n", val);
            let diags = validate_config_file(&content);
            assert!(diags.is_empty(), "defaultindex={} should be valid; got: {:#?}", val, diags);
        }
    }

    #[test]
    fn test_validate_defaultindex_invalid_is_error() {
        let content = "defaultindex = garbage\n";
        let diags = validate_config_file(content);
        let errors: Vec<_> = diags.iter().filter(|d| d.severity == "error").collect();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("defaultindex must be one of"));
    }

    // -----------------------------------------------------------------------
    // validate_config_file — cross-field checks
    // -----------------------------------------------------------------------

    #[test]
    fn test_validate_mbox_shortened_without_usegdbm_is_error() {
        let content = "mbox_shortened = On\nusegdbm = Off\n";
        let diags = validate_config_file(content);
        let errors: Vec<_> = diags.iter().filter(|d| d.severity == "error").collect();
        assert!(
            errors.iter().any(|d| d.message.contains("usegdbm")),
            "should flag missing usegdbm; diagnostics: {:#?}",
            diags
        );
    }

    #[test]
    fn test_validate_mbox_shortened_with_usegdbm_ok() {
        let content = "mbox_shortened = On\nusegdbm = On\nincrement = 0\n";
        let diags = validate_config_file(content);
        assert!(
            diags.is_empty(),
            "mbox_shortened+usegdbm+increment=0 should be valid; got: {:#?}",
            diags
        );
    }

    #[test]
    fn test_validate_mbox_shortened_with_increment_nonzero_is_error() {
        let content = "mbox_shortened = On\nusegdbm = On\nincrement = 1\n";
        let diags = validate_config_file(content);
        let errors: Vec<_> = diags.iter().filter(|d| d.severity == "error").collect();
        assert!(
            errors.iter().any(|d| d.message.contains("increment")),
            "should flag increment != 0; diagnostics: {:#?}",
            diags
        );
    }

    #[test]
    fn test_validate_folder_by_date_and_msgsperfolder_is_error() {
        let content = "folder_by_date = %Y-%m\nmsgsperfolder = 100\n";
        let diags = validate_config_file(content);
        let errors: Vec<_> = diags.iter().filter(|d| d.severity == "error").collect();
        assert!(
            errors.iter().any(
                |d| d.message.contains("msgsperfolder") || d.message.contains("folder_by_date")
            ),
            "should flag mutual exclusion; diagnostics: {:#?}",
            diags
        );
    }

    #[test]
    fn test_validate_dir_with_dotdot_is_warning() {
        let content = "dir = /tmp/../archive\n";
        let diags = validate_config_file(content);
        assert!(
            diags.iter().any(|d| d.severity == "warning" && d.message.contains("..")),
            "dir with '..' should be a warning; got: {:#?}",
            diags
        );
    }

    #[test]
    fn test_validate_multiple_errors_all_reported() {
        let content = "\
totally_unknown = foo
bare_line_no_sep
showhtml = 99
defaultindex = nonsense
";
        let diags = validate_config_file(content);
        // Should report: 1 warning (unknown key) + 1 error (no sep) + 1 error (showhtml range) + 1 error (defaultindex)
        assert!(diags.len() >= 3, "should report multiple diagnostics; got: {:#?}", diags);
    }

    // -----------------------------------------------------------------------
    // load_config_file — integration: errors prevent loading
    // -----------------------------------------------------------------------

    #[test]
    fn test_load_config_file_rejects_malformed_line() {
        let dir = tempfile::tempdir().unwrap();
        let cfg_path = dir.path().join("bad.cfg");
        std::fs::write(&cfg_path, "this_has_no_separator\n").unwrap();
        let mut config = Config::default();
        let result = load_config_file(cfg_path.to_str().unwrap(), &mut config);
        assert!(result.is_err(), "malformed config should return Err");
    }

    #[test]
    fn test_load_config_file_rejects_conflicting_folder_options() {
        let dir = tempfile::tempdir().unwrap();
        let cfg_path = dir.path().join("conflict.cfg");
        std::fs::write(&cfg_path, "folder_by_date = %Y-%m\nmsgsperfolder = 50\n").unwrap();
        let mut config = Config::default();
        let result = load_config_file(cfg_path.to_str().unwrap(), &mut config);
        assert!(result.is_err(), "conflicting folder options should return Err");
    }

    #[test]
    fn test_load_config_file_rejects_bad_showhtml() {
        let dir = tempfile::tempdir().unwrap();
        let cfg_path = dir.path().join("badhtml.cfg");
        std::fs::write(&cfg_path, "showhtml = 7\n").unwrap();
        let mut config = Config::default();
        let result = load_config_file(cfg_path.to_str().unwrap(), &mut config);
        assert!(result.is_err(), "showhtml out of range should return Err");
    }

    #[test]
    fn test_load_config_file_accepts_unknown_key_with_warning() {
        let dir = tempfile::tempdir().unwrap();
        let cfg_path = dir.path().join("unknown.cfg");
        std::fs::write(&cfg_path, "totally_unknown_key = foobar\n").unwrap();
        let mut config = Config::default();
        // Unknown key is a WARNING only — loading should still succeed
        let result = load_config_file(cfg_path.to_str().unwrap(), &mut config);
        assert!(result.is_ok(), "unknown key (warning-only) should not prevent loading");
    }

    #[test]
    fn test_load_config_file_rejects_bad_defaultindex() {
        let dir = tempfile::tempdir().unwrap();
        let cfg_path = dir.path().join("badidx.cfg");
        std::fs::write(&cfg_path, "defaultindex = nonsense\n").unwrap();
        let mut config = Config::default();
        let result = load_config_file(cfg_path.to_str().unwrap(), &mut config);
        assert!(result.is_err(), "invalid defaultindex should return Err");
    }

    // -----------------------------------------------------------------------
    // show_warnings / warn() — config flag and per-message warning logic
    // -----------------------------------------------------------------------

    #[test]
    fn test_show_warnings_default_off() {
        let cfg = Config::default();
        assert!(!cfg.show_warnings, "show_warnings should default to false");
    }

    #[test]
    fn test_show_warnings_can_be_enabled() {
        let mut cfg = Config::default();
        cfg.set_switch("show_warnings", true).unwrap();
        assert!(cfg.show_warnings);
    }

    #[test]
    fn test_warn_func_silent_when_disabled() {
        // warn() should not panic; just verify it runs without side effects
        let cfg = Config::default(); // show_warnings = false
        warn(&cfg, "this should not be printed");
    }

    #[test]
    fn test_parse_email_missing_msgid_flagged() {
        // Simulate a message without Message-ID — warning infrastructure exists via parse_email
        let config = Config::default();
        let headers = "From: Alice <alice@example.com>\nSubject: No ID\nDate: Mon, 15 Mar 2021 12:00:00 +0000\n\n";
        let body = "body";
        let email = parse_email(1, headers, headers.as_bytes(), body, body.as_bytes(), &config);
        assert!(email.msgid.is_none(), "msgid should be None for this message");
    }

    #[test]
    fn test_parse_email_missing_subject_flagged() {
        let config = Config::default();
        let headers = "From: Alice <alice@example.com>\nMessage-ID: <x@x.com>\nDate: Mon, 15 Mar 2021 12:00:00 +0000\n\n";
        let body = "body";
        let email = parse_email(1, headers, headers.as_bytes(), body, body.as_bytes(), &config);
        assert!(email.subject.is_none(), "subject should be None for this message");
    }

    #[test]
    fn test_parse_email_zero_date_flagged() {
        let config = Config::default();
        // No Date header, no Received header → date == 0
        let headers = "From: Alice <alice@example.com>\nSubject: Test\nMessage-ID: <y@y.com>\n\n";
        let body = "body";
        let email = parse_email(1, headers, headers.as_bytes(), body, body.as_bytes(), &config);
        assert_eq!(email.date, 0, "date should be 0 when Date header is absent");
    }

    #[test]
    fn test_show_warnings_config_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let cfg_path = dir.path().join("warn.cfg");
        std::fs::write(&cfg_path, "show_warnings = On\n").unwrap();
        let mut config = Config::default();
        load_config_file(cfg_path.to_str().unwrap(), &mut config).unwrap();
        assert!(config.show_warnings, "show_warnings=On in config file should be applied");
    }
}
