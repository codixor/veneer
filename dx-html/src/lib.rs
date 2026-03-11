//! `dx-html` provides a small framework-oriented surface for:
//! - context-aware HTML escaping
//! - sanitizing untrusted HTML fragments
//! - safely embedding JSON / JavaScript values into inline scripts
//!
//! The crate is intentionally small but aims to be suitable for framework and
//! runtime internals where correctness and explicit trust boundaries matter.

pub mod escape;
pub mod sanitize;
pub mod script;

pub use escape::{EscapedAttr, EscapedText};
pub use sanitize::{sanitize_html, sanitize_with, SanitizedHtml, SanitizerPreset};
pub use script::{push_js_assignment, push_json_script_tag, to_js_literal, to_js_string_literal};
