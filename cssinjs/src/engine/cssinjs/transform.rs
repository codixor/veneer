//! CSS transformation: prefix rewriting and rule scoping.

use super::config::CssRewriteCfg;

pub struct CssTransform;

impl CssTransform {
    fn rewrite_class_attr_value(input: &str, from: &str, to: &str) -> String {
        if input.is_empty() || from.is_empty() || to.is_empty() || from == to {
            return input.to_string();
        }

        let mut out = String::with_capacity(input.len() + 8);
        let mut i = 0usize;

        while i < input.len() {
            let rest = &input[i..];
            let at_token_start = i == 0 || input.as_bytes()[i - 1].is_ascii_whitespace();
            if at_token_start && rest.starts_with(from) {
                let next = rest.as_bytes().get(from.len()).copied();
                let token_like =
                    next.is_none() || matches!(next, Some(b'-' | b' ' | b'"' | b'\'' | b']'));
                if token_like {
                    out.push_str(to);
                    i += from.len();
                    continue;
                }
            }

            let ch = rest.chars().next().expect("slice is non-empty");
            out.push(ch);
            i += ch.len_utf8();
        }

        out
    }

    pub fn apply_rewrite_rules(raw: &str, rw: &CssRewriteCfg) -> String {
        if rw.class_prefix_pairs.is_empty() && rw.css_var_prefix_pairs.is_empty() {
            return raw.to_string();
        }

        let mut out = String::with_capacity(raw.len());
        let bytes = raw.as_bytes();
        let mut i = 0usize;
        let mut in_comment = false;

        while i < raw.len() {
            let b = bytes[i];

            if in_comment {
                if b == b'*' && i + 1 < raw.len() && bytes[i + 1] == b'/' {
                    in_comment = false;
                    out.push_str("*/");
                    i += 2;
                    continue;
                }
                out.push(b as char);
                i += 1;
                continue;
            }

            if b == b'/' && i + 1 < raw.len() && bytes[i + 1] == b'*' {
                in_comment = true;
                out.push_str("/*");
                i += 2;
                continue;
            }

            if b == b'\'' || b == b'"' {
                let quote = b as char;
                let start = i + 1;
                out.push(quote);
                i += 1;
                while i < raw.len() {
                    let qb = bytes[i];
                    if qb == b'\\' && i + 1 < raw.len() {
                        i += 2;
                        continue;
                    }
                    if qb == b {
                        break;
                    }
                    i += 1;
                }
                let content = &raw[start..i];
                let mut rewritten = content.to_string();
                for (from, to) in &rw.class_prefix_pairs {
                    let from = from.as_ref().trim();
                    let to = to.as_ref().trim();
                    rewritten = Self::rewrite_class_attr_value(rewritten.as_str(), from, to);
                }
                out.push_str(&rewritten);
                if i < raw.len() {
                    out.push(quote);
                    i += 1;
                }
                continue;
            }

            let start = i;
            while i < raw.len() {
                let b2 = bytes[i];
                if b2 == b'\''
                    || b2 == b'"'
                    || (b2 == b'/' && i + 1 < raw.len() && bytes[i + 1] == b'*')
                {
                    break;
                }
                i += 1;
            }
            let chunk = &raw[start..i];
            let mut rewritten = chunk.to_string();

            for (from, to) in &rw.class_prefix_pairs {
                let from = from.as_ref().trim();
                let to = to.as_ref().trim();
                if from.is_empty() || to.is_empty() || from == to {
                    continue;
                }
                rewritten = rewritten.replace(&format!(".{from}-"), &format!(".{to}-"));
                rewritten = rewritten.replace(&format!(".{from} "), &format!(".{to} "));
                rewritten = rewritten.replace(&format!(".{from}["), &format!(".{to}["));
            }
            for (from, to) in &rw.css_var_prefix_pairs {
                let from = from.as_ref().trim();
                let to = to.as_ref().trim();
                if from.is_empty() || to.is_empty() || from == to {
                    continue;
                }
                rewritten = rewritten.replace(&format!("--{from}-"), &format!("--{to}-"));
            }

            out.push_str(&rewritten);
        }

        out
    }

    pub fn scope_each_rule(css: &str, scope_prefix: &str) -> String {
        let bytes = css.as_bytes();
        let mut out = String::with_capacity(css.len() + 32);
        let mut i = 0usize;

        while i < bytes.len() {
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i >= bytes.len() {
                break;
            }

            let prelude_start = i;
            let mut in_sq = false;
            let mut in_dq = false;
            let mut in_comment = false;

            while i < bytes.len() {
                let b = bytes[i];

                if in_comment {
                    if b == b'*' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                        in_comment = false;
                        i += 2;
                        continue;
                    }
                    i += 1;
                    continue;
                }

                if in_sq {
                    if b == b'\\' && i + 1 < bytes.len() {
                        i += 2;
                        continue;
                    }
                    if b == b'\'' {
                        in_sq = false;
                    }
                    i += 1;
                    continue;
                }

                if in_dq {
                    if b == b'\\' && i + 1 < bytes.len() {
                        i += 2;
                        continue;
                    }
                    if b == b'"' {
                        in_dq = false;
                    }
                    i += 1;
                    continue;
                }

                if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
                    in_comment = true;
                    i += 2;
                    continue;
                }
                if b == b'\'' {
                    in_sq = true;
                    i += 1;
                    continue;
                }
                if b == b'"' {
                    in_dq = true;
                    i += 1;
                    continue;
                }

                if b == b'{' || b == b';' {
                    break;
                }
                i += 1;
            }

            if i >= bytes.len() {
                break;
            }

            let prelude = css[prelude_start..i].trim();
            if prelude.is_empty() {
                i += 1;
                continue;
            }

            if bytes[i] == b';' {
                out.push_str(prelude);
                out.push(';');
                out.push('\n');
                i += 1;
                continue;
            }

            let Some(body_end) = Self::find_matching_brace(css, i) else {
                out.push_str(&css[prelude_start..]);
                break;
            };

            let body = css[i + 1..body_end].trim();
            if Self::is_passthrough_at_rule(prelude) {
                out.push_str(prelude);
                out.push('{');
                out.push_str(body);
                out.push('}');
                out.push('\n');
            } else if Self::is_recursive_scope_at_rule(prelude) {
                let nested = Self::scope_each_rule(body, scope_prefix);
                out.push_str(prelude);
                out.push('{');
                out.push_str(nested.trim());
                out.push('}');
                out.push('\n');
            } else if prelude.starts_with('@') {
                out.push_str(prelude);
                out.push('{');
                out.push_str(body);
                out.push('}');
                out.push('\n');
            } else {
                let scoped_sel = Self::split_selector_list(prelude)
                    .into_iter()
                    .map(|s| super::hash::merge_scope_prefix_into_selector(scope_prefix, s.trim()))
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join(", ");

                if !scoped_sel.is_empty() {
                    out.push_str(&scoped_sel);
                    out.push('{');
                    out.push_str(body);
                    out.push('}');
                    out.push('\n');
                }
            }

            i = body_end + 1;
        }

        out
    }

    fn is_passthrough_at_rule(prelude: &str) -> bool {
        let lower = prelude.trim().to_ascii_lowercase();
        lower.starts_with("@keyframes")
            || lower.starts_with("@-webkit-keyframes")
            || lower.starts_with("@font-face")
            || lower.starts_with("@property")
            || lower.starts_with("@page")
    }

    fn is_recursive_scope_at_rule(prelude: &str) -> bool {
        let lower = prelude.trim().to_ascii_lowercase();
        lower.starts_with("@media")
            || lower.starts_with("@supports")
            || lower.starts_with("@container")
            || lower.starts_with("@layer")
    }

    fn split_selector_list(selector: &str) -> Vec<&str> {
        let bytes = selector.as_bytes();
        let mut parts = Vec::new();
        let mut start = 0usize;
        let mut i = 0usize;
        let mut paren_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut in_sq = false;
        let mut in_dq = false;
        let mut in_comment = false;

        while i < bytes.len() {
            let b = bytes[i];

            if in_comment {
                if b == b'*' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                    in_comment = false;
                    i += 2;
                    continue;
                }
                i += 1;
                continue;
            }

            if in_sq {
                if b == b'\\' && i + 1 < bytes.len() {
                    i += 2;
                    continue;
                }
                if b == b'\'' {
                    in_sq = false;
                }
                i += 1;
                continue;
            }

            if in_dq {
                if b == b'\\' && i + 1 < bytes.len() {
                    i += 2;
                    continue;
                }
                if b == b'"' {
                    in_dq = false;
                }
                i += 1;
                continue;
            }

            if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
                in_comment = true;
                i += 2;
                continue;
            }
            if b == b'\'' {
                in_sq = true;
                i += 1;
                continue;
            }
            if b == b'"' {
                in_dq = true;
                i += 1;
                continue;
            }

            match b {
                b'(' => paren_depth = paren_depth.saturating_add(1),
                b')' => paren_depth = paren_depth.saturating_sub(1),
                b'[' => bracket_depth = bracket_depth.saturating_add(1),
                b']' => bracket_depth = bracket_depth.saturating_sub(1),
                b',' if paren_depth == 0 && bracket_depth == 0 => {
                    let part = selector[start..i].trim();
                    if !part.is_empty() {
                        parts.push(part);
                    }
                    start = i + 1;
                }
                _ => {}
            }

            i += 1;
        }

        let tail = selector[start..].trim();
        if !tail.is_empty() {
            parts.push(tail);
        }

        parts
    }

    fn find_matching_brace(css: &str, open_brace_idx: usize) -> Option<usize> {
        let bytes = css.as_bytes();
        if bytes.get(open_brace_idx) != Some(&b'{') {
            return None;
        }

        let mut i = open_brace_idx + 1;
        let mut depth = 1usize;
        let mut in_sq = false;
        let mut in_dq = false;
        let mut in_comment = false;

        while i < bytes.len() {
            let b = bytes[i];

            if in_comment {
                if b == b'*' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                    in_comment = false;
                    i += 2;
                    continue;
                }
                i += 1;
                continue;
            }

            if in_sq {
                if b == b'\\' && i + 1 < bytes.len() {
                    i += 2;
                    continue;
                }
                if b == b'\'' {
                    in_sq = false;
                }
                i += 1;
                continue;
            }

            if in_dq {
                if b == b'\\' && i + 1 < bytes.len() {
                    i += 2;
                    continue;
                }
                if b == b'"' {
                    in_dq = false;
                }
                i += 1;
                continue;
            }

            if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
                in_comment = true;
                i += 2;
                continue;
            }
            if b == b'\'' {
                in_sq = true;
                i += 1;
                continue;
            }
            if b == b'"' {
                in_dq = true;
                i += 1;
                continue;
            }

            if b == b'{' {
                depth += 1;
            } else if b == b'}' {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(i);
                }
            }

            i += 1;
        }

        None
    }
}

// ----------------------------------------------------------------------------
// LightningCSS integration (optional)
// ----------------------------------------------------------------------------

pub(crate) fn lightning_scope(css: &str, scope_prefix: &str, minify: bool) -> Option<String> {
    use lightningcss::stylesheet::{ParserOptions, PrinterOptions, StyleSheet};

    let parsed = StyleSheet::parse(css, ParserOptions::default()).ok()?;
    let normalized = parsed
        .to_css(PrinterOptions {
            minify,
            ..Default::default()
        })
        .ok()?
        .code;

    Some(CssTransform::scope_each_rule(
        normalized.as_str(),
        scope_prefix,
    ))
}
