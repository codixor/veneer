//! Escaping helpers built on top of `html_escape`.
//!
//! Use text escaping for content between tags and attribute escaping for quoted
//! attribute values.

use std::borrow::Cow;
use std::fmt::{self, Display, Write};

/// A display wrapper that escapes HTML text-node content.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EscapedText<'a>(pub &'a str);

impl Display for EscapedText<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&html_escape::encode_text(self.0))
    }
}

/// A display wrapper that escapes a value for a double-quoted HTML attribute.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EscapedAttr<'a>(pub &'a str);

impl Display for EscapedAttr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&html_escape::encode_double_quoted_attribute(self.0))
    }
}

/// Escape content for use inside a normal HTML text node.
#[must_use]
pub fn escape_text(value: &str) -> Cow<'_, str> {
    html_escape::encode_text(value)
}

/// Escape content for use inside a double-quoted HTML attribute.
#[must_use]
pub fn escape_attr(value: &str) -> Cow<'_, str> {
    html_escape::encode_double_quoted_attribute(value)
}

/// Push escaped text-node content into an output buffer.
pub fn push_escaped_text(out: &mut String, value: &str) {
    out.push_str(&escape_text(value));
}

/// Push escaped attribute content into an output buffer.
pub fn push_escaped_attr(out: &mut String, value: &str) {
    out.push_str(&escape_attr(value));
}

/// Write escaped text-node content into any `fmt::Write` sink.
pub fn write_escaped_text(out: &mut impl Write, value: &str) -> fmt::Result {
    write!(out, "{}", EscapedText(value))
}

/// Write escaped attribute content into any `fmt::Write` sink.
pub fn write_escaped_attr(out: &mut impl Write, value: &str) -> fmt::Result {
    write!(out, "{}", EscapedAttr(value))
}

/// Append ` name="value"` when the value is non-empty after trimming.
pub fn push_optional_attr(out: &mut String, name: &str, value: Option<&str>) {
    let Some(value) = normalized_attr_value(value) else {
        return;
    };

    out.push(' ');
    out.push_str(name);
    out.push_str("=\"");
    push_escaped_attr(out, value);
    out.push('"');
}

/// Write ` name="value"` when the value is non-empty after trimming.
pub fn write_optional_attr(out: &mut impl Write, name: &str, value: Option<&str>) -> fmt::Result {
    let Some(value) = normalized_attr_value(value) else {
        return Ok(());
    };

    out.write_char(' ')?;
    out.write_str(name)?;
    out.write_str("=\"")?;
    write_escaped_attr(out, value)?;
    out.write_char('"')
}

/// Write a boolean HTML attribute such as `hidden` or `disabled` when enabled.
pub fn write_bool_attr(out: &mut impl Write, name: &str, enabled: bool) -> fmt::Result {
    if enabled {
        out.write_char(' ')?;
        out.write_str(name)?;
    }
    Ok(())
}

/// Write a `data-*` attribute when the value is non-empty after trimming.
pub fn write_data_attr(out: &mut impl Write, suffix: &str, value: Option<&str>) -> fmt::Result {
    if suffix.trim().is_empty() {
        return Ok(());
    }

    let mut name = String::with_capacity(5 + suffix.len());
    name.push_str("data-");
    name.push_str(suffix);
    write_optional_attr(out, &name, value)
}

fn normalized_attr_value(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}
