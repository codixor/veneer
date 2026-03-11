//! Registry for storing and composing registered styles.

use std::collections::HashMap;
use std::sync::Arc;

use serde::Serialize;

use super::CssInJsStyleInput;
use super::config::store as config_store;
use super::hash;
use super::transform;

// ----------------------------------------------------------------------------
// Entry and Registration
// ----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CssInJsRegistration {
    pub cache_key: String,
    pub hash_class: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CssInJsEntry {
    pub cache_key: String,
    pub identity_key: String,
    pub style_id: Arc<str>,
    pub identity_scope: Option<Arc<str>>,
    pub order: i32,
    pub layer: Option<Arc<str>>,
    pub css: Arc<str>,
    pub rendered_css: Arc<str>,
    pub hash_class: Arc<str>,
    pub rewrite_signature: Arc<str>,

    pub token_hash: Option<Arc<str>>,
    pub hashed: Option<bool>,
    pub css_var_key: Option<Arc<str>>,
    pub algorithm: Option<Arc<str>>,
    pub theme_scope: Option<Arc<str>>,
    pub nonce: Option<Arc<str>>,
}

impl CssInJsEntry {
    pub(crate) fn from_input(input: CssInJsStyleInput) -> Option<Self> {
        let style_id = input.style_id.as_ref().trim();
        let css = input.css.as_ref().trim();
        if style_id.is_empty() || css.is_empty() {
            return None;
        }

        let cfg = config_store::get();
        let rw = input.rewrite.as_deref().unwrap_or(&cfg.rewrite);
        let rw_fp = hash::rewrite_fingerprint(rw, &cfg);

        let hash_class = input
            .hash_class
            .as_ref()
            .map(|v| v.as_ref().trim())
            .filter(|v| !v.is_empty())
            .map(ToString::to_string)
            .unwrap_or_else(|| {
                let profile = hash::resolved_runtime_hash_profile(
                    &cfg,
                    hash::HashProfileTarget::CssInJsClass,
                );
                let h = hash::hash64_chunks(
                    &cfg,
                    hash::HashProfileTarget::CssInJsClass,
                    &[
                        style_id.as_bytes(),
                        b"|",
                        css.as_bytes(),
                        b"|",
                        input.token_hash.as_deref().unwrap_or("").as_bytes(),
                        b"|",
                        input.layer.as_deref().unwrap_or("").as_bytes(),
                        b"|",
                        &rw_fp.to_le_bytes(),
                    ],
                );
                let body =
                    hash::class_hash_u64_compact(h, cfg.class_hash_len.or(profile.encoded_len));
                if let Some(pre) = cfg
                    .class_prefix
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
            });

        let rewritten = transform::CssTransform::apply_rewrite_rules(css, rw);
        let scope_prefix = hash::scoped_hash_selector(hash_class.as_str(), input.hash_priority);

        let scoper: Arc<dyn super::config::CssScoper> = cfg
            .scoper
            .clone()
            .unwrap_or_else(|| Arc::new(super::config::DefaultCssScoper));
        let scoped = scoper.scope(&rewritten, &scope_prefix);

        let rendered_css = if let Some(layer) = input
            .layer
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            format!("@layer {layer} {{\n{scoped}\n}}\n")
        } else {
            scoped
        };

        let rewrite_signature = Arc::<str>::from(format!("rw={rw_fp:016x}"));
        let identity_key = hash::identity_key(&input, hash_class.as_str(), rw_fp);
        let cache_key = hash::cache_key(&cfg, &input, hash_class.as_str(), &rendered_css, rw_fp);

        Some(Self {
            cache_key,
            identity_key,
            style_id: Arc::<str>::from(style_id),
            identity_scope: input
                .identity_scope
                .as_ref()
                .map(|v| Arc::<str>::from(v.as_ref().trim()))
                .filter(|v| !v.is_empty()),
            order: input.order,
            layer: input
                .layer
                .as_ref()
                .map(|v| Arc::<str>::from(v.as_ref().trim()))
                .filter(|v| !v.is_empty()),
            css: Arc::<str>::from(css),
            rendered_css: Arc::<str>::from(rendered_css),
            hash_class: Arc::<str>::from(hash_class),
            rewrite_signature,

            token_hash: input
                .token_hash
                .as_ref()
                .map(|v| Arc::<str>::from(v.as_ref().trim())),
            hashed: input.hashed,
            css_var_key: input
                .css_var_key
                .as_ref()
                .map(|v| Arc::<str>::from(v.as_ref().trim())),
            algorithm: input
                .algorithm
                .as_ref()
                .map(|v| Arc::<str>::from(v.as_ref().trim())),
            theme_scope: input
                .theme_scope
                .as_ref()
                .map(|v| Arc::<str>::from(v.as_ref().trim())),
            nonce: input
                .nonce
                .as_ref()
                .map(|v| Arc::<str>::from(v.as_ref().trim())),
        })
    }
}

// ----------------------------------------------------------------------------
// Registry
// ----------------------------------------------------------------------------

pub(crate) struct RegisterResult {
    pub entry: CssInJsEntry,
    pub changed: bool,
    #[cfg(target_arch = "wasm32")]
    pub removed_old_key: Option<String>,
}

#[derive(Debug, Default)]
pub struct CssInJsRegistry {
    entries: HashMap<String, CssInJsEntry>,
    order: Vec<String>,
    identity_index: HashMap<String, String>,
    #[cfg(not(feature = "arcswap"))]
    pub(crate) cached_output: Arc<str>,
    #[cfg(not(feature = "arcswap"))]
    pub(crate) dirty: bool,
}

impl CssInJsRegistry {
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            order: Vec::new(),
            identity_index: HashMap::new(),
            #[cfg(not(feature = "arcswap"))]
            cached_output: Arc::<str>::from(""),
            #[cfg(not(feature = "arcswap"))]
            dirty: false,
        }
    }

    pub(crate) fn register(&mut self, input: CssInJsStyleInput) -> Option<RegisterResult> {
        let entry = CssInJsEntry::from_input(input)?;
        let cache_key = entry.cache_key.clone();
        let identity_key = entry.identity_key.clone();
        let mut preserved_order_index = None;

        #[cfg(target_arch = "wasm32")]
        let mut removed_old_key = None;

        if let Some(existing_key) = self.identity_index.get(identity_key.as_str()).cloned()
            && existing_key != cache_key
        {
            preserved_order_index = self.order.iter().position(|k| k == existing_key.as_str());
            self.entries.remove(existing_key.as_str());
            self.order.retain(|k| k != existing_key.as_str());
            #[cfg(target_arch = "wasm32")]
            {
                removed_old_key = Some(existing_key);
            }
        }

        let changed = !matches!(self.entries.get(cache_key.as_str()), Some(prev) if prev == &entry);

        if changed {
            if !self.entries.contains_key(cache_key.as_str()) {
                if let Some(index) = preserved_order_index {
                    let insert_at = index.min(self.order.len());
                    self.order.insert(insert_at, cache_key.clone());
                } else {
                    self.order.push(cache_key.clone());
                }
            }
            self.entries.insert(cache_key.clone(), entry.clone());
            self.identity_index.insert(identity_key, cache_key);
            self.mark_dirty();
        }

        Some(RegisterResult {
            entry,
            changed,
            #[cfg(target_arch = "wasm32")]
            removed_old_key,
        })
    }

    pub(crate) fn unregister(&mut self, cache_key: &str) -> bool {
        let Some(_) = self.entries.remove(cache_key) else {
            return false;
        };
        self.order.retain(|k| k != cache_key);
        self.identity_index.retain(|_, key| key != cache_key);
        self.mark_dirty();
        true
    }

    pub(crate) fn clear(&mut self) -> Vec<String> {
        if self.entries.is_empty() {
            return Vec::new();
        }
        let keys = self.order.clone();
        self.entries.clear();
        self.order.clear();
        self.identity_index.clear();
        self.reset_cache_state();
        keys
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
    pub(crate) fn compose_css(&self) -> Arc<str> {
        super::CSSINJS_COMPOSE_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut out = String::new();
        for (_, _, entry) in self.sorted_entries() {
            if let Some(entry) = self.entries.get(entry.cache_key.as_str()) {
                out.push_str(entry.rendered_css.as_ref());
                if !out.ends_with('\n') {
                    out.push('\n');
                }
            }
        }
        Arc::<str>::from(out)
    }

    #[inline]
    #[must_use]
    fn sorted_entries(&self) -> Vec<(usize, &String, &CssInJsEntry)> {
        let mut ordered = self
            .order
            .iter()
            .enumerate()
            .filter_map(|(idx, key)| {
                self.entries
                    .get(key.as_str())
                    .map(|entry| (idx, key, entry))
            })
            .collect::<Vec<_>>();

        ordered.sort_by(|(idx_a, key_a, entry_a), (idx_b, key_b, entry_b)| {
            entry_a
                .order
                .cmp(&entry_b.order)
                .then_with(|| idx_a.cmp(idx_b))
                .then_with(|| key_a.cmp(key_b))
        });
        ordered
    }

    #[cfg(not(feature = "arcswap"))]
    #[inline]
    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    #[cfg(feature = "arcswap")]
    #[inline]
    fn mark_dirty(&mut self) {}

    #[cfg(not(feature = "arcswap"))]
    #[inline]
    fn reset_cache_state(&mut self) {
        self.cached_output = Arc::<str>::from("");
        self.dirty = false;
    }

    #[cfg(feature = "arcswap")]
    #[inline]
    fn reset_cache_state(&mut self) {}

    #[cfg(not(feature = "arcswap"))]
    #[inline]
    #[must_use]
    pub fn css_arc(&mut self) -> Arc<str> {
        if !self.dirty {
            return self.cached_output.clone();
        }

        self.cached_output = self.compose_css();
        self.dirty = false;
        self.cached_output.clone()
    }

    #[inline]
    #[must_use]
    pub fn records(&self) -> Vec<CssInJsStyleRecord> {
        self.sorted_entries()
            .into_iter()
            .map(|(_, _, entry)| {
                let tier = entry
                    .identity_scope
                    .as_deref()
                    .map(str::to_lowercase)
                    .filter(|v| v.contains("|effect|") || v.ends_with("|effect"))
                    .map(|_| "effect".to_string())
                    .unwrap_or_else(|| "main".to_string());

                CssInJsStyleRecord {
                    style_id: entry.style_id.to_string(),
                    path: entry.identity_scope.as_deref().map(ToString::to_string),
                    scope: entry.theme_scope.as_deref().map(ToString::to_string),
                    order: entry.order,
                    layer: entry.layer.as_deref().map(ToString::to_string),
                    tier,
                    hash: entry.hash_class.to_string(),
                    cache_key: entry.cache_key.clone(),
                    rewrite_signature: entry.rewrite_signature.to_string(),
                    hashed: entry.hashed,
                    css_var_key: entry.css_var_key.as_deref().map(ToString::to_string),
                    algorithm: entry.algorithm.as_deref().map(ToString::to_string),
                    theme_scope: entry.theme_scope.as_deref().map(ToString::to_string),
                    token_hash: entry.token_hash.as_deref().map(ToString::to_string),
                    nonce: entry.nonce.as_deref().map(ToString::to_string),
                }
            })
            .collect()
    }

    #[inline]
    #[must_use]
    pub fn css_entries(&self) -> Vec<CssInJsCssEntry> {
        self.sorted_entries()
            .into_iter()
            .map(|(_, _, entry)| CssInJsCssEntry {
                cache_key: entry.cache_key.clone(),
                style_id: entry.style_id.to_string(),
                path: entry.identity_scope.as_deref().map(ToString::to_string),
                rendered_css: entry.rendered_css.clone(),
            })
            .collect()
    }
}

// ----------------------------------------------------------------------------
// Style Record (for serialization)
// ----------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CssInJsStyleRecord {
    pub style_id: String,
    pub path: Option<String>,
    pub scope: Option<String>,
    pub order: i32,
    pub layer: Option<String>,
    pub tier: String,
    pub hash: String,
    pub cache_key: String,
    pub rewrite_signature: String,
    pub hashed: Option<bool>,
    pub css_var_key: Option<String>,
    pub algorithm: Option<String>,
    pub theme_scope: Option<String>,
    pub token_hash: Option<String>,
    pub nonce: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CssInJsCssEntry {
    pub cache_key: String,
    pub style_id: String,
    pub path: Option<String>,
    pub rendered_css: Arc<str>,
}
