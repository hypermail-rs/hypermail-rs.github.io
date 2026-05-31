use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HypermailError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Date parse error: {0}")]
    DateParse(String),

    #[error("Mbox parse error at line {line}: {message}")]
    MboxParse { line: usize, message: String },

    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    #[error("Invalid config value for '{key}': {message}")]
    InvalidConfigValue { key: String, message: String },

    #[error("Lock error: {0}")]
    Lock(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, HypermailError>;
