//! Scope hashing utilities (CSS-safe, deterministic).
//!
//! Enterprise goals:
//! - deterministic across platforms/builds
//! - streaming (no intermediate buffers)
//! - version-tag salt for format evolution

use std::collections::BTreeMap;
use std::sync::{OnceLock, RwLock};

use xxhash_rust::xxh3::Xxh3;

/// Max emitted hash-body length for generated scopes.
///
/// This aligns with runtime cssinjs class hash shape (short, CSS-safe).
pub const MAX_SCOPE_HASH_LEN: usize = 6;

const ENV_HASH_VERSION_TAG: &str = "DIOXUS_STYLE_HASH_VERSION_TAG";
const ENV_HASH_SEPARATOR: &str = "DIOXUS_STYLE_HASH_SEPARATOR";
const ENV_HASH_ENCODED_LEN: &str = "DIOXUS_STYLE_HASH_ENCODED_LEN";

const ENV_SCOPE_PREFIX: &str = "DIOXUS_STYLE_SCOPE_HASH_";
const ENV_RUNTIME_PREFIX: &str = "DIOXUS_STYLE_RUNTIME_HASH_";

/// Validation issue produced while decoding shared hash source values.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HashSourceValidationError {
    pub key: String,
    pub reason: String,
}

impl HashSourceValidationError {
    #[inline]
    #[must_use]
    pub fn new(key: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            reason: reason.into(),
        }
    }
}

#[inline]
fn scope_target_key(target: ScopeHashTarget) -> &'static str {
    match target {
        ScopeHashTarget::Scope => "SCOPE",
        ScopeHashTarget::Acss => "ACSS",
        ScopeHashTarget::AcssMacro => "ACSS_MACRO",
        ScopeHashTarget::ScssCache => "SCSS_CACHE",
        ScopeHashTarget::CssInJs => "CSSINJS",
    }
}

#[inline]
fn runtime_target_key(target: HashProfileTarget) -> &'static str {
    match target {
        HashProfileTarget::Global => "GLOBAL",
        HashProfileTarget::CssInJsUnique => "CSSINJS_UNIQUE",
        HashProfileTarget::CssInJsCache => "CSSINJS_CACHE",
        HashProfileTarget::CssInJsClass => "CSSINJS_CLASS",
    }
}

#[inline]
fn env_lookup(key: &str) -> Option<String> {
    std::env::var(key).ok()
}

#[inline]
fn lookup_non_empty<F>(
    lookup: &F,
    key: &str,
    errors: &mut Vec<HashSourceValidationError>,
) -> Option<String>
where
    F: Fn(&str) -> Option<String>,
{
    let raw = lookup(key)?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        errors.push(HashSourceValidationError::new(
            key,
            "value must not be empty",
        ));
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[inline]
fn lookup_char<F>(
    lookup: &F,
    key: &str,
    errors: &mut Vec<HashSourceValidationError>,
) -> Option<char>
where
    F: Fn(&str) -> Option<String>,
{
    let value = lookup_non_empty(lookup, key, errors)?;
    let mut chars = value.chars();
    match (chars.next(), chars.next()) {
        (Some(ch), None) => Some(ch),
        _ => {
            errors.push(HashSourceValidationError::new(
                key,
                "separator must be exactly one character",
            ));
            None
        }
    }
}

#[inline]
fn lookup_usize<F>(
    lookup: &F,
    key: &str,
    min: usize,
    max: usize,
    errors: &mut Vec<HashSourceValidationError>,
) -> Option<usize>
where
    F: Fn(&str) -> Option<String>,
{
    let value = lookup_non_empty(lookup, key, errors)?;
    match value.parse::<usize>() {
        Ok(v) if v >= min && v <= max => Some(v),
        Ok(v) => {
            errors.push(HashSourceValidationError::new(
                key,
                format!("value `{v}` is out of range [{min}..={max}]"),
            ));
            None
        }
        Err(_) => {
            errors.push(HashSourceValidationError::new(
                key,
                format!("value `{value}` is not a valid integer"),
            ));
            None
        }
    }
}

#[inline]
fn scope_params_mut(
    profiles: &mut ScopeHashProfiles,
    target: ScopeHashTarget,
) -> &mut ScopeHashParams {
    match target {
        ScopeHashTarget::Scope => &mut profiles.scope,
        ScopeHashTarget::Acss => &mut profiles.acss,
        ScopeHashTarget::AcssMacro => &mut profiles.acss_macro,
        ScopeHashTarget::ScssCache => &mut profiles.scss_cache,
        ScopeHashTarget::CssInJs => &mut profiles.cssinjs,
    }
}

#[inline]
fn runtime_params_mut(
    profiles: &mut HashProfiles,
    target: HashProfileTarget,
) -> &mut HashCtorParams {
    match target {
        HashProfileTarget::Global => &mut profiles.global,
        HashProfileTarget::CssInJsUnique => &mut profiles.cssinjs_unique,
        HashProfileTarget::CssInJsCache => &mut profiles.cssinjs_cache,
        HashProfileTarget::CssInJsClass => &mut profiles.cssinjs_class,
    }
}

fn apply_scope_profiles_from_source<F>(
    profiles: &mut ScopeHashProfiles,
    lookup: &F,
    errors: &mut Vec<HashSourceValidationError>,
) where
    F: Fn(&str) -> Option<String>,
{
    if let Some(version_tag) = lookup_non_empty(lookup, ENV_HASH_VERSION_TAG, errors) {
        for target in [
            ScopeHashTarget::Scope,
            ScopeHashTarget::Acss,
            ScopeHashTarget::AcssMacro,
            ScopeHashTarget::ScssCache,
            ScopeHashTarget::CssInJs,
        ] {
            scope_params_mut(profiles, target).version_tag = version_tag.as_bytes().to_vec();
        }
    }
    if let Some(separator) = lookup_char(lookup, ENV_HASH_SEPARATOR, errors) {
        for target in [
            ScopeHashTarget::Scope,
            ScopeHashTarget::Acss,
            ScopeHashTarget::AcssMacro,
            ScopeHashTarget::ScssCache,
            ScopeHashTarget::CssInJs,
        ] {
            scope_params_mut(profiles, target).separator = separator;
        }
    }
    if let Some(encoded_len) =
        lookup_usize(lookup, ENV_HASH_ENCODED_LEN, 1, MAX_SCOPE_HASH_LEN, errors)
    {
        for target in [
            ScopeHashTarget::Scope,
            ScopeHashTarget::Acss,
            ScopeHashTarget::AcssMacro,
            ScopeHashTarget::ScssCache,
            ScopeHashTarget::CssInJs,
        ] {
            scope_params_mut(profiles, target).encoded_len = encoded_len;
        }
    }

    for target in [
        ScopeHashTarget::Scope,
        ScopeHashTarget::Acss,
        ScopeHashTarget::AcssMacro,
        ScopeHashTarget::ScssCache,
        ScopeHashTarget::CssInJs,
    ] {
        let key_name = scope_target_key(target);
        let version_key = format!("{ENV_SCOPE_PREFIX}{key_name}_VERSION_TAG");
        if let Some(version_tag) = lookup_non_empty(lookup, version_key.as_str(), errors) {
            scope_params_mut(profiles, target).version_tag = version_tag.as_bytes().to_vec();
        }

        let prefix_key = format!("{ENV_SCOPE_PREFIX}{key_name}_PREFIX");
        if let Some(prefix) = lookup_non_empty(lookup, prefix_key.as_str(), errors) {
            scope_params_mut(profiles, target).prefix = prefix;
        }

        let separator_key = format!("{ENV_SCOPE_PREFIX}{key_name}_SEPARATOR");
        if let Some(separator) = lookup_char(lookup, separator_key.as_str(), errors) {
            scope_params_mut(profiles, target).separator = separator;
        }

        let encoded_len_key = format!("{ENV_SCOPE_PREFIX}{key_name}_ENCODED_LEN");
        if let Some(encoded_len) = lookup_usize(
            lookup,
            encoded_len_key.as_str(),
            1,
            MAX_SCOPE_HASH_LEN,
            errors,
        ) {
            scope_params_mut(profiles, target).encoded_len = encoded_len;
        }
    }
}

fn apply_runtime_profiles_from_source<F>(
    profiles: &mut HashProfiles,
    lookup: &F,
    errors: &mut Vec<HashSourceValidationError>,
) where
    F: Fn(&str) -> Option<String>,
{
    if let Some(version_tag) = lookup_non_empty(lookup, ENV_HASH_VERSION_TAG, errors) {
        for target in [
            HashProfileTarget::Global,
            HashProfileTarget::CssInJsUnique,
            HashProfileTarget::CssInJsCache,
            HashProfileTarget::CssInJsClass,
        ] {
            runtime_params_mut(profiles, target).version_tag = version_tag.as_bytes().to_vec();
        }
    }
    if let Some(encoded_len) = lookup_usize(lookup, ENV_HASH_ENCODED_LEN, 0, 64, errors) {
        for target in [
            HashProfileTarget::Global,
            HashProfileTarget::CssInJsUnique,
            HashProfileTarget::CssInJsCache,
            HashProfileTarget::CssInJsClass,
        ] {
            runtime_params_mut(profiles, target).encoded_len = Some(encoded_len);
        }
    }

    for target in [
        HashProfileTarget::Global,
        HashProfileTarget::CssInJsUnique,
        HashProfileTarget::CssInJsCache,
        HashProfileTarget::CssInJsClass,
    ] {
        let key_name = runtime_target_key(target);
        let version_key = format!("{ENV_RUNTIME_PREFIX}{key_name}_VERSION_TAG");
        if let Some(version_tag) = lookup_non_empty(lookup, version_key.as_str(), errors) {
            runtime_params_mut(profiles, target).version_tag = version_tag.as_bytes().to_vec();
        }

        let prefix_key = format!("{ENV_RUNTIME_PREFIX}{key_name}_PREFIX");
        if let Some(prefix) = lookup_non_empty(lookup, prefix_key.as_str(), errors) {
            runtime_params_mut(profiles, target).prefix = Some(prefix);
        }

        let encoded_len_key = format!("{ENV_RUNTIME_PREFIX}{key_name}_ENCODED_LEN");
        if let Some(encoded_len) = lookup_usize(lookup, encoded_len_key.as_str(), 0, 64, errors) {
            runtime_params_mut(profiles, target).encoded_len = Some(encoded_len);
        }
    }
}

#[inline]
fn report_source_errors(errors: &[HashSourceValidationError]) {
    #[cfg(debug_assertions)]
    for error in errors {
        eprintln!(
            "[cssinjs/compiler/hash] invalid shared source key `{}`: {}",
            error.key, error.reason
        );
    }
}

fn build_scope_profiles_from_source<F>(
    lookup: F,
) -> (ScopeHashProfiles, Vec<HashSourceValidationError>)
where
    F: Fn(&str) -> Option<String>,
{
    let mut profiles = ScopeHashProfiles::default();
    let mut errors = Vec::new();
    apply_scope_profiles_from_source(&mut profiles, &lookup, &mut errors);
    (profiles, errors)
}

fn build_runtime_profiles_from_source<F>(
    lookup: F,
) -> (HashProfiles, Vec<HashSourceValidationError>)
where
    F: Fn(&str) -> Option<String>,
{
    let mut profiles = HashProfiles::default();
    let mut errors = Vec::new();
    apply_runtime_profiles_from_source(&mut profiles, &lookup, &mut errors);
    (profiles, errors)
}

/// Target domain for hash configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub enum ScopeHashTarget {
    Scope,
    Acss,
    AcssMacro,
    ScssCache,
    CssInJs,
}

/// Owned hash parameters with runtime mutability support.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopeHashParams {
    pub prefix: String,
    pub version_tag: Vec<u8>,
    pub separator: char,
    pub encoded_len: usize,
}

impl ScopeHashParams {
    #[inline]
    #[must_use]
    pub fn new(prefix: impl Into<String>, version_tag: impl AsRef<[u8]>) -> Self {
        Self {
            prefix: prefix.into(),
            version_tag: version_tag.as_ref().to_vec(),
            separator: '_',
            encoded_len: MAX_SCOPE_HASH_LEN,
        }
    }

    #[inline]
    #[must_use]
    pub fn scope_default() -> Self {
        Self::new("sc", b"sc:v1")
    }

    #[inline]
    #[must_use]
    pub fn acss_default() -> Self {
        Self::new("acss", b"acss:v1")
    }

    #[inline]
    #[must_use]
    pub fn acss_macro_default() -> Self {
        Self::new("acss", b"acss:macro:v1")
    }

    #[inline]
    #[must_use]
    pub fn scss_cache_default() -> Self {
        Self::new("scss-cache", b"scss-cache:v1")
    }

    #[inline]
    #[must_use]
    pub fn cssinjs_default() -> Self {
        Self::new("cssinjs", b"cssinjs:v1")
    }

    #[inline]
    #[must_use]
    pub fn as_cfg(&self) -> ScopeHashCfg<'_> {
        ScopeHashCfg {
            prefix: self.prefix.as_str(),
            version_tag: self.version_tag.as_slice(),
            separator: self.separator,
            encoded_len: self.encoded_len,
        }
    }
}

impl Default for ScopeHashParams {
    #[inline]
    fn default() -> Self {
        Self::scope_default()
    }
}

/// Global profile bag used by ACSS/SCSS/macros/runtime consumers.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopeHashProfiles {
    pub scope: ScopeHashParams,
    pub acss: ScopeHashParams,
    pub acss_macro: ScopeHashParams,
    pub scss_cache: ScopeHashParams,
    pub cssinjs: ScopeHashParams,
}

impl Default for ScopeHashProfiles {
    #[inline]
    fn default() -> Self {
        Self {
            scope: ScopeHashParams::scope_default(),
            acss: ScopeHashParams::acss_default(),
            acss_macro: ScopeHashParams::acss_macro_default(),
            scss_cache: ScopeHashParams::scss_cache_default(),
            cssinjs: ScopeHashParams::cssinjs_default(),
        }
    }
}

static SCOPE_HASH_PROFILES: OnceLock<RwLock<ScopeHashProfiles>> = OnceLock::new();

#[inline]
fn hash_profiles_lock() -> &'static RwLock<ScopeHashProfiles> {
    SCOPE_HASH_PROFILES.get_or_init(|| {
        let (profiles, errors) = build_scope_profiles_from_source(env_lookup);
        report_source_errors(errors.as_slice());
        RwLock::new(profiles)
    })
}

#[inline]
#[must_use]
pub fn scope_hash_profiles() -> ScopeHashProfiles {
    match hash_profiles_lock().read() {
        Ok(guard) => guard.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    }
}

#[inline]
#[allow(dead_code)]
pub fn set_scope_hash_profiles(next: ScopeHashProfiles) -> bool {
    match hash_profiles_lock().write() {
        Ok(mut guard) => {
            if *guard == next {
                false
            } else {
                *guard = next;
                true
            }
        }
        Err(poisoned) => {
            let mut guard = poisoned.into_inner();
            if *guard == next {
                false
            } else {
                *guard = next;
                true
            }
        }
    }
}

#[inline]
#[allow(dead_code)]
pub fn reset_scope_hash_profiles() {
    let (profiles, _) = build_scope_profiles_from_source(env_lookup);
    let _ = set_scope_hash_profiles(profiles);
}

#[inline]
#[allow(dead_code)]
pub fn set_scope_hash_params_global(next: ScopeHashParams) {
    let mut profiles = scope_hash_profiles();
    profiles.scope = next.clone();
    profiles.acss = next.clone();
    profiles.acss_macro = next.clone();
    profiles.scss_cache = next.clone();
    profiles.cssinjs = next;
    let _ = set_scope_hash_profiles(profiles);
}

#[inline]
#[allow(dead_code)]
pub fn set_scope_hash_params_for(target: ScopeHashTarget, next: ScopeHashParams) {
    let mut profiles = scope_hash_profiles();
    match target {
        ScopeHashTarget::Scope => profiles.scope = next,
        ScopeHashTarget::Acss => profiles.acss = next,
        ScopeHashTarget::AcssMacro => profiles.acss_macro = next,
        ScopeHashTarget::ScssCache => profiles.scss_cache = next,
        ScopeHashTarget::CssInJs => profiles.cssinjs = next,
    }
    let _ = set_scope_hash_profiles(profiles);
}

#[inline]
#[must_use]
pub fn scope_hash_params_for(target: ScopeHashTarget) -> ScopeHashParams {
    let profiles = scope_hash_profiles();
    match target {
        ScopeHashTarget::Scope => profiles.scope,
        ScopeHashTarget::Acss => profiles.acss,
        ScopeHashTarget::AcssMacro => profiles.acss_macro,
        ScopeHashTarget::ScssCache => profiles.scss_cache,
        ScopeHashTarget::CssInJs => profiles.cssinjs,
    }
}

/// Resolve scope-hash profiles from a map-based shared source.
#[must_use]
pub fn resolve_scope_hash_profiles_from_map(
    source: &BTreeMap<String, String>,
) -> (ScopeHashProfiles, Vec<HashSourceValidationError>) {
    build_scope_profiles_from_source(|key| source.get(key).cloned())
}

/// Re-read scope-hash profiles from process env and apply them.
///
/// Returns any validation issues discovered while parsing shared source values.
pub fn reload_scope_hash_profiles_from_env() -> Vec<HashSourceValidationError> {
    let (profiles, errors) = build_scope_profiles_from_source(env_lookup);
    let _ = set_scope_hash_profiles(profiles);
    errors
}

/// Hash configuration for scope generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScopeHashCfg<'a> {
    /// CSS identifier prefix (should start with a letter, recommended).
    pub prefix: &'a str,
    /// Mixed into the hash input first; use for versioning/salting.
    pub version_tag: &'a [u8],
    /// Separator between prefix and encoded hash. Must be CSS-safe.
    pub separator: char,
    /// Hash body length (clamped to `1..=MAX_SCOPE_HASH_LEN`).
    ///
    /// Body pattern is alpha-first + base62 tail.
    pub encoded_len: usize,
}

impl<'a> Default for ScopeHashCfg<'a> {
    #[inline]
    fn default() -> Self {
        Self {
            prefix: "sc",
            version_tag: b"sc:v1",
            separator: '_',
            encoded_len: MAX_SCOPE_HASH_LEN,
        }
    }
}

/// Enterprise-friendly hasher facade.
///
/// Prefer calling `ScopeHasher::generate(..)`.
pub struct ScopeHasher;

impl ScopeHasher {
    /// Generate a CSS-safe scope string like `sc_a1b2c3d`.
    ///
    /// - `content`: typically the CSS content
    /// - `file_path`: optional extra uniqueness (e.g. module path)
    #[inline]
    #[must_use]
    pub fn generate(content: &str, file_path: Option<&str>) -> String {
        let params = scope_hash_params_for(ScopeHashTarget::Scope);
        Self::generate_with_cfg(content, file_path, params.as_cfg())
    }

    /// Same as [`generate`] but configurable (prefix/version/separator).
    #[inline]
    #[must_use]
    pub fn generate_with_cfg(
        content: &str,
        file_path: Option<&str>,
        cfg: ScopeHashCfg<'_>,
    ) -> String {
        let mut b = ScopeHashBuilder::new(cfg);

        // stable delimiter so ("ab","c") != ("a","bc")
        b.update_opt_str(file_path);
        if file_path.is_some() {
            b.update_bytes(b"::");
        }
        b.update_str(content);

        let scope = b.finish();
        debug_assert!(ScopeHasher::looks_like_generated_scope(&scope, cfg));
        scope
    }

    /// Validate that a scope is CSS-friendly.
    #[inline]
    #[must_use]
    pub fn looks_like_generated_scope(s: &str, cfg: ScopeHashCfg<'_>) -> bool {
        let Some((pfx, rest)) = s.split_once(cfg.separator) else {
            return false;
        };
        if pfx != cfg.prefix {
            return false;
        }
        !rest.is_empty() && rest.chars().all(|c| c.is_ascii_alphanumeric())
    }

    /// Format `prefix + separator + short-hash` into a `String`.
    ///
    /// Emitted hash body:
    /// - starts with alphabetic ASCII char (`a-zA-Z`)
    /// - remaining chars are base62 (`0-9a-zA-Z`)
    /// - max length is `MAX_SCOPE_HASH_LEN`
    #[inline]
    #[must_use]
    pub fn format_scope(cfg: ScopeHashCfg<'_>, hash: u64) -> String {
        let hash_len = cfg.encoded_len.clamp(1, MAX_SCOPE_HASH_LEN);
        let mut out = String::with_capacity(cfg.prefix.len() + 1 + hash_len);
        out.push_str(cfg.prefix);
        out.push(cfg.separator);
        Self::push_compact_alpha_base62(&mut out, hash, hash_len);
        out
    }

    #[inline]
    fn push_base62_u64(out: &mut String, mut num: u64) {
        const CH: &[u8; 62] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

        if num == 0 {
            out.push('0');
            return;
        }

        // Max base62(u64) length is 11 chars.
        let mut buf = [0u8; 11];
        let mut i = buf.len();

        while num != 0 {
            let rem = (num % 62) as usize;
            num /= 62;
            i -= 1;
            buf[i] = CH[rem];
        }

        let s = core::str::from_utf8(&buf[i..]).unwrap();
        out.push_str(s);
    }

    #[inline]
    fn push_compact_alpha_base62(out: &mut String, hash: u64, len: usize) {
        const ALPHA: &[u8; 52] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

        let lead = ALPHA[(hash % 52) as usize] as char;
        out.push(lead);

        if len <= 1 {
            return;
        }

        let tail_seed = hash.rotate_left(13) ^ 0x9e37_79b9_7f4a_7c15;
        let mut tail = String::new();
        Self::push_base62_u64(&mut tail, tail_seed);
        let tail_need = len - 1;
        if tail.len() > tail_need {
            tail.truncate(tail_need);
        }
        out.push_str(tail.as_str());
    }
}

/// Streaming builder for composing scope hashes from multiple inputs.
#[derive(Clone)]
pub struct ScopeHashBuilder<'a> {
    cfg: ScopeHashCfg<'a>,
    hasher: Xxh3,
}

impl<'a> ScopeHashBuilder<'a> {
    #[inline]
    #[must_use]
    pub fn new(cfg: ScopeHashCfg<'a>) -> Self {
        let mut hasher = Xxh3::new();
        hasher.update(cfg.version_tag);
        hasher.update(b"|");
        Self { cfg, hasher }
    }

    #[inline]
    pub fn update_bytes(&mut self, bytes: &[u8]) -> &mut Self {
        self.hasher.update(bytes);
        self
    }

    #[inline]
    pub fn update_str(&mut self, s: &str) -> &mut Self {
        self.hasher.update(s.as_bytes());
        self
    }

    #[inline]
    pub fn update_opt_str(&mut self, s: Option<&str>) -> &mut Self {
        if let Some(v) = s {
            self.update_str(v);
        }
        self
    }

    #[inline]
    #[must_use]
    pub fn finish_u64(self) -> u64 {
        self.hasher.digest()
    }

    #[inline]
    #[must_use]
    pub fn finish(self) -> String {
        let cfg = self.cfg;
        let hash = self.finish_u64();
        ScopeHasher::format_scope(cfg, hash)
    }
}

// Runtime hash facade (shared with cssinjs).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HashProfileTarget {
    Global,
    CssInJsUnique,
    CssInJsCache,
    CssInJsClass,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HashCtorParams {
    pub version_tag: Vec<u8>,
    pub prefix: Option<String>,
    pub encoded_len: Option<usize>,
}

impl HashCtorParams {
    #[inline]
    #[must_use]
    pub fn new(version_tag: impl AsRef<[u8]>) -> Self {
        Self {
            version_tag: version_tag.as_ref().to_vec(),
            prefix: None,
            encoded_len: None,
        }
    }

    #[inline]
    #[must_use]
    pub fn cssinjs_default() -> Self {
        Self::new(b"cssinjs:v1")
    }

    #[inline]
    #[must_use]
    pub fn version_tag_slice(&self) -> &[u8] {
        self.version_tag.as_slice()
    }
}

impl Default for HashCtorParams {
    #[inline]
    fn default() -> Self {
        Self::cssinjs_default()
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HashProfiles {
    pub global: HashCtorParams,
    pub cssinjs_unique: HashCtorParams,
    pub cssinjs_cache: HashCtorParams,
    pub cssinjs_class: HashCtorParams,
}

impl Default for HashProfiles {
    #[inline]
    fn default() -> Self {
        Self {
            global: HashCtorParams::cssinjs_default(),
            cssinjs_unique: HashCtorParams::cssinjs_default(),
            cssinjs_cache: HashCtorParams::cssinjs_default(),
            cssinjs_class: HashCtorParams {
                encoded_len: Some(6),
                ..HashCtorParams::cssinjs_default()
            },
        }
    }
}

/// Serializable shared hash facade configuration consumed by macro/runtime/generator layers.
///
/// Both fields are optional to allow partial patch-style payloads.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StyleHashFacadeConfig {
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub scope_profiles: Option<ScopeHashProfiles>,
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub runtime_profiles: Option<HashProfiles>,
}

impl StyleHashFacadeConfig {
    #[inline]
    #[must_use]
    pub fn from_current() -> Self {
        Self {
            scope_profiles: Some(scope_hash_profiles()),
            runtime_profiles: Some(hash_profiles()),
        }
    }
}

/// Apply shared hash facade config to runtime profiles.
///
/// Precedence should be handled by caller. This function only applies what is provided.
#[inline]
pub fn apply_style_hash_facade_config(next: &StyleHashFacadeConfig) -> bool {
    let mut changed = false;
    if let Some(scope_profiles) = &next.scope_profiles {
        changed |= set_scope_hash_profiles(scope_profiles.clone());
    }
    if let Some(runtime_profiles) = &next.runtime_profiles {
        changed |= set_hash_profiles(runtime_profiles.clone());
    }
    changed
}

/// Resolve scope+runtime profiles from a map-based source into one facade payload.
#[must_use]
pub fn resolve_style_hash_facade_config_from_map(
    source: &BTreeMap<String, String>,
) -> (StyleHashFacadeConfig, Vec<HashSourceValidationError>) {
    let (scope_profiles, mut scope_errors) = resolve_scope_hash_profiles_from_map(source);
    let (runtime_profiles, runtime_errors) = resolve_hash_profiles_from_map(source);
    scope_errors.extend(runtime_errors);
    (
        StyleHashFacadeConfig {
            scope_profiles: Some(scope_profiles),
            runtime_profiles: Some(runtime_profiles),
        },
        scope_errors,
    )
}

/// Re-read scope+runtime profiles from process env and apply them as one facade payload.
///
/// Returns validation issues discovered while parsing shared source values.
pub fn reload_style_hash_facade_config_from_env() -> Vec<HashSourceValidationError> {
    let scope_errors = reload_scope_hash_profiles_from_env();
    let runtime_errors = reload_hash_profiles_from_env();
    let mut errors = scope_errors;
    errors.extend(runtime_errors);
    errors
}

static HASH_PROFILES: OnceLock<RwLock<HashProfiles>> = OnceLock::new();

#[inline]
fn runtime_hash_profiles_lock() -> &'static RwLock<HashProfiles> {
    HASH_PROFILES.get_or_init(|| {
        let (profiles, errors) = build_runtime_profiles_from_source(env_lookup);
        report_source_errors(errors.as_slice());
        RwLock::new(profiles)
    })
}

#[inline]
#[must_use]
pub fn hash_profiles() -> HashProfiles {
    match runtime_hash_profiles_lock().read() {
        Ok(guard) => guard.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    }
}

#[inline]
pub fn set_hash_profiles(next: HashProfiles) -> bool {
    match runtime_hash_profiles_lock().write() {
        Ok(mut guard) => {
            if *guard == next {
                false
            } else {
                *guard = next;
                true
            }
        }
        Err(poisoned) => {
            let mut guard = poisoned.into_inner();
            if *guard == next {
                false
            } else {
                *guard = next;
                true
            }
        }
    }
}

#[inline]
pub fn reset_hash_profiles() {
    let (profiles, _) = build_runtime_profiles_from_source(env_lookup);
    let _ = set_hash_profiles(profiles);
}

#[inline]
pub fn set_hash_profile_global(next: HashCtorParams) {
    let mut profiles = hash_profiles();
    profiles.global = next.clone();
    profiles.cssinjs_unique = next.clone();
    profiles.cssinjs_cache = next.clone();
    profiles.cssinjs_class = next;
    let _ = set_hash_profiles(profiles);
}

#[inline]
pub fn set_hash_profile_for(target: HashProfileTarget, next: HashCtorParams) {
    let mut profiles = hash_profiles();
    match target {
        HashProfileTarget::Global => profiles.global = next,
        HashProfileTarget::CssInJsUnique => profiles.cssinjs_unique = next,
        HashProfileTarget::CssInJsCache => profiles.cssinjs_cache = next,
        HashProfileTarget::CssInJsClass => profiles.cssinjs_class = next,
    }
    let _ = set_hash_profiles(profiles);
}

#[inline]
#[must_use]
pub fn hash_profile_for(target: HashProfileTarget) -> HashCtorParams {
    let profiles = hash_profiles();
    match target {
        HashProfileTarget::Global => profiles.global,
        HashProfileTarget::CssInJsUnique => profiles.cssinjs_unique,
        HashProfileTarget::CssInJsCache => profiles.cssinjs_cache,
        HashProfileTarget::CssInJsClass => profiles.cssinjs_class,
    }
}

/// Resolve runtime hash profiles from a map-based shared source.
#[must_use]
pub fn resolve_hash_profiles_from_map(
    source: &BTreeMap<String, String>,
) -> (HashProfiles, Vec<HashSourceValidationError>) {
    build_runtime_profiles_from_source(|key| source.get(key).cloned())
}

/// Re-read runtime hash profiles from process env and apply them.
///
/// Returns any validation issues discovered while parsing shared source values.
pub fn reload_hash_profiles_from_env() -> Vec<HashSourceValidationError> {
    let (profiles, errors) = build_runtime_profiles_from_source(env_lookup);
    let _ = set_hash_profiles(profiles);
    errors
}

/// Validate shared hash source values from process env for both scope and runtime profiles.
#[must_use]
pub fn validate_shared_hash_source_from_env() -> Vec<HashSourceValidationError> {
    let (_, mut errors) = build_scope_profiles_from_source(env_lookup);
    let (_, runtime_errors) = build_runtime_profiles_from_source(env_lookup);
    errors.extend(runtime_errors);
    errors
}

#[derive(Clone)]
pub struct Hash64Builder {
    hasher: Xxh3,
}

impl Hash64Builder {
    #[inline]
    #[must_use]
    pub fn new(version_tag: &[u8]) -> Self {
        let mut hasher = Xxh3::new();
        hasher.update(version_tag);
        hasher.update(b"|");
        Self { hasher }
    }

    #[inline]
    #[must_use]
    pub fn from_params(params: &HashCtorParams) -> Self {
        Self::new(params.version_tag_slice())
    }

    #[inline]
    #[must_use]
    pub fn for_target(target: HashProfileTarget) -> Self {
        let params = hash_profile_for(target);
        Self::new(params.version_tag_slice())
    }

    #[inline]
    pub fn update_bytes(&mut self, bytes: &[u8]) -> &mut Self {
        self.hasher.update(bytes);
        self
    }

    #[inline]
    pub fn update_str(&mut self, value: &str) -> &mut Self {
        self.hasher.update(value.as_bytes());
        self
    }

    #[inline]
    pub fn update_len_prefixed_bytes(&mut self, bytes: &[u8]) -> &mut Self {
        let len = bytes.len() as u64;
        self.hasher.update(&len.to_le_bytes());
        self.hasher.update(b":");
        self.hasher.update(bytes);
        self.hasher.update(b";");
        self
    }

    #[inline]
    #[must_use]
    pub fn finish_u64(self) -> u64 {
        self.hasher.digest()
    }
}
