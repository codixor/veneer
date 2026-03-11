//! Shared utilities used across modules.

use std::borrow::Cow;

use super::config::VendorCompatCfg;

pub(crate) fn format_css_number(value: f64) -> String {
    if value.fract().abs() <= f64::EPSILON {
        format!("{}", value as i64)
    } else {
        let mut s = format!("{value}");
        if s.contains('.') {
            while s.ends_with('0') {
                s.pop();
            }
            if s.ends_with('.') {
                s.pop();
            }
        }
        s
    }
}

pub(crate) fn normalize_css_ident(raw: &str) -> String {
    let chars: Vec<char> = raw.trim().chars().collect();
    if chars.is_empty() {
        return String::new();
    }
    let mut out = String::with_capacity(chars.len() + 8);

    let push_dash = |buf: &mut String| {
        if !buf.is_empty() && !buf.ends_with('-') {
            buf.push('-');
        }
    };

    for (idx, ch) in chars.iter().enumerate() {
        let prev = idx.checked_sub(1).and_then(|i| chars.get(i)).copied();
        let next = chars.get(idx + 1).copied();
        match ch {
            'A'..='Z' => {
                if let Some(prev) = prev
                    && ((prev.is_ascii_lowercase() || prev.is_ascii_digit())
                        || (prev.is_ascii_uppercase()
                            && next
                                .map(|v| v.is_ascii_lowercase() || v.is_ascii_digit())
                                .unwrap_or(false)))
                {
                    push_dash(&mut out);
                }
                out.push(ch.to_ascii_lowercase());
            }
            'a'..='z' => out.push(*ch),
            '0'..='9' => {
                if let Some(prev) = prev
                    && prev.is_ascii_lowercase()
                {
                    push_dash(&mut out);
                }
                out.push(*ch);
            }
            _ => push_dash(&mut out),
        }
    }

    out.trim_matches('-').to_string()
}

/// Normalize a CSS property key:
/// - if key starts with `--`:
///   - when `normalize_explicit_custom_props=true`, normalize the remainder to kebab-case
///   - otherwise, keep as-is
/// - otherwise, normalize to kebab-case (camelCase -> kebab-case)
#[inline]
pub(crate) fn normalize_css_property_name(
    key: &str,
    normalize_explicit_custom_props: bool,
) -> String {
    let k = key.trim();
    if let Some(rest) = k.strip_prefix("--") {
        if !normalize_explicit_custom_props {
            return k.to_string();
        }
        let norm = normalize_css_ident(rest);
        if norm.is_empty() {
            return String::new();
        }
        return format!("--{norm}");
    }
    normalize_css_ident(k)
}

#[inline]
pub(crate) fn has_ascii_camel_case(raw: &str) -> bool {
    let bytes = raw.as_bytes();
    if bytes.len() < 2 {
        return false;
    }
    bytes
        .windows(2)
        .any(|w| w[0].is_ascii_lowercase() && w[1].is_ascii_uppercase())
}

#[inline]
pub(crate) fn should_normalize_property_key_fast(
    key: &str,
    normalize_explicit_custom_props: bool,
) -> bool {
    let k = key.trim();
    if k.is_empty() {
        return false;
    }
    if let Some(rest) = k.strip_prefix("--") {
        return normalize_explicit_custom_props
            && (has_ascii_camel_case(rest) || rest.contains('_'));
    }
    has_ascii_camel_case(k) || k.contains('_') || k.contains(' ') || k.contains('\t')
}

/// Normalize vendor property keys:
/// `WebkitFoo -> -webkit-foo`, `MozBar -> -moz-bar`, `msBaz -> -ms-baz`, `OFoo -> -o-foo`.
#[inline]
pub(crate) fn normalize_vendor_property_key(key: &str) -> String {
    let k = key.trim();
    if k.is_empty() {
        return String::new();
    }
    if k.starts_with("--") {
        return k.to_string();
    }
    if k.starts_with("-webkit-")
        || k.starts_with("-moz-")
        || k.starts_with("-ms-")
        || k.starts_with("-o-")
    {
        return k.to_string();
    }

    let (prefix, rest) = if let Some(r) = k.strip_prefix("Webkit") {
        ("-webkit-", r)
    } else if let Some(r) = k.strip_prefix("webkit") {
        ("-webkit-", r)
    } else if let Some(r) = k.strip_prefix("Moz") {
        ("-moz-", r)
    } else if let Some(r) = k.strip_prefix("moz") {
        ("-moz-", r)
    } else if let Some(r) = k.strip_prefix("ms") {
        ("-ms-", r)
    } else if let Some(r) = k.strip_prefix("Ms") {
        ("-ms-", r)
    } else if let Some(r) = k.strip_prefix("O") {
        ("-o-", r)
    } else if let Some(r) = k.strip_prefix("o") {
        ("-o-", r)
    } else {
        ("", k)
    };

    if prefix.is_empty() {
        return normalize_css_ident(rest);
    }
    let tail = normalize_css_ident(rest);
    if tail.is_empty() {
        return String::new();
    }
    format!("{prefix}{tail}")
}

#[inline]
pub(crate) fn strip_vendor_prefix(prop: &str) -> &str {
    prop.strip_prefix("-webkit-")
        .or_else(|| prop.strip_prefix("-moz-"))
        .or_else(|| prop.strip_prefix("-ms-"))
        .or_else(|| prop.strip_prefix("-o-"))
        .unwrap_or(prop)
}

pub(crate) fn expand_vendor_compat_properties(prop: &str, compat: &VendorCompatCfg) -> Vec<String> {
    let prop = prop.trim();
    if prop.is_empty() {
        return Vec::new();
    }

    let mut out = Vec::<String>::new();
    let mut push_unique = |value: &str| {
        let value = value.trim();
        if value.is_empty() {
            return;
        }
        if !out.iter().any(|existing| existing == value) {
            out.push(value.to_string());
        }
    };

    // Always keep the direct property first.
    push_unique(prop);

    // Never expand explicit custom properties.
    if prop.starts_with("--") {
        return out;
    }

    // Vendor -> standard expansion.
    if prop.starts_with('-') {
        if compat.emit_standard_from_prefixed
            && let Some(standard) = standard_variant_for_prefixed(prop)
        {
            push_unique(standard);
        }
        return out;
    }

    // Standard -> vendor expansion.
    if compat.emit_prefixed_from_standard {
        for prefixed in prefixed_variants_for_standard(prop) {
            push_unique(prefixed);
        }
    }

    out
}

fn standard_variant_for_prefixed(prop: &str) -> Option<&'static str> {
    match prop {
        "-webkit-appearance" | "-moz-appearance" => Some("appearance"),
        "-webkit-backdrop-filter" => Some("backdrop-filter"),
        "-webkit-backface-visibility" => Some("backface-visibility"),
        "-webkit-line-clamp" => Some("line-clamp"),
        "-webkit-transform-style" => Some("transform-style"),
        "-webkit-transition" | "-moz-transition" | "-o-transition" => Some("transition"),
        _ => None,
    }
}

fn prefixed_variants_for_standard(prop: &str) -> &'static [&'static str] {
    match prop {
        "appearance" => &["-webkit-appearance", "-moz-appearance"],
        "backdrop-filter" => &["-webkit-backdrop-filter"],
        "backface-visibility" => &["-webkit-backface-visibility"],
        "line-clamp" => &["-webkit-line-clamp"],
        "transform-style" => &["-webkit-transform-style"],
        "transition" => &["-webkit-transition", "-moz-transition", "-o-transition"],
        _ => &[],
    }
}

pub(crate) fn normalize_var_references<'a>(
    value: &'a str,
    normalize_explicit_custom_props: bool,
) -> Cow<'a, str> {
    if !value.contains("var(") {
        return Cow::Borrowed(value);
    }

    let mut out = String::with_capacity(value.len() + 16);
    let bytes = value.as_bytes();
    let mut i = 0usize;
    let mut changed = false;

    while i < value.len() {
        if i + 4 <= value.len() && &bytes[i..i + 4] == b"var(" {
            out.push_str("var(");
            i += 4;

            let inner_start = i;
            let mut depth: i32 = 1;
            while i < value.len() {
                match bytes[i] {
                    b'(' => depth += 1,
                    b')' => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }

            if depth != 0 {
                // Keep malformed tail as-is.
                out.push_str(&value[inner_start..]);
                changed = true;
                break;
            }

            let inner = &value[inner_start..i];
            let normalized_inner = normalize_var_inner(inner, normalize_explicit_custom_props);
            if normalized_inner != inner {
                changed = true;
            }
            out.push_str(normalized_inner.as_str());
            out.push(')');
            i += 1;
            continue;
        }

        out.push(bytes[i] as char);
        i += 1;
    }

    if changed {
        Cow::Owned(out)
    } else {
        Cow::Borrowed(value)
    }
}

fn normalize_var_inner(inner: &str, normalize_explicit_custom_props: bool) -> String {
    let (name_part, tail) = if let Some(comma_idx) = find_top_level_comma(inner) {
        (&inner[..comma_idx], &inner[comma_idx..])
    } else {
        (inner, "")
    };

    let raw_name = name_part.trim();
    if !looks_like_css_var_name(raw_name) {
        return inner.to_string();
    }

    let mut normalized_name =
        normalize_css_property_name(raw_name, normalize_explicit_custom_props);
    if !normalized_name.starts_with("--") {
        normalized_name = format!("--{normalized_name}");
    }

    if tail.is_empty() {
        normalized_name
    } else {
        format!("{normalized_name}{tail}")
    }
}

fn looks_like_css_var_name(raw: &str) -> bool {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return false;
    }
    let candidate = trimmed.strip_prefix("--").unwrap_or(trimmed);
    !candidate.is_empty()
        && candidate
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

fn find_top_level_comma(input: &str) -> Option<usize> {
    let mut depth: i32 = 0;
    for (idx, ch) in input.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            ',' if depth == 0 => return Some(idx),
            _ => {}
        }
    }
    None
}

#[inline]
pub(crate) fn escape_selector_digit_class_prefixes(selector: &str) -> String {
    let chars: Vec<char> = selector.chars().collect();
    let len = chars.len();
    if len < 2 {
        return selector.to_string();
    }

    let mut needs_escape = false;
    for i in 0..(len - 1) {
        if chars[i] == '.'
            && (chars[i + 1].is_ascii_digit()
                || (chars[i + 1] == '-' && i + 2 < len && chars[i + 2].is_ascii_digit()))
        {
            needs_escape = true;
            break;
        }
    }
    if !needs_escape {
        return selector.to_string();
    }

    let mut out = String::with_capacity(selector.len() + 8);
    let mut i = 0usize;
    while i < len {
        if chars[i] != '.' {
            out.push(chars[i]);
            i += 1;
            continue;
        }

        out.push('.');
        i += 1;
        if i >= len {
            break;
        }

        if chars[i] == '-' && i + 1 < len && chars[i + 1].is_ascii_digit() {
            out.push('-');
            i += 1;
        }

        if i < len && chars[i].is_ascii_digit() {
            let digit = chars[i];
            out.push('\\');
            out.push_str(&format!("{:x}", digit as u32));
            out.push(' ');
            i += 1;
        }
    }

    out
}
