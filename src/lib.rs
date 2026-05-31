//! hypermail-rs: A Rust port of hypermail, converting mbox email archives to HTML.
//!
//! This library provides parsing of mbox files, MIME decoding, HTML generation,
//! index creation (by date, subject, author, thread), i18n support, and
//! configurable template-based output.

// Allow field reassignment with default in tests - Config has 109 fields
#![allow(clippy::field_reassign_with_default)]

pub mod config;
pub mod date;
pub mod error;
pub mod file_utils;
pub mod filter;
pub mod gdbm;
pub mod haof;
pub mod headers;
pub mod html;
pub mod i18n;
pub mod index;
pub mod link;
pub mod mbox;
pub mod message;
pub mod mime;
pub mod quotes;
pub mod search;
pub mod string_utils;
pub mod structs;
pub mod templates;
pub mod txt2html;
