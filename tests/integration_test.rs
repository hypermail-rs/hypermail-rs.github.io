use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn hypermail_binary() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/hypermail");
    if cfg!(windows) {
        path.set_extension("exe");
    }
    path
}

fn mbox_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/test_data/simple.mbox")
}

/// Creates a temp directory and returns its path. Caller must clean up.
fn temp_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("tests/tmp_{}", name));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

// ══════════════════════════════════════════════════════════════════════════════
// CLI parameter tests — one test per flag
// ══════════════════════════════════════════════════════════════════════════════

// -h / --help
#[test]
fn test_cli_help() {
    let (stdout, _) = run_ok(&["--help"]);
    assert!(stdout.contains("Usage:"), "--help should show usage");
    assert!(stdout.contains("--mbox"), "--help should list --mbox");
}

// -V / --version
#[test]
fn test_cli_version() {
    let (stdout, _) = run_ok(&["--version"]);
    assert!(stdout.contains("1.0"), "--version should print version");
}

// -v / --verbose: print config and exit
#[test]
fn test_cli_verbose() {
    let mbox = mbox_path();
    let (stdout, _) = run_ok(&["-m", mbox.to_str().unwrap(), "-v"]);
    assert!(stdout.contains("language"), "-v should dump config");
}

// -m / --mbox: specify mailbox file
#[test]
fn test_cli_mbox() {
    let dir = temp_dir("cli_mbox");
    let mbox = mbox_path();
    run_ok(&["-m", mbox.to_str().unwrap(), "-d", dir.to_str().unwrap(), "-x"]);
    assert!(dir.join("index.html").exists());
    let _ = fs::remove_dir_all(&dir);
}

// -d / --dir: specify output directory
#[test]
fn test_cli_dir() {
    let dir = temp_dir("cli_dir");
    let sub = dir.join("subdir");
    let mbox = mbox_path();
    run_ok(&["-m", mbox.to_str().unwrap(), "-d", sub.to_str().unwrap(), "-x"]);
    assert!(sub.join("index.html").exists(), "-d should create output in specified dir");
    let _ = fs::remove_dir_all(&dir);
}

// -l / --label: archive label
#[test]
fn test_cli_label() {
    let dir = temp_dir("cli_label");
    let mbox = mbox_path();
    run_ok(&[
        "-m",
        mbox.to_str().unwrap(),
        "-d",
        dir.to_str().unwrap(),
        "-x",
        "-l",
        "TestArchive",
    ]);
    let index = fs::read_to_string(dir.join("index.html")).unwrap();
    assert!(index.contains("TestArchive"), "-l label should appear in index");
    let _ = fs::remove_dir_all(&dir);
}

// -L / --language: set language code
#[test]
fn test_cli_language() {
    let dir = temp_dir("cli_lang");
    let mbox = mbox_path();
    run_ok(&["-m", mbox.to_str().unwrap(), "-d", dir.to_str().unwrap(), "-x", "-L", "de"]);
    let index = fs::read_to_string(dir.join("index.html")).unwrap();
    assert!(index.contains("lang=\"de\""), "-L should set HTML lang attribute");
    let _ = fs::remove_dir_all(&dir);
}

// -s / --suffix: HTML file suffix
#[test]
fn test_cli_suffix() {
    let dir = temp_dir("cli_suffix");
    let mbox = mbox_path();
    run_ok(&["-m", mbox.to_str().unwrap(), "-d", dir.to_str().unwrap(), "-x", "-s", "htm"]);
    assert!(dir.join("index.htm").exists(), "-s should change file extension");
    assert!(!dir.join("index.html").exists());
    let _ = fs::remove_dir_all(&dir);
}

// -x / --overwrite: overwrite existing archive
#[test]
fn test_cli_overwrite() {
    let dir = temp_dir("cli_overwrite");
    let mbox = mbox_path();
    // First run creates archive
    run_ok(&["-m", mbox.to_str().unwrap(), "-d", dir.to_str().unwrap(), "-x"]);
    assert!(dir.join("index.html").exists());

    // Second run without -x should fail (archive exists)
    let (_stdout, stderr) = run_fail(&["-m", mbox.to_str().unwrap(), "-d", dir.to_str().unwrap()]);
    assert!(
        stderr.contains("already exists") || stderr.contains("overwrite"),
        "should refuse without -x: {}",
        stderr
    );

    // Third run with -x should succeed
    run_ok(&["-m", mbox.to_str().unwrap(), "-d", dir.to_str().unwrap(), "-x"]);
    let _ = fs::remove_dir_all(&dir);
}

// -u / --update: incremental update
#[test]
fn test_cli_update() {
    let dir = temp_dir("cli_update");
    let mbox = mbox_path();
    // Initial archive
    run_ok(&["-m", mbox.to_str().unwrap(), "-d", dir.to_str().unwrap(), "-x"]);
    let count1 = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "html"))
        .count();

    // Incremental update with same mbox (should not crash)
    run_ok(&["-m", mbox.to_str().unwrap(), "-d", dir.to_str().unwrap(), "-u"]);
    let count2 = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "html"))
        .count();
    assert!(count2 >= count1, "-u should preserve or add articles");
    let _ = fs::remove_dir_all(&dir);
}

// -p / --progress: show progress bar on stderr
#[test]
fn test_cli_progress() {
    let dir = temp_dir("cli_progress");
    let mbox = mbox_path();
    let (_, stderr) =
        run_ok(&["-m", mbox.to_str().unwrap(), "-d", dir.to_str().unwrap(), "-x", "-p"]);
    assert!(stderr.contains("Reading"), "-p should show reading progress");
    assert!(stderr.contains("Writing"), "-p should show writing progress");
    assert!(stderr.contains("100%"), "-p should reach 100%");
    let _ = fs::remove_dir_all(&dir);
}

// -1 / --readone: only read first message
#[test]
fn test_cli_readone() {
    let dir = temp_dir("cli_readone");
    let mbox = mbox_path();
    run_ok(&["-m", mbox.to_str().unwrap(), "-d", dir.to_str().unwrap(), "-x", "-1"]);
    // Should produce exactly 1 message article + index files
    let article_files: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.ends_with(".html")
                && name != "index.html"
                && name != "date.html"
                && name != "subject.html"
                && name != "author.html"
                && name != "thread.html"
                && name != "threads.html"
                && name != "attachment.html"
        })
        .collect();
    assert_eq!(
        article_files.len(),
        1,
        "-1 should produce exactly 1 article, got {}",
        article_files.len()
    );
    let _ = fs::remove_dir_all(&dir);
}

// -N / --nonsequential: use hash-based filenames
#[test]
fn test_cli_nonsequential() {
    let dir = temp_dir("cli_nonseq");
    let mbox = mbox_path();
    run_ok(&["-m", mbox.to_str().unwrap(), "-d", dir.to_str().unwrap(), "-x", "-N"]);
    // Nonsequential mode should NOT produce 0000.html, 0001.html etc.
    assert!(!dir.join("0000.html").exists(), "-N should use hash filenames, not sequential");
    assert!(dir.join("index.html").exists());
    let _ = fs::remove_dir_all(&dir);
}

// -c / --config: load config file
#[test]
fn test_cli_config() {
    let dir = temp_dir("cli_config");
    let mbox = mbox_path();
    let cfg_path = dir.join("test.hmrc");
    fs::write(
        &cfg_path,
        format!(
            "mbox = {}\ndir = {}\noverwrite = On\nlabel = ConfigLabel\n",
            mbox.to_str().unwrap(),
            dir.to_str().unwrap()
        ),
    )
    .unwrap();
    run_ok(&["-c", cfg_path.to_str().unwrap()]);
    let index = fs::read_to_string(dir.join("index.html")).unwrap();
    assert!(index.contains("ConfigLabel"), "-c should load config with label");
    let _ = fs::remove_dir_all(&dir);
}

// -o / --set: override config items
#[test]
fn test_cli_set_option() {
    let dir = temp_dir("cli_set");
    let mbox = mbox_path();
    run_ok(&[
        "-m",
        mbox.to_str().unwrap(),
        "-d",
        dir.to_str().unwrap(),
        "-x",
        "-o",
        "label=OverrideLabel",
    ]);
    let index = fs::read_to_string(dir.join("index.html")).unwrap();
    assert!(index.contains("OverrideLabel"), "-o should override label");
    let _ = fs::remove_dir_all(&dir);
}

// -a / --archives: set other archives URL (used in external templates)
#[test]
fn test_cli_archives() {
    let dir = temp_dir("cli_archives");
    let mbox = mbox_path();
    // -a is accepted without error; the URL is used by external printfile templates
    run_ok(&[
        "-m",
        mbox.to_str().unwrap(),
        "-d",
        dir.to_str().unwrap(),
        "-x",
        "-a",
        "http://example.com/archives",
    ]);
    assert!(dir.join("index.html").exists());
    let _ = fs::remove_dir_all(&dir);
}

// -b / --about: set about URL (used in external templates)
#[test]
fn test_cli_about() {
    let dir = temp_dir("cli_about");
    let mbox = mbox_path();
    // -b is accepted without error; the URL is used by external printfile templates
    run_ok(&[
        "-m",
        mbox.to_str().unwrap(),
        "-d",
        dir.to_str().unwrap(),
        "-x",
        "-b",
        "http://example.com/about",
    ]);
    assert!(dir.join("index.html").exists());
    let _ = fs::remove_dir_all(&dir);
}

// -n / --hmail: set submission address
#[test]
fn test_cli_hmail() {
    let dir = temp_dir("cli_hmail");
    let mbox = mbox_path();
    run_ok(&[
        "-m",
        mbox.to_str().unwrap(),
        "-d",
        dir.to_str().unwrap(),
        "-x",
        "-n",
        "list@example.com",
    ]);
    let article = fs::read_to_string(dir.join("0000.html")).unwrap();
    assert!(article.contains("list@example.com"), "-n hmail should appear in article pages");
    let _ = fs::remove_dir_all(&dir);
}

// -X / --xml: write HAOF XML
#[test]
fn test_cli_xml() {
    let dir = temp_dir("cli_xml");
    let mbox = mbox_path();
    run_ok(&["-m", mbox.to_str().unwrap(), "-d", dir.to_str().unwrap(), "-x", "-X"]);
    let haof = dir.join("haof.xml");
    assert!(haof.exists(), "-X should create haof.xml");
    let content = fs::read_to_string(&haof).unwrap();
    assert!(content.contains("<?xml"), "haof.xml should be valid XML");
    let _ = fs::remove_dir_all(&dir);
}

// -g / --gdbm: use GDBM cache
#[test]
fn test_cli_gdbm() {
    let dir = temp_dir("cli_gdbm");
    let mbox = mbox_path();
    run_ok(&["-m", mbox.to_str().unwrap(), "-d", dir.to_str().unwrap(), "-x", "-g"]);
    let gdbm = dir.join(".hm2index");
    assert!(gdbm.exists(), "-g should create .hm2index cache file");
    let _ = fs::remove_dir_all(&dir);
}

// -T / --indextables: use table-based index layout
#[test]
fn test_cli_indextables() {
    let dir = temp_dir("cli_indextbl");
    let mbox = mbox_path();
    run_ok(&["-m", mbox.to_str().unwrap(), "-d", dir.to_str().unwrap(), "-x", "-T"]);
    let index = fs::read_to_string(dir.join("index.html")).unwrap();
    assert!(index.contains("<table"), "-T should produce table-based index");
    let _ = fs::remove_dir_all(&dir);
}

// -i / --stdin: read from stdin
#[test]
fn test_cli_stdin() {
    let dir = temp_dir("cli_stdin");
    let mbox = mbox_path();
    let mbox_content = fs::read(&mbox).unwrap();

    let output = Command::new(hypermail_binary())
        .args(["-i", "-d", dir.to_str().unwrap(), "-x"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(&mbox_content).unwrap();
            child.wait_with_output()
        })
        .expect("failed to run with stdin");

    assert!(
        output.status.success(),
        "stdin read failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(dir.join("index.html").exists(), "-i should produce output from stdin");
    let _ = fs::remove_dir_all(&dir);
}

// -A / --append: append mode
#[test]
fn test_cli_append() {
    let dir = temp_dir("cli_append");
    let mbox = mbox_path();
    run_ok(&["-m", mbox.to_str().unwrap(), "-d", dir.to_str().unwrap(), "-x", "-A"]);
    assert!(dir.join("index.html").exists());
    let _ = fs::remove_dir_all(&dir);
}

// -M / --metadata: flag is accepted without error
#[test]
fn test_cli_metadata() {
    let dir = temp_dir("cli_meta");
    let mbox = mbox_path();
    run_ok(&["-m", mbox.to_str().unwrap(), "-d", dir.to_str().unwrap(), "-x", "-M"]);
    assert!(dir.join("index.html").exists());
    let _ = fs::remove_dir_all(&dir);
}

// -t / --tables: deprecated flag accepted without error
#[test]
fn test_cli_tables_deprecated() {
    let dir = temp_dir("cli_tables");
    let mbox = mbox_path();
    run_ok(&["-m", mbox.to_str().unwrap(), "-d", dir.to_str().unwrap(), "-x", "-t"]);
    assert!(dir.join("index.html").exists());
    let _ = fs::remove_dir_all(&dir);
}

// No mbox specified → error
#[test]
fn test_cli_no_mbox_error() {
    let dir = temp_dir("cli_nombox");
    let (_, stderr) = run_fail(&["-d", dir.to_str().unwrap(), "-x"]);
    assert!(
        stderr.contains("No mailbox") || stderr.contains("mbox"),
        "should error without mbox: {}",
        stderr
    );
    let _ = fs::remove_dir_all(&dir);
}

/// Run hypermail with given args, assert success, return (stdout, stderr).
fn run_ok(args: &[&str]) -> (String, String) {
    let output = Command::new(hypermail_binary())
        .args(args)
        .output()
        .expect("failed to execute hypermail");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(
        output.status.success(),
        "hypermail failed with args {:?}\nstdout: {}\nstderr: {}",
        args,
        stdout,
        stderr
    );
    (stdout, stderr)
}

/// Run hypermail with given args, expect failure.
fn run_fail(args: &[&str]) -> (String, String) {
    let output = Command::new(hypermail_binary())
        .args(args)
        .output()
        .expect("failed to execute hypermail");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(
        !output.status.success(),
        "hypermail should have failed with args {:?}\nstdout: {}\nstderr: {}",
        args,
        stdout,
        stderr
    );
    (stdout, stderr)
}

#[test]
fn test_simple_mbox_parse() {
    let mbox = mbox_path();
    assert!(mbox.exists(), "test mbox file does not exist");

    let file = fs::File::open(&mbox).unwrap();
    let reader = std::io::BufReader::new(file);
    let mbox_reader = hypermail::mbox::MboxReader::new(reader, hypermail::mbox::MboxFormat::MboxO);
    let mut count = 0;
    for result in mbox_reader {
        let msg = result.unwrap();
        count += 1;
        let headers = String::from_utf8_lossy(&msg.headers);
        let body = String::from_utf8_lossy(&msg.body);
        match count {
            1 => {
                assert!(headers.contains("Message-ID: <msg001@example.com>"));
                assert!(body.contains("This is the first test message."));
            },
            2 => {
                assert!(headers.contains("Message-ID: <msg002@example.com>"));
                assert!(body.contains("This is a reply to Alice."));
            },
            3 => {
                assert!(headers.contains("Message-ID: <msg003@example.com>"));
                assert!(body.contains("Line with"));
            },
            _ => panic!("unexpected message count"),
        }
    }
    assert_eq!(count, 3, "expected 3 messages in mbox");
}

#[test]
fn test_end_to_end_archive() {
    let dir = temp_dir("e2e");
    let mbox = mbox_path();

    run_ok(&["-m", mbox.to_str().unwrap(), "-d", dir.to_str().unwrap(), "-x"]);

    let html_files: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "html"))
        .collect();
    assert!(
        html_files.len() >= 3,
        "expected at least 3 HTML files, got {}",
        html_files.len()
    );

    let index_path = dir.join("index.html");
    assert!(index_path.exists(), "index.html was not generated");
    let index_content = fs::read_to_string(&index_path).unwrap();
    assert!(index_content.contains("Alice"), "index should contain Alice");
    assert!(index_content.contains("Bob"), "index should contain Bob");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_parse_email_with_deleted_header() {
    let _config = hypermail::config::Config::default();
    let headers = "From: Test <test@e.com>\nSubject: Test\nX-Hypermail-Deleted: yes\nMessage-ID: <del@e.com>\nDate: Mon, 15 Mar 2021 12:00:00 +0000\n\n";
    let _body = "Body text";
    let parsed = hypermail::headers::parse_headers(headers.as_bytes());
    let from = hypermail::headers::find_header(&parsed, "From").unwrap_or_default();
    let (name, _email) = hypermail::headers::parse_email_address(from);
    assert_eq!(name, Some("Test".to_string()));
}

#[test]
fn test_gdbm_roundtrip() {
    let mut config = hypermail::config::Config::default();
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/test_gdbm_out");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    config.dir = Some(dir.to_str().unwrap().to_string());

    let mut store = hypermail::structs::EmailStore::new();
    let email = hypermail::message::EmailInfo {
        msgnum: 1,
        name: Some("Alice".to_string()),
        email_addr: Some("alice@e.com".to_string()),
        subject: Some("Hello".to_string()),
        msgid: Some("<a@b>".to_string()),
        from_date_str: Some("Mon, 1 Jan 2024 12:00:00 +0000".to_string()),
        date_str: Some("Mon, 1 Jan 2024 12:00:00 +0000".to_string()),
        date: 1704110400,
        from_date: 1704110400,
        ..Default::default()
    };
    store.add_email(email);

    hypermail::gdbm::togdbm(&store, &config).unwrap();

    let mut loaded = hypermail::structs::EmailStore::new();
    let count = hypermail::gdbm::load_from_gdbm(&mut loaded, &config).unwrap();
    assert!(count > 0);
    assert_eq!(loaded.emails[0].name.as_deref(), Some("Alice"));
    assert_eq!(loaded.emails[0].subject.as_deref(), Some("Hello"));

    let _ = fs::remove_dir_all(&dir);
}
