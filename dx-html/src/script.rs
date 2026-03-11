//! Inline-script helpers built on top of `serde_json`.
//!
//! The primary goal is to serialize values as JSON-backed JavaScript literals
//! and then make that JSON safe for embedding inside HTML `<script>` tags.

use serde::Serialize;

/// Serialize a value as a JavaScript literal using JSON encoding and then make
/// it safe for embedding inside an HTML `<script>` tag.
pub fn to_js_literal<T>(value: &T) -> Result<String, serde_json::Error>
where
    T: Serialize,
{
    let raw = serde_json::to_string(value)?;
    Ok(make_html_safe_script_json(&raw))
}

/// Serialize a string as a quoted JavaScript string literal safe for HTML `<script>`.
pub fn to_js_string_literal(value: &str) -> Result<String, serde_json::Error> {
    to_js_literal(&value)
}

/// Write `name = <json literal>;` into the output buffer.
pub fn push_js_assignment<T>(
    out: &mut String,
    name: &str,
    value: &T,
) -> Result<(), serde_json::Error>
where
    T: Serialize,
{
    out.push_str(name);
    out.push_str(" = ");
    out.push_str(&to_js_literal(value)?);
    out.push(';');
    Ok(())
}

/// Append a JSON `<script>` tag such as:
/// `<script id="boot-data" type="application/json">{"ok":true}</script>`
pub fn push_json_script_tag<T>(
    out: &mut String,
    id: Option<&str>,
    value: &T,
) -> Result<(), serde_json::Error>
where
    T: Serialize,
{
    out.push_str("<script");
    if let Some(id) = id.map(str::trim).filter(|id| !id.is_empty()) {
        out.push_str(" id=\"");
        out.push_str(&html_escape::encode_double_quoted_attribute(id));
        out.push('"');
    }
    out.push_str(" type=\"application/json\">");
    out.push_str(&to_js_literal(value)?);
    out.push_str("</script>");
    Ok(())
}

/// Make a JSON string safe to embed inside an HTML `<script>` block.
///
/// This preserves valid JSON while escaping characters and separators that can
/// interact badly with HTML parsing or legacy JS parsers.
#[must_use]
pub fn make_html_safe_script_json(raw_json: &str) -> String {
    let mut out = String::with_capacity(raw_json.len());

    for ch in raw_json.chars() {
        match ch {
            '<' => out.push_str("\\u003C"),
            '>' => out.push_str("\\u003E"),
            '&' => out.push_str("\\u0026"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            _ => out.push(ch),
        }
    }

    out
}
