use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

use serde::Serialize;

use crate::CssInJs;

// ============================================================================
// Globals
// ============================================================================

pub static STYLE_REGISTRY: OnceLock<RwLock<StyleRegistry>> = OnceLock::new();
static HEAD_CONFIG: OnceLock<RwLock<HeadStyleConfig>> = OnceLock::new();

const DEFAULT_STYLE_STYLE_ID: &str = "dx-style-global";
const DEFAULT_THEME_STYLE_ID: &str = "dx-theme-host";
const DEFAULT_SCOPE_ID_PREFIX: &str = "dx-style-";

struct RuntimeState;

impl RuntimeState {
    #[inline]
    fn registry_lock() -> &'static RwLock<StyleRegistry> {
        STYLE_REGISTRY.get_or_init(|| RwLock::new(StyleRegistry::new()))
    }

    #[inline]
    fn head_cfg_lock() -> &'static RwLock<HeadStyleConfig> {
        HEAD_CONFIG.get_or_init(|| RwLock::new(HeadStyleConfig::default()))
    }

    #[inline]
    #[must_use]
    fn empty_css_arc() -> Arc<str> {
        static EMPTY: OnceLock<Arc<str>> = OnceLock::new();
        EMPTY.get_or_init(|| Arc::<str>::from("")).clone()
    }

    #[inline]
    #[must_use]
    fn css_for_scope(scope: &str) -> Arc<str> {
        match Self::registry_lock().read() {
            Ok(g) => g.css_for_scope(scope),
            Err(poisoned) => poisoned.into_inner().css_for_scope(scope),
        }
    }
}

// ============================================================================
// Config
// ============================================================================

/// Optional metadata for injected <style> nodes.
///
/// IMPORTANT: attribute *names* are `Option<String>` with **no fallback**.
/// We only set/remove an attribute when the corresponding `*_attr` is `Some(name)`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HeadMeta {
    /// Apply meta attrs to per-scope style nodes.
    pub apply_to_scope_styles: bool,
    /// Apply meta attrs to theme style node.
    pub apply_to_theme_style: bool,

    /// Optional values.
    pub hashed: Option<bool>,
    pub css_var_key: Option<String>,
    pub algorithm: Option<String>,
    pub theme_scope: Option<String>,

    /// Optional attribute names (only used when Some).
    pub hashed_attr: Option<String>,
    pub css_var_key_attr: Option<String>,
    pub algorithm_attr: Option<String>,
    pub theme_scope_attr: Option<String>,
}

/// Runtime options used by head style injection.
///
/// - Generic, no framework-specific assumptions.
/// - By default, keeps DOM clean: `emit_debug_attrs=false` and no scope/meta attrs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HeadStyleConfig {
    /// CSP nonce applied to injected `<style>` nodes.
    pub nonce: Option<String>,

    /// Optional global style CSS (generated app/component CSS).
    /// When set (non-empty), a dedicated style node is ensured in `<head>`.
    pub style_css: Option<String>,
    /// Optional global style CSS URL.
    pub style_url: Option<String>,

    /// DOM id for the global style tag.
    pub style_style_id: Option<String>,

    /// Optional theme CSS (tokens, variables, etc).
    /// When set (non-empty), a dedicated theme style node is ensured in `<head>`.
    pub theme_css: Option<String>,
    /// Optional theme CSS URL.
    pub theme_url: Option<String>,

    /// DOM id for the theme style tag.
    pub theme_style_id: Option<String>,

    /// Prefix for per-scope injected style node IDs.
    /// Resulting id = `{scope_id_prefix}{scope}`.
    pub scope_id_prefix: Option<String>,

    /// If true, emit debug attributes on injected styles.
    /// If false, keep DOM clean (only id + nonce + text).
    pub emit_debug_attrs: bool,

    /// Optional attribute name to mark scope style nodes (value is `scope`).
    /// Example: Some("data-css-hash").
    pub scope_attr_name: Option<String>,

    /// Extra attributes applied to injected scope `<style>` nodes (only when `emit_debug_attrs=true`).
    pub extra_attrs: Vec<(String, String)>,

    /// Optional “meta” attrs. Only applied when:
    /// - `emit_debug_attrs=true`
    /// - `meta=Some(...)`
    /// - plus the meta’s own apply flags.
    pub meta: Option<HeadMeta>,

    /// Generic ordering hook:
    /// If set, after our styles are inserted we move the first matching element to the end of `<head>`.
    /// Useful when you want a particular stylesheet/link to remain last.
    pub keep_last_selector: Option<String>,
}

impl HeadStyleConfig {
    #[inline]
    #[must_use]
    fn resolved_style_style_id(&self) -> Option<&str> {
        match self.style_style_id.as_deref() {
            Some(raw) => {
                let trimmed = raw.trim();
                (!trimmed.is_empty()).then_some(trimmed)
            }
            None => Some(DEFAULT_STYLE_STYLE_ID),
        }
    }

    #[inline]
    #[must_use]
    fn resolved_theme_style_id(&self) -> Option<&str> {
        match self.theme_style_id.as_deref() {
            Some(raw) => {
                let trimmed = raw.trim();
                (!trimmed.is_empty()).then_some(trimmed)
            }
            None => Some(DEFAULT_THEME_STYLE_ID),
        }
    }

    #[inline]
    #[must_use]
    fn resolved_scope_id_prefix(&self) -> Option<&str> {
        match self.scope_id_prefix.as_deref() {
            Some(raw) => {
                let trimmed = raw.trim();
                (!trimmed.is_empty()).then_some(trimmed)
            }
            None => Some(DEFAULT_SCOPE_ID_PREFIX),
        }
    }

    #[inline]
    #[must_use]
    fn resolved_nonce(&self) -> Option<&str> {
        self.nonce
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
    }

    #[inline]
    #[must_use]
    fn resolved_theme_css(&self) -> Option<&str> {
        self.theme_css
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
    }

    #[inline]
    #[must_use]
    fn resolved_style_css(&self) -> Option<&str> {
        self.style_css
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
    }

    #[inline]
    #[must_use]
    #[allow(dead_code)]
    fn resolved_theme_url(&self) -> Option<&str> {
        self.theme_url
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
    }

    #[inline]
    #[must_use]
    #[allow(dead_code)]
    fn resolved_style_url(&self) -> Option<&str> {
        self.style_url
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
    }

    #[inline]
    #[must_use]
    #[cfg(target_arch = "wasm32")]
    fn resolved_scope_attr_name(&self) -> Option<&str> {
        self.scope_attr_name
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
    }
}

/// Serialized scope style metadata for SSR.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SsrScopeStyle {
    pub style_id: String,
    pub scope: String,
    pub css: String,
    pub tier: StyleTier,
    pub layer: Option<String>,
    pub hash: Option<String>,
    pub rewrite_signature: Option<String>,
    pub nonce: Option<String>,
}

/// Serialized theme style metadata for SSR.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SsrThemeStyle {
    pub style_id: String,
    pub css: String,
    pub nonce: Option<String>,
}

/// Serialized global style metadata for SSR.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SsrGlobalStyle {
    pub style_id: String,
    pub css: String,
    pub nonce: Option<String>,
}

#[inline]
#[must_use]
pub fn head_style_config() -> HeadStyleConfig {
    match RuntimeState::head_cfg_lock().read() {
        Ok(g) => g.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    }
}

#[inline]
pub fn set_head_style_config(next: HeadStyleConfig) -> bool {
    let changed = match RuntimeState::head_cfg_lock().write() {
        Ok(mut g) => {
            if *g == next {
                false
            } else {
                *g = next;
                true
            }
        }
        Err(poisoned) => {
            let mut g = poisoned.into_inner();
            if *g == next {
                false
            } else {
                *g = next;
                true
            }
        }
    };

    #[cfg(target_arch = "wasm32")]
    if changed {
        WasmHeadInjector::sync_theme_and_ordering_now();
    }

    changed
}

/// Compatibility switch used by macro-generated fallback `<style>` nodes.
///
/// This is intentionally narrow: the macro can only emit a fixed `data-css-hash`
/// attribute name, so we expose whether that specific debug marker is enabled.
#[inline]
#[must_use]
pub fn should_emit_scope_hash_attr() -> bool {
    let cfg = head_style_config();
    cfg.emit_debug_attrs
        && cfg
            .scope_attr_name
            .as_deref()
            .is_some_and(|name| name.trim() == "data-css-hash")
}

#[inline]
fn auto_freeze_after_inject_enabled() -> bool {
    static AUTO_FREEZE: OnceLock<bool> = OnceLock::new();
    *AUTO_FREEZE.get_or_init(|| {
        std::env::var("DIOXUS_STYLE_FREEZE_AFTER_INJECT")
            .ok()
            .is_some_and(|raw| {
                matches!(
                    raw.trim().to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                )
            })
    })
}

// ============================================================================
// Registry
// ============================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StyleTier {
    Base,
    Scoped,
    Runtime,
    Override,
}

impl StyleTier {
    #[inline]
    #[must_use]
    fn rank(self) -> u8 {
        match self {
            Self::Base => 0,
            Self::Scoped => 1,
            Self::Runtime => 2,
            Self::Override => 3,
        }
    }

    #[inline]
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Base => "base",
            Self::Scoped => "scoped",
            Self::Runtime => "runtime",
            Self::Override => "override",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StyleEntry {
    pub id: String,
    pub scope: String,
    pub css: String,
    pub tier: StyleTier,
    pub order: i32,
    pub layer: Option<String>,
    pub rewrite_signature: Option<String>,
    pub hash: Option<String>,
    pub rewrite_enabled: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RuntimeStyleRecord {
    pub cache_key: String,
    pub style_id: String,
    pub scope: String,
    pub tier: String,
    pub order: i32,
    pub layer: Option<String>,
    pub hash: Option<String>,
    pub rewrite_signature: Option<String>,
    pub rewrite_enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeStyleCssEntry {
    pub cache_key: String,
    pub css: Arc<str>,
}

impl StyleEntry {
    #[inline]
    #[must_use]
    pub fn new(id: impl Into<String>, scope: impl Into<String>, css: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            scope: scope.into(),
            css: css.into(),
            tier: StyleTier::Scoped,
            order: 0,
            layer: None,
            rewrite_signature: None,
            hash: None,
            rewrite_enabled: false,
        }
    }

    #[inline]
    #[must_use]
    pub fn scoped_static(scope: &'static str, css: &'static str) -> Self {
        Self::new(scope, scope, css).with_tier(StyleTier::Scoped)
    }

    #[inline]
    #[must_use]
    pub fn with_tier(mut self, tier: StyleTier) -> Self {
        self.tier = tier;
        self
    }

    #[inline]
    #[must_use]
    pub fn with_layer(mut self, layer: Option<String>) -> Self {
        self.layer = layer.and_then(|v| {
            let trimmed = v.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        });
        self
    }

    #[inline]
    #[must_use]
    pub fn with_order(mut self, order: i32) -> Self {
        self.order = order;
        self
    }

    #[inline]
    #[must_use]
    pub fn with_rewrite_signature(mut self, rewrite_signature: Option<String>) -> Self {
        self.rewrite_signature = rewrite_signature.and_then(|v| {
            let trimmed = v.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        });
        self
    }

    #[inline]
    #[must_use]
    pub fn with_hash(mut self, hash: Option<String>) -> Self {
        self.hash = hash.and_then(|v| {
            let trimmed = v.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        });
        self
    }

    #[inline]
    #[must_use]
    pub fn with_rewrite_enabled(mut self, rewrite_enabled: bool) -> Self {
        self.rewrite_enabled = rewrite_enabled;
        self
    }

    #[inline]
    #[must_use]
    pub fn cache_key(&self) -> String {
        let mut key = String::with_capacity(
            self.id.len()
                + self.scope.len()
                + self.rewrite_signature.as_deref().unwrap_or("").len()
                + 32,
        );
        key.push_str(self.id.as_str());
        key.push('|');
        key.push_str(self.scope.as_str());
        key.push('|');
        key.push(char::from(b'0' + self.tier.rank()));
        key.push('|');
        if let Some(sig) = self.rewrite_signature.as_deref() {
            key.push_str(sig);
        }
        key
    }

    #[inline]
    #[must_use]
    pub fn identity_key(&self) -> String {
        let mut key = String::with_capacity(
            self.id.len()
                + self.scope.len()
                + self.rewrite_signature.as_deref().unwrap_or("").len()
                + 16,
        );
        key.push_str(self.id.as_str());
        key.push('|');
        key.push_str(self.scope.as_str());
        key.push('|');
        if let Some(sig) = self.rewrite_signature.as_deref() {
            key.push_str(sig);
        }
        key
    }

    #[inline]
    #[must_use]
    fn normalized(self) -> Option<Self> {
        let id = self.id.trim();
        let scope = self.scope.trim();
        let css = self.css.trim();
        if id.is_empty() || scope.is_empty() || css.is_empty() {
            return None;
        }
        Some(Self {
            id: id.to_string(),
            scope: scope.to_string(),
            css: css.to_string(),
            tier: self.tier,
            order: self.order,
            layer: self.layer,
            rewrite_signature: self.rewrite_signature,
            hash: self.hash,
            rewrite_enabled: self.rewrite_enabled,
        })
    }
}

#[derive(Debug, Clone)]
struct StoredStyleEntry {
    identity_key: Arc<str>,
    id: Arc<str>,
    scope: Arc<str>,
    css: Arc<str>,
    tier: StyleTier,
    layer: Option<Arc<str>>,
    order: i32,
    rewrite_signature: Option<Arc<str>>,
    hash: Option<Arc<str>>,
    rewrite_enabled: bool,
    insertion_order: u64,
}

impl StoredStyleEntry {
    #[inline]
    #[must_use]
    fn from_entry(entry: StyleEntry, insertion_order: u64, identity_key: String) -> Self {
        Self {
            identity_key: Arc::<str>::from(identity_key),
            id: Arc::<str>::from(entry.id),
            scope: Arc::<str>::from(entry.scope),
            css: Arc::<str>::from(entry.css),
            tier: entry.tier,
            order: entry.order,
            layer: entry.layer.map(Arc::<str>::from),
            rewrite_signature: entry.rewrite_signature.map(Arc::<str>::from),
            hash: entry.hash.map(Arc::<str>::from),
            rewrite_enabled: entry.rewrite_enabled,
            insertion_order,
        }
    }

    #[inline]
    fn update_from_entry(&mut self, entry: StyleEntry) -> bool {
        let mut changed = false;

        if self.css.as_ref() != entry.css.as_str() {
            self.css = Arc::<str>::from(entry.css);
            changed = true;
        }
        if self.id.as_ref() != entry.id.as_str() {
            self.id = Arc::<str>::from(entry.id);
            changed = true;
        }
        if self.scope.as_ref() != entry.scope.as_str() {
            self.scope = Arc::<str>::from(entry.scope);
            changed = true;
        }
        if self.tier != entry.tier {
            self.tier = entry.tier;
            changed = true;
        }
        if self.order != entry.order {
            self.order = entry.order;
            changed = true;
        }

        let next_layer = entry.layer.map(Arc::<str>::from);
        if self.layer != next_layer {
            self.layer = next_layer;
            changed = true;
        }

        let next_rewrite_signature = entry.rewrite_signature.map(Arc::<str>::from);
        if self.rewrite_signature != next_rewrite_signature {
            self.rewrite_signature = next_rewrite_signature;
            changed = true;
        }

        let next_hash = entry.hash.map(Arc::<str>::from);
        if self.hash != next_hash {
            self.hash = next_hash;
            changed = true;
        }

        if self.rewrite_enabled != entry.rewrite_enabled {
            self.rewrite_enabled = entry.rewrite_enabled;
            changed = true;
        }

        changed
    }
}

/// Registry that tracks all styles in the application with deterministic ordering.
#[derive(Debug)]
pub struct StyleRegistry {
    entries: HashMap<String, StoredStyleEntry>,
    identity_index: HashMap<String, String>,
    scope_index: HashMap<String, Vec<String>>,
    insertion_counter: u64,

    cached_output: Arc<str>,
    dirty: bool,

    frozen: bool,
}

impl Default for StyleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl StyleRegistry {
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: HashMap::with_capacity(32),
            identity_index: HashMap::with_capacity(32),
            scope_index: HashMap::with_capacity(32),
            insertion_counter: 0,
            cached_output: Arc::<str>::from(""),
            dirty: false,
            frozen: false,
        }
    }

    /// Freeze registry to disallow further mutations.
    /// Useful in production after all styles are known to be registered.
    #[inline]
    pub fn freeze(&mut self) {
        let _ = self.get_all_styles_arc();
        self.frozen = true;
    }

    #[inline]
    #[must_use]
    pub fn is_frozen(&self) -> bool {
        self.frozen
    }

    #[inline]
    #[must_use]
    fn layer_order_key(layer: Option<&str>) -> (u8, &str) {
        match layer {
            Some(name) if !name.trim().is_empty() => (0, name),
            _ => (1, ""),
        }
    }

    #[inline]
    #[must_use]
    fn compare_priority(a: &StoredStyleEntry, b: &StoredStyleEntry) -> Ordering {
        a.tier
            .rank()
            .cmp(&b.tier.rank())
            .then_with(|| a.order.cmp(&b.order))
            .then_with(|| {
                Self::layer_order_key(a.layer.as_deref())
                    .cmp(&Self::layer_order_key(b.layer.as_deref()))
            })
            .then_with(|| a.insertion_order.cmp(&b.insertion_order))
            .then_with(|| a.scope.cmp(&b.scope))
            .then_with(|| a.id.cmp(&b.id))
    }

    #[inline]
    #[must_use]
    fn best_entry_for_scope_ref(&self, scope: &str) -> Option<&StoredStyleEntry> {
        let keys = self.scope_index.get(scope)?;
        let mut best: Option<&StoredStyleEntry> = None;
        for key in keys {
            if let Some(entry) = self.entries.get(key) {
                best = match best {
                    None => Some(entry),
                    Some(current) => {
                        if Self::compare_priority(entry, current) == Ordering::Greater {
                            Some(entry)
                        } else {
                            Some(current)
                        }
                    }
                };
            }
        }
        best
    }

    #[inline]
    fn remove_entry_by_key(&mut self, key: &str) -> Option<StoredStyleEntry> {
        let removed = self.entries.remove(key)?;

        if let Some(keys) = self.scope_index.get_mut(removed.scope.as_ref()) {
            keys.retain(|k| k != key);
            if keys.is_empty() {
                self.scope_index.remove(removed.scope.as_ref());
            }
        }

        if let Some(active_key) = self.identity_index.get(removed.identity_key.as_ref())
            && active_key == key
        {
            self.identity_index.remove(removed.identity_key.as_ref());
        }

        Some(removed)
    }

    /// Compatibility register path used by macro-generated scoped styles.
    #[inline]
    pub fn register(&mut self, scope: &'static str, css: &'static str) {
        let _ = self.register_or_update(StyleEntry::scoped_static(scope, css));
    }

    /// Register/update style entry using deterministic keying and tiered ordering.
    ///
    /// Returns `true` when registry content changed.
    #[inline]
    pub fn register_or_update(&mut self, entry: StyleEntry) -> bool {
        if self.frozen {
            return false;
        }

        let Some(entry) = entry.normalized() else {
            return false;
        };

        let identity_key = entry.identity_key();
        let key = entry.cache_key();

        if let Some(active_key) = self.identity_index.get(identity_key.as_str()).cloned()
            && active_key != key
            && let Some(active_entry) = self.entries.get(&active_key)
        {
            // Cross-tier collision policy:
            // For the same (id, scope, rewrite_signature), keep the highest tier.
            // Lower-tier writes are ignored; higher-tier writes replace the existing entry.
            if entry.tier.rank() < active_entry.tier.rank() {
                return false;
            }
            let _ = self.remove_entry_by_key(active_key.as_str());
        }

        if let Some(existing) = self.entries.get_mut(&key) {
            let changed = existing.update_from_entry(entry);
            if changed {
                self.dirty = true;
            }
            self.identity_index.insert(identity_key, key);
            return changed;
        }

        let stored =
            StoredStyleEntry::from_entry(entry, self.insertion_counter, identity_key.clone());
        self.insertion_counter = self.insertion_counter.saturating_add(1);

        let scope_key = stored.scope.to_string();
        self.scope_index
            .entry(scope_key)
            .or_default()
            .push(key.clone());
        self.entries.insert(key.clone(), stored);
        self.identity_index.insert(identity_key, key);
        self.dirty = true;
        true
    }

    /// Gets all registered styles as a single CSS string (Arc-backed).
    /// - O(1) clone when cache is clean
    /// - rebuild only when dirty
    #[inline]
    #[must_use]
    pub fn get_all_styles_arc(&mut self) -> Arc<str> {
        if !self.dirty {
            return self.cached_output.clone();
        }

        if self.entries.is_empty() {
            self.cached_output = Arc::<str>::from("");
            self.dirty = false;
            return self.cached_output.clone();
        }

        let mut ordered: Vec<&StoredStyleEntry> = self.entries.values().collect();
        ordered.sort_by(|a, b| Self::compare_priority(a, b));

        let total_size: usize = ordered.iter().map(|entry| entry.css.len() + 1).sum();

        let mut out = String::with_capacity(total_size);
        for entry in ordered {
            out.push_str(entry.css.as_ref());
            out.push('\n');
        }

        self.cached_output = Arc::<str>::from(out);
        self.dirty = false;
        self.cached_output.clone()
    }

    #[inline]
    #[must_use]
    pub fn contains(&self, scope: &str) -> bool {
        self.scope_index.contains_key(scope)
    }

    /// Clears all registered styles. No-op if frozen.
    #[inline]
    pub fn clear(&mut self) {
        if self.frozen {
            return;
        }
        self.entries.clear();
        self.identity_index.clear();
        self.scope_index.clear();
        self.insertion_counter = 0;
        self.cached_output = Arc::<str>::from("");
        self.dirty = false;
    }

    /// Force clear even if frozen (intended for tests/tools).
    #[inline]
    pub fn clear_force(&mut self) {
        self.entries.clear();
        self.identity_index.clear();
        self.scope_index.clear();
        self.insertion_counter = 0;
        self.cached_output = Arc::<str>::from("");
        self.dirty = false;
        self.frozen = false;
    }

    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[inline]
    #[must_use]
    pub fn css_for_scope(&self, scope: &str) -> Arc<str> {
        self.best_entry_for_scope_ref(scope)
            .map_or_else(RuntimeState::empty_css_arc, |entry| entry.css.clone())
    }

    #[inline]
    #[must_use]
    pub fn ordered_scopes(&self) -> Vec<String> {
        let mut scopes: Vec<(String, &StoredStyleEntry)> =
            Vec::with_capacity(self.scope_index.len());
        for scope in self.scope_index.keys() {
            if let Some(entry) = self.best_entry_for_scope_ref(scope.as_str()) {
                scopes.push((scope.clone(), entry));
            }
        }

        scopes.sort_by(|(scope_a, entry_a), (scope_b, entry_b)| {
            Self::compare_priority(entry_a, entry_b).then_with(|| scope_a.cmp(scope_b))
        });

        scopes.into_iter().map(|(scope, _)| scope).collect()
    }

    #[inline]
    #[must_use]
    pub fn export_ssr_scope_styles(&self, cfg: &HeadStyleConfig) -> Vec<SsrScopeStyle> {
        let nonce = cfg.resolved_nonce().map(ToOwned::to_owned);
        let scope_id_prefix = cfg
            .resolved_scope_id_prefix()
            .unwrap_or(DEFAULT_SCOPE_ID_PREFIX);

        let scopes = self.ordered_scopes();
        let mut out = Vec::with_capacity(scopes.len());
        for scope in scopes {
            if let Some(entry) = self.best_entry_for_scope_ref(scope.as_str()) {
                out.push(SsrScopeStyle {
                    style_id: format!("{scope_id_prefix}{scope}"),
                    scope: scope.clone(),
                    css: entry.css.to_string(),
                    tier: entry.tier,
                    layer: entry.layer.as_deref().map(ToOwned::to_owned),
                    hash: entry.hash.as_deref().map(ToOwned::to_owned),
                    rewrite_signature: entry.rewrite_signature.as_deref().map(ToOwned::to_owned),
                    nonce: nonce.clone(),
                });
            }
        }
        out
    }

    #[inline]
    #[must_use]
    pub fn runtime_style_records(&self) -> Vec<RuntimeStyleRecord> {
        let mut ordered: Vec<(&String, &StoredStyleEntry)> = self.entries.iter().collect();
        ordered.sort_by(|(key_a, a), (key_b, b)| {
            Self::compare_priority(a, b).then_with(|| key_a.cmp(key_b))
        });

        ordered
            .into_iter()
            .map(|(cache_key, entry)| RuntimeStyleRecord {
                cache_key: cache_key.clone(),
                style_id: entry.id.to_string(),
                scope: entry.scope.to_string(),
                tier: entry.tier.as_str().to_string(),
                order: entry.order,
                layer: entry.layer.as_deref().map(ToOwned::to_owned),
                hash: entry.hash.as_deref().map(ToOwned::to_owned),
                rewrite_signature: entry.rewrite_signature.as_deref().map(ToOwned::to_owned),
                rewrite_enabled: entry.rewrite_enabled,
            })
            .collect()
    }

    #[inline]
    #[must_use]
    pub fn runtime_style_css_entries(&self) -> Vec<RuntimeStyleCssEntry> {
        let mut ordered: Vec<(&String, &StoredStyleEntry)> = self.entries.iter().collect();
        ordered.sort_by(|(key_a, a), (key_b, b)| {
            Self::compare_priority(a, b).then_with(|| key_a.cmp(key_b))
        });

        ordered
            .into_iter()
            .map(|(cache_key, entry)| RuntimeStyleCssEntry {
                cache_key: cache_key.clone(),
                css: entry.css.clone(),
            })
            .collect()
    }
}

// ============================================================================
// Public APIs
// ============================================================================

/// Fast injection path: Arc-backed CSS snapshot.
/// - Read-lock + Arc clone when cache is clean.
/// - Write-lock rebuild only when dirty.
#[inline]
#[must_use]
pub fn inject_styles_arc() -> Arc<str> {
    // Fast path
    if let Ok(g) = RuntimeState::registry_lock().read()
        && !g.dirty
    {
        return g.cached_output.clone();
    }

    // Slow path
    let output = match RuntimeState::registry_lock().write() {
        Ok(mut g) => g.get_all_styles_arc(),
        Err(poisoned) => poisoned.into_inner().get_all_styles_arc(),
    };

    if auto_freeze_after_inject_enabled() {
        freeze_style_registry();
    }

    output
}

/// Backward-compatible String API (allocates).
#[inline]
#[must_use]
pub fn inject_styles() -> String {
    inject_styles_arc().to_string()
}

/// Freeze the global registry (no further register/clear).
#[inline]
pub fn freeze_style_registry() {
    match RuntimeState::registry_lock().write() {
        Ok(mut g) => g.freeze(),
        Err(poisoned) => poisoned.into_inner().freeze(),
    }
}

/// Register or update a style entry in the shared injector registry.
///
/// Returns `true` when registry content changed.
#[inline]
pub fn register_or_update(entry: StyleEntry) -> bool {
    match RuntimeState::registry_lock().write() {
        Ok(mut g) => g.register_or_update(entry),
        Err(poisoned) => poisoned.into_inner().register_or_update(entry),
    }
}

#[inline]
#[must_use]
pub fn runtime_style_records() -> Vec<RuntimeStyleRecord> {
    match RuntimeState::registry_lock().read() {
        Ok(g) => g.runtime_style_records(),
        Err(poisoned) => poisoned.into_inner().runtime_style_records(),
    }
}

#[inline]
#[must_use]
pub fn runtime_style_css_entries() -> Vec<RuntimeStyleCssEntry> {
    match RuntimeState::registry_lock().read() {
        Ok(g) => g.runtime_style_css_entries(),
        Err(poisoned) => poisoned.into_inner().runtime_style_css_entries(),
    }
}

/// Export scope styles for SSR using the same ordering semantics as head injection.
#[inline]
#[must_use]
pub fn export_ssr_scope_styles() -> Vec<SsrScopeStyle> {
    let cfg = head_style_config();
    match RuntimeState::registry_lock().read() {
        Ok(g) => g.export_ssr_scope_styles(&cfg),
        Err(poisoned) => poisoned.into_inner().export_ssr_scope_styles(&cfg),
    }
}

/// Export theme style for SSR, if configured.
#[inline]
#[must_use]
pub fn export_ssr_theme_style() -> Option<SsrThemeStyle> {
    let cfg = head_style_config();
    let style_id = cfg.resolved_theme_style_id()?.to_string();
    let css = cfg.resolved_theme_css().map(ToOwned::to_owned)?;
    let nonce = cfg.resolved_nonce().map(ToOwned::to_owned);

    Some(SsrThemeStyle {
        style_id,
        css,
        nonce,
    })
}

/// Export global style for SSR, if configured.
#[inline]
#[must_use]
pub fn export_ssr_global_style() -> Option<SsrGlobalStyle> {
    let cfg = head_style_config();
    let style_id = cfg.resolved_style_style_id()?.to_string();
    let css = cfg.resolved_style_css().map(ToOwned::to_owned)?;
    let nonce = cfg.resolved_nonce().map(ToOwned::to_owned);

    Some(SsrGlobalStyle {
        style_id,
        css,
        nonce,
    })
}

/// Flush style side effects immediately.
///
/// - wasm32: syncs theme node and ordering in `<head>`.
/// - non-wasm: no-op.
#[inline]
pub fn flush_head() {
    #[cfg(target_arch = "wasm32")]
    {
        WasmHeadInjector::sync_theme_and_ordering_now();
    }
}

/// Inject a single scoped stylesheet by scope.
///
/// - wasm32: attempts head injection; returns empty text if injected, else returns css fallback text.
/// - non-wasm: returns css (caller renders inline).
#[inline]
#[must_use]
pub fn inject_style(scope: &str) -> Arc<str> {
    #[cfg(target_arch = "wasm32")]
    {
        WasmHeadInjector::inject_style(scope)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        RuntimeState::css_for_scope(scope)
    }
}

/// Helper struct for managing a single scoped style instance.
#[derive(Debug, Clone, Copy)]
pub struct ScopedStyle {
    scope: &'static str,
    css: &'static str,
}

impl ScopedStyle {
    #[inline]
    fn build_entry(&self) -> (StyleEntry, String) {
        let (rewritten_css, rewrite_signature, rewrite_enabled) =
            CssInJs::rewrite_for_runtime(self.css);
        let entry = StyleEntry::new(self.scope, self.scope, rewritten_css)
            .with_tier(StyleTier::Scoped)
            .with_rewrite_signature(rewrite_signature)
            .with_rewrite_enabled(rewrite_enabled);
        let key = entry.cache_key();
        (entry, key)
    }

    #[inline]
    fn ensure_registered(&self) {
        let (entry, key) = self.build_entry();

        if let Ok(g) = RuntimeState::registry_lock().read() {
            if g.frozen {
                return;
            }
            if let Some(existing) = g.entries.get(&key)
                && existing.css.as_ref() == entry.css.as_str()
            {
                return;
            }
        }

        match RuntimeState::registry_lock().write() {
            Ok(mut g) => {
                let _ = g.register_or_update(entry);
            }
            Err(poisoned) => {
                let _ = poisoned.into_inner().register_or_update(entry);
            }
        }
    }

    /// Creates a new scoped style and registers it.
    ///
    /// Update-safe fast path:
    /// - If already registered with identical css => no write lock.
    /// - If frozen => no-op.
    #[inline]
    pub fn new(scope: &'static str, css: &'static str) -> Self {
        let style = Self { scope, css };
        style.ensure_registered();
        style
    }

    #[inline]
    pub fn scope(&self) -> &'static str {
        self.ensure_registered();
        self.scope
    }

    #[inline]
    pub const fn style_id(&self) -> &'static str {
        self.scope
    }

    #[inline]
    pub const fn raw_css(&self) -> &'static str {
        self.css
    }
}

impl std::fmt::Display for ScopedStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.scope)
    }
}

// ============================================================================
// wasm32 head injection
// ============================================================================

#[cfg(target_arch = "wasm32")]
struct WasmHeadInjector;

#[cfg(target_arch = "wasm32")]
impl WasmHeadInjector {
    #[inline]
    fn inject_style(scope: &str) -> Arc<str> {
        let css = RuntimeState::css_for_scope(scope);
        if css.is_empty() {
            return RuntimeState::empty_css_arc();
        }
        if Self::try_inject_scope_into_head(scope, css.as_ref()) {
            RuntimeState::empty_css_arc()
        } else {
            css
        }
    }

    fn try_inject_scope_into_head(scope: &str, css: &str) -> bool {
        let Some(window) = web_sys::window() else {
            return false;
        };
        let Some(document) = window.document() else {
            return false;
        };

        let cfg = head_style_config();
        Self::ensure_global_style(&document, &cfg);
        Self::ensure_theme_style(&document, &cfg);

        let Some(scope_id_prefix) = cfg.resolved_scope_id_prefix() else {
            return false;
        };
        let style_id = format!("{scope_id_prefix}{scope}");

        if let Some(existing) = document.get_element_by_id(&style_id) {
            Self::apply_scope_style_attrs(&existing, scope, &cfg);
            if existing.text_content().as_deref() != Some(css) {
                existing.set_text_content(Some(css));
            }
            Self::ensure_ordering(&window, &document);
            return true;
        }

        if cfg.emit_debug_attrs
            && let Some(scope_attr) = cfg.resolved_scope_attr_name()
        {
            let selector = format!(r#"style[{}="{}"]"#, scope_attr, scope);
            if let Ok(Some(existing)) = document.query_selector(&selector) {
                Self::apply_scope_style_attrs(&existing, scope, &cfg);
                if existing.text_content().as_deref() != Some(css) {
                    existing.set_text_content(Some(css));
                }
                if existing.id() != style_id {
                    let _ = existing.set_attribute("id", &style_id);
                }
                Self::ensure_ordering(&window, &document);
                return true;
            }
        }

        let Ok(style_el) = document.create_element("style") else {
            return false;
        };
        style_el.set_id(&style_id);
        style_el.set_text_content(Some(css));
        Self::apply_scope_style_attrs(&style_el, scope, &cfg);

        if let Some(head) = document.head() {
            let ok = head.append_child(&style_el).is_ok();
            if ok {
                Self::ensure_ordering(&window, &document);
            }
            return ok;
        }
        if let Some(body) = document.body() {
            let ok = body.append_child(&style_el).is_ok();
            if ok {
                Self::ensure_ordering(&window, &document);
            }
            return ok;
        }
        false
    }

    fn apply_scope_style_attrs(el: &web_sys::Element, scope: &str, cfg: &HeadStyleConfig) {
        if let Some(nonce) = cfg.resolved_nonce() {
            let _ = el.set_attribute("nonce", nonce);
        } else {
            let _ = el.remove_attribute("nonce");
        }

        if cfg.emit_debug_attrs {
            if let Some(scope_attr) = cfg.resolved_scope_attr_name() {
                let _ = el.set_attribute(scope_attr, scope);
            }
            for (k, v) in &cfg.extra_attrs {
                let kt = k.trim();
                if !kt.is_empty() {
                    let _ = el.set_attribute(kt, v.as_str());
                }
            }
        } else {
            if let Some(scope_attr) = cfg.resolved_scope_attr_name() {
                let _ = el.remove_attribute(scope_attr);
            }
            for (k, _) in &cfg.extra_attrs {
                let kt = k.trim();
                if !kt.is_empty() {
                    let _ = el.remove_attribute(kt);
                }
            }
        }

        if cfg.emit_debug_attrs {
            if let Some(meta) = cfg.meta.as_ref().filter(|m| m.apply_to_scope_styles) {
                Self::apply_optional_meta_attrs(el, meta);
            } else if let Some(meta) = cfg.meta.as_ref() {
                Self::clear_meta_attrs(el, meta);
            }
        } else if let Some(meta) = cfg.meta.as_ref() {
            Self::clear_meta_attrs(el, meta);
        }
    }

    fn ensure_optional_style(
        document: &web_sys::Document,
        css: Option<&str>,
        style_id: Option<&str>,
        nonce: Option<&str>,
        meta: Option<&HeadMeta>,
        apply_meta: bool,
    ) {
        let Some(head) = document.head() else {
            return;
        };
        let Some(id) = style_id else {
            return;
        };

        let existing = document.get_element_by_id(id);
        if css.is_none() {
            if let Some(node) = existing {
                node.remove();
            }
            return;
        }
        let css = css.unwrap_or_default();

        if let Some(node) = existing {
            if node.text_content().as_deref() != Some(css) {
                node.set_text_content(Some(css));
            }

            if let Some(nonce) = nonce {
                let _ = node.set_attribute("nonce", nonce);
            } else {
                let _ = node.remove_attribute("nonce");
            }

            if let Some(meta) = meta {
                if apply_meta {
                    Self::apply_optional_meta_attrs(&node, meta);
                } else {
                    Self::clear_meta_attrs(&node, meta);
                }
            }
            return;
        }

        let Ok(style_el) = document.create_element("style") else {
            return;
        };
        style_el.set_id(id);
        style_el.set_text_content(Some(css));

        if let Some(nonce) = nonce {
            let _ = style_el.set_attribute("nonce", nonce);
        }

        if apply_meta && let Some(meta) = meta {
            Self::apply_optional_meta_attrs(&style_el, meta);
        }
        let _ = head.append_child(&style_el);
    }

    fn ensure_optional_link(
        document: &web_sys::Document,
        href: Option<&str>,
        link_id: Option<&str>,
        nonce: Option<&str>,
        meta: Option<&HeadMeta>,
        apply_meta: bool,
    ) {
        let Some(head) = document.head() else {
            return;
        };
        let Some(id) = link_id else {
            return;
        };

        let existing = document.get_element_by_id(id);
        if href.is_none() {
            if let Some(node) = existing {
                node.remove();
            }
            return;
        }
        let href = href.unwrap_or_default();

        if let Some(node) = existing {
            let _ = node.set_attribute("href", href);
            let _ = node.set_attribute("rel", "stylesheet");

            if let Some(nonce) = nonce {
                let _ = node.set_attribute("nonce", nonce);
            } else {
                let _ = node.remove_attribute("nonce");
            }

            if let Some(meta) = meta {
                if apply_meta {
                    Self::apply_optional_meta_attrs(&node, meta);
                } else {
                    Self::clear_meta_attrs(&node, meta);
                }
            }
            return;
        }

        let Ok(link_el) = document.create_element("link") else {
            return;
        };
        link_el.set_id(id);
        let _ = link_el.set_attribute("rel", "stylesheet");
        let _ = link_el.set_attribute("href", href);

        if let Some(nonce) = nonce {
            let _ = link_el.set_attribute("nonce", nonce);
        }

        if apply_meta && let Some(meta) = meta {
            Self::apply_optional_meta_attrs(&link_el, meta);
        }
        let _ = head.append_child(&link_el);
    }

    fn ensure_global_style(document: &web_sys::Document, cfg: &HeadStyleConfig) {
        let meta = cfg.meta.as_ref().filter(|_| cfg.emit_debug_attrs);
        if cfg.resolved_style_url().is_some() {
            Self::ensure_optional_style(
                document,
                None,
                cfg.resolved_style_style_id(),
                cfg.resolved_nonce(),
                meta,
                false,
            );
            Self::ensure_optional_link(
                document,
                cfg.resolved_style_url(),
                cfg.resolved_style_style_id(),
                cfg.resolved_nonce(),
                meta,
                false,
            );
            return;
        }
        Self::ensure_optional_style(
            document,
            cfg.resolved_style_css(),
            cfg.resolved_style_style_id(),
            cfg.resolved_nonce(),
            meta,
            false,
        );
    }

    fn ensure_theme_style(document: &web_sys::Document, cfg: &HeadStyleConfig) {
        let meta = cfg.meta.as_ref().filter(|_| cfg.emit_debug_attrs);
        if cfg.resolved_theme_url().is_some() {
            Self::ensure_optional_style(
                document,
                None,
                cfg.resolved_theme_style_id(),
                cfg.resolved_nonce(),
                meta,
                false,
            );
            Self::ensure_optional_link(
                document,
                cfg.resolved_theme_url(),
                cfg.resolved_theme_style_id(),
                cfg.resolved_nonce(),
                meta,
                meta.is_some_and(|m| m.apply_to_theme_style),
            );
            return;
        }
        Self::ensure_optional_style(
            document,
            cfg.resolved_theme_css(),
            cfg.resolved_theme_style_id(),
            cfg.resolved_nonce(),
            meta,
            meta.is_some_and(|m| m.apply_to_theme_style),
        );
    }

    fn apply_optional_meta_attrs(el: &web_sys::Element, meta: &HeadMeta) {
        Self::set_opt_bool_attr(el, meta.hashed_attr.as_deref(), meta.hashed);
        Self::set_opt_str_attr(
            el,
            meta.css_var_key_attr.as_deref(),
            meta.css_var_key.as_deref(),
        );
        Self::set_opt_str_attr(
            el,
            meta.algorithm_attr.as_deref(),
            meta.algorithm.as_deref(),
        );
        Self::set_opt_str_attr(
            el,
            meta.theme_scope_attr.as_deref(),
            meta.theme_scope.as_deref(),
        );
    }

    fn clear_meta_attrs(el: &web_sys::Element, meta: &HeadMeta) {
        Self::remove_opt_attr(el, meta.hashed_attr.as_deref());
        Self::remove_opt_attr(el, meta.css_var_key_attr.as_deref());
        Self::remove_opt_attr(el, meta.algorithm_attr.as_deref());
        Self::remove_opt_attr(el, meta.theme_scope_attr.as_deref());
    }

    fn set_opt_bool_attr(el: &web_sys::Element, name: Option<&str>, value: Option<bool>) {
        let Some(name) = name.map(str::trim).filter(|n| !n.is_empty()) else {
            return;
        };
        match value {
            Some(v) => {
                let _ = el.set_attribute(name, if v { "true" } else { "false" });
            }
            None => {
                let _ = el.remove_attribute(name);
            }
        }
    }

    fn set_opt_str_attr(el: &web_sys::Element, name: Option<&str>, value: Option<&str>) {
        let Some(name) = name.map(str::trim).filter(|n| !n.is_empty()) else {
            return;
        };
        match value.map(str::trim).filter(|v| !v.is_empty()) {
            Some(v) => {
                let _ = el.set_attribute(name, v);
            }
            None => {
                let _ = el.remove_attribute(name);
            }
        }
    }

    fn remove_opt_attr(el: &web_sys::Element, name: Option<&str>) {
        let Some(name) = name.map(str::trim).filter(|n| !n.is_empty()) else {
            return;
        };
        let _ = el.remove_attribute(name);
    }

    fn sync_theme_and_ordering_now() {
        let Some(window) = web_sys::window() else {
            return;
        };
        let Some(document) = window.document() else {
            return;
        };
        let cfg = head_style_config();
        Self::ensure_global_style(&document, &cfg);
        Self::ensure_theme_style(&document, &cfg);
        Self::ensure_ordering(&window, &document);
    }

    fn ensure_ordering(window: &web_sys::Window, document: &web_sys::Document) {
        use wasm_bindgen::JsCast;

        if document.ready_state() == "complete" {
            Self::reorder(document);
            return;
        }

        static LOAD_HOOK_INSTALLED: OnceLock<()> = OnceLock::new();
        if LOAD_HOOK_INSTALLED.set(()).is_err() {
            return;
        }

        // Self-removing pattern: the closure removes its own event listener
        // after firing, then drops itself.  This avoids `Closure::forget()`
        // which would retain a permanent JS→WASM reference preventing the
        // old WASM module from being GC'd on page reload.
        type LoadHookClosure = wasm_bindgen::closure::Closure<dyn FnMut(web_sys::Event)>;
        type LoadHookSlot = std::rc::Rc<std::cell::RefCell<Option<LoadHookClosure>>>;
        let cb_slot: LoadHookSlot = std::rc::Rc::new(std::cell::RefCell::new(None));
        let cb_slot_inner = cb_slot.clone();
        let win_for_remove = window.clone();

        let closure = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(
            move |_evt| {
                if let Some(document) = web_sys::window().and_then(|w| w.document()) {
                    Self::reorder(&document);
                }
                // Remove this listener so the closure (and its WASM reference) can be collected.
                if let Some(cb) = cb_slot_inner.borrow().as_ref() {
                    let _ = win_for_remove
                        .remove_event_listener_with_callback("load", cb.as_ref().unchecked_ref());
                }
                // Drop the closure reference.
                *cb_slot_inner.borrow_mut() = None;
            },
        ));

        let _ = window.add_event_listener_with_callback("load", closure.as_ref().unchecked_ref());
        *cb_slot.borrow_mut() = Some(closure);
    }

    fn reorder(document: &web_sys::Document) {
        let cfg = head_style_config();
        let Some(head) = document.head() else {
            return;
        };

        Self::ensure_global_style(document, &cfg);
        Self::ensure_theme_style(document, &cfg);

        let scopes: Vec<String> = match RuntimeState::registry_lock().read() {
            Ok(g) => g.ordered_scopes(),
            Err(poisoned) => poisoned.into_inner().ordered_scopes(),
        };

        let scope_id_prefix = cfg.resolved_scope_id_prefix();

        for scope in scopes {
            if let Some(prefix) = scope_id_prefix {
                let style_id = format!("{prefix}{scope}");
                if let Some(style_el) = document.get_element_by_id(&style_id) {
                    let _ = head.append_child(&style_el);
                    continue;
                }

                if cfg.emit_debug_attrs
                    && let Some(scope_attr) = cfg.resolved_scope_attr_name()
                {
                    let selector = format!(r#"style[{}="{}"]"#, scope_attr, scope);
                    if let Ok(Some(style_el)) = document.query_selector(&selector) {
                        if style_el.id() != style_id {
                            let _ = style_el.set_attribute("id", &style_id);
                        }
                        let _ = head.append_child(&style_el);
                    }
                }
                continue;
            }

            if cfg.emit_debug_attrs
                && let Some(scope_attr) = cfg.resolved_scope_attr_name()
            {
                let selector = format!(r#"style[{}="{}"]"#, scope_attr, scope);
                if let Ok(Some(style_el)) = document.query_selector(&selector) {
                    let _ = head.append_child(&style_el);
                }
            }
        }

        if let Some(sel) = cfg
            .keep_last_selector
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            && let Ok(Some(node)) = document.query_selector(sel)
        {
            let _ = head.append_child(&node);
        }
    }
}
