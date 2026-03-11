//! Hashing utilities for cache keys, class names, and rewrite fingerprints.

pub(crate) use crate::compiler::hash::HashProfileTarget;
use crate::compiler::hash::{Hash64Builder, HashCtorParams, hash_profile_for};
use crate::engine::cssinjs::util::escape_selector_digit_class_prefixes;

use super::CssInJsStyleInput;
use super::config::{CssInJsConfig, CssRewriteCfg};
use crate::style_provider::HashPriority;

// ----------------------------------------------------------------------------
// Profile resolution
// ----------------------------------------------------------------------------

#[inline]
pub(crate) fn resolved_runtime_hash_profile(
    cfg: &CssInJsConfig,
    target: HashProfileTarget,
) -> HashCtorParams {
    let mut params = hash_profile_for(target);
    if let Some(tag) = cfg
        .hash_version_tag
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        params.version_tag = tag.as_bytes().to_vec();
    }
    params
}

// ----------------------------------------------------------------------------
// Core hashing
// ----------------------------------------------------------------------------

#[inline]
pub(crate) fn hash64_chunks(
    cfg: &CssInJsConfig,
    target: HashProfileTarget,
    chunks: &[&[u8]],
) -> u64 {
    let params = resolved_runtime_hash_profile(cfg, target);
    let mut hasher = Hash64Builder::from_params(&params);
    for chunk in chunks {
        hasher.update_len_prefixed_bytes(chunk);
    }
    hasher.finish_u64()
}

// ----------------------------------------------------------------------------
// Base62 encoding
// ----------------------------------------------------------------------------

#[inline]
fn base62_u64(mut num: u64) -> String {
    const CH: &[u8; 62] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    if num == 0 {
        return "0".to_string();
    }
    let mut buf = Vec::<u8>::with_capacity(11);
    while num != 0 {
        let rem = (num % 62) as usize;
        num /= 62;
        buf.push(CH[rem]);
    }
    buf.reverse();
    core::str::from_utf8(&buf)
        .map(ToString::to_string)
        .unwrap_or_else(|_| "0".to_string())
}

#[inline]
fn base62_u64_compact(num: u64, max_len: usize) -> String {
    let full = base62_u64(num);
    if max_len == 0 || max_len >= full.len() {
        return full;
    }
    full[..max_len].to_string()
}

// ----------------------------------------------------------------------------
// Class hash (alpha-only compact body)
// ----------------------------------------------------------------------------

#[inline]
pub(crate) fn class_hash_u64_compact(num: u64, configured_len: Option<usize>) -> String {
    const ALPHA: &[u8; 52] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let max_len = configured_len.filter(|v| *v > 0).unwrap_or(6).min(6);

    let mut out = String::with_capacity(max_len);
    let mut value = num;
    for _ in 0..max_len {
        value = value.wrapping_mul(0x9e37_79b9_7f4a_7c15).rotate_left(11) ^ 0xbf58_476d_1ce4_e5b9;
        out.push(ALPHA[(value % 52) as usize] as char);
    }
    out
}

// ----------------------------------------------------------------------------
// Rewrite fingerprint
// ----------------------------------------------------------------------------

#[inline]
pub(crate) fn rewrite_fingerprint(rw: &CssRewriteCfg, cfg: &CssInJsConfig) -> u64 {
    let mut chunks: Vec<Vec<u8>> = Vec::new();
    for (a, b) in &rw.class_prefix_pairs {
        chunks.push(format!("c:{}=>{}", a.as_ref(), b.as_ref()).into_bytes());
    }
    for (a, b) in &rw.css_var_prefix_pairs {
        chunks.push(format!("v:{}=>{}", a.as_ref(), b.as_ref()).into_bytes());
    }
    let mut refs: Vec<&[u8]> = Vec::with_capacity(chunks.len());
    for c in &chunks {
        refs.push(c.as_slice());
    }
    hash64_chunks(cfg, HashProfileTarget::Global, &refs)
}

// ----------------------------------------------------------------------------
// Identity and cache keys
// ----------------------------------------------------------------------------

#[inline]
pub(crate) fn identity_key(input: &CssInJsStyleInput, hash_class: &str, rewrite_fp: u64) -> String {
    let stable_identity = input
        .identity_scope
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| input.style_id.as_ref().trim());

    format!(
        "{stable_identity}|hc={hash_class}|rw={rewrite_fp:016x}|layer={}|th={}|alg={}|scope={}",
        input.layer.as_deref().unwrap_or("").trim(),
        input.token_hash.as_deref().unwrap_or("").trim(),
        input.algorithm.as_deref().unwrap_or("").trim(),
        input.theme_scope.as_deref().unwrap_or("").trim(),
    )
}

#[inline]
pub(crate) fn cache_key(
    cfg: &CssInJsConfig,
    input: &CssInJsStyleInput,
    hash_class: &str,
    rendered_css: &str,
    rewrite_fp: u64,
) -> String {
    let id = identity_key(input, hash_class, rewrite_fp);
    let profile = resolved_runtime_hash_profile(cfg, HashProfileTarget::CssInJsCache);
    let h = hash64_chunks(
        cfg,
        HashProfileTarget::CssInJsCache,
        &[
            id.as_bytes(),
            b"|",
            rendered_css.as_bytes(),
            b"|",
            input.css.as_bytes(),
        ],
    );
    let body = base62_u64_compact(h, cfg.cache_key_len.or(profile.encoded_len).unwrap_or(0));
    if let Some(pre) = cfg
        .cache_key_prefix
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToString::to_string)
        .or_else(|| profile.prefix.clone())
    {
        format!("{pre}{body}")
    } else {
        body
    }
}

// ----------------------------------------------------------------------------
// Unique hash (for style IDs)
// ----------------------------------------------------------------------------

#[inline]
pub(crate) fn unique_hash(cfg: &CssInJsConfig, path: &[String], style_str: &str) -> String {
    let profile = resolved_runtime_hash_profile(cfg, HashProfileTarget::CssInJsUnique);
    let mut hasher = Hash64Builder::from_params(&profile);

    hasher.update_bytes(b"|path:");
    for seg in path {
        let seg = seg.trim();
        if seg.is_empty() {
            continue;
        }
        hasher.update_len_prefixed_bytes(seg.as_bytes());
    }

    hasher.update_bytes(b"|css:");
    hasher.update_str(style_str.trim());

    let digest = hasher.finish_u64();
    let body = base62_u64_compact(
        digest,
        cfg.unique_hash_len.or(profile.encoded_len).unwrap_or(0),
    );

    if let Some(pre) = cfg
        .unique_hash_prefix
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToString::to_string)
        .or_else(|| profile.prefix.clone())
    {
        format!("{pre}{body}")
    } else {
        body
    }
}

// ----------------------------------------------------------------------------
// Scoped selector helper
// ----------------------------------------------------------------------------

#[inline]
pub(crate) fn scoped_hash_selector(class_name: &str, hash_priority: HashPriority) -> String {
    let selector = class_selector(class_name);
    if selector.is_empty() {
        return String::new();
    }
    match hash_priority {
        HashPriority::Low => format!(":where({selector})"),
        HashPriority::High => selector,
    }
}

#[inline]
pub(crate) fn merge_scope_prefix_into_selector(scope_prefix: &str, selector: &str) -> String {
    let scope_prefix = scope_prefix.trim();
    if scope_prefix.is_empty() {
        return selector.to_string();
    }

    let normalized_selector = escape_selector_digit_class_prefixes(selector);

    normalized_selector
        .split(',')
        .map(str::trim)
        .filter(|seg: &&str| !seg.is_empty())
        .map(|seg: &str| {
            let mut full_path = seg
                .split_whitespace()
                .map(ToString::to_string)
                .collect::<Vec<_>>();
            if full_path.is_empty() {
                return scope_prefix.to_string();
            }
            let first = full_path[0].clone();
            let html_prefix = first
                .chars()
                .take_while(|ch: &char| ch.is_ascii_alphanumeric())
                .collect::<String>();
            let merged_first = if html_prefix.is_empty() {
                format!("{scope_prefix}{first}")
            } else {
                format!(
                    "{html_prefix}{scope_prefix}{}",
                    &first[html_prefix.len()..]
                )
            };
            full_path[0] = merged_first;
            full_path.join(" ")
        })
        .collect::<Vec<_>>()
        .join(",")
}

#[inline]
pub(crate) fn inject_hash_into_selector(
    selector: &str,
    class_name: &str,
    hash_priority: HashPriority,
) -> String {
    let scope_prefix = scoped_hash_selector(class_name, hash_priority);
    if scope_prefix.is_empty() {
        selector.to_string()
    } else {
        merge_scope_prefix_into_selector(scope_prefix.as_str(), selector)
    }
}

#[inline]
pub(crate) fn class_selector(class_name: &str) -> String {
    let escaped = escape_css_class_name(class_name);
    if escaped.is_empty() {
        String::new()
    } else {
        format!(".{escaped}")
    }
}

#[inline]
fn escape_css_class_name(raw: &str) -> String {
    let chars: Vec<char> = raw.trim().chars().collect();
    if chars.is_empty() {
        return String::new();
    }

    let mut out = String::with_capacity(chars.len() + 8);
    for (idx, ch) in chars.iter().copied().enumerate() {
        let needs_numeric_start_escape = (idx == 0 && ch.is_ascii_digit())
            || (idx == 1 && chars.first().copied() == Some('-') && ch.is_ascii_digit());

        if needs_numeric_start_escape {
            out.push('\\');
            out.push_str(&format!("{:x}", ch as u32));
            out.push(' ');
            continue;
        }

        match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => out.push(ch),
            c if c as u32 >= 0x80 => out.push(c),
            _ => {
                out.push('\\');
                out.push(ch);
            }
        }
    }

    out
}
