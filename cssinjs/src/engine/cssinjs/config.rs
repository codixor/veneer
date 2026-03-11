//! Configuration types for the CSS‑in‑JS runtime.

use std::sync::Arc;

// ----------------------------------------------------------------------------
// Re‑exports from submodules
// ----------------------------------------------------------------------------

pub use crate::style_provider::HashPriority;

// ----------------------------------------------------------------------------
// Config store (internal)
// ----------------------------------------------------------------------------

pub(crate) mod store {
    use super::CssInJsConfig;
    use std::sync::{OnceLock, RwLock};

    static CSSINJS_CONFIG: OnceLock<RwLock<CssInJsConfig>> = OnceLock::new();

    #[inline]
    fn lock() -> &'static RwLock<CssInJsConfig> {
        CSSINJS_CONFIG.get_or_init(|| RwLock::new(CssInJsConfig::default()))
    }

    #[inline]
    pub(crate) fn get() -> CssInJsConfig {
        match lock().read() {
            Ok(cfg) => cfg.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    #[inline]
    pub(crate) fn set(next: CssInJsConfig) -> bool {
        match lock().write() {
            Ok(mut cfg) => {
                if *cfg == next {
                    false
                } else {
                    *cfg = next;
                    true
                }
            }
            Err(poisoned) => {
                let mut cfg = poisoned.into_inner();
                if *cfg == next {
                    false
                } else {
                    *cfg = next;
                    true
                }
            }
        }
    }

    #[inline]
    pub(crate) fn with<R>(f: impl FnOnce(&CssInJsConfig) -> R) -> R {
        match lock().read() {
            Ok(cfg) => f(&cfg),
            Err(poisoned) => f(&poisoned.into_inner()),
        }
    }
}

// ----------------------------------------------------------------------------
// Meta attributes (optional)
// ----------------------------------------------------------------------------

/// Optional mapping from semantic values -> DOM attributes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CssInJsMetaAttrs {
    pub enabled_attr: Option<String>,
    pub style_id_attr: Option<String>,
    pub token_hash_attr: Option<String>,
    pub hashed_attr: Option<String>,
    pub hash_class_attr: Option<String>,
    pub css_var_key_attr: Option<String>,
    pub algorithm_attr: Option<String>,
    pub theme_scope_attr: Option<String>,
}

// ----------------------------------------------------------------------------
// Rewrite rules
// ----------------------------------------------------------------------------

/// Opt‑in rewrite rules.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CssRewriteCfg {
    pub class_prefix_pairs: Vec<(Arc<str>, Arc<str>)>,
    pub css_var_prefix_pairs: Vec<(Arc<str>, Arc<str>)>,
}

// ----------------------------------------------------------------------------
// Vendor compatibility
// ----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VendorCompatCfg {
    pub normalize_vendor_properties: bool,
    pub emit_standard_from_prefixed: bool,
    pub emit_prefixed_from_standard: bool,
}

impl Default for VendorCompatCfg {
    fn default() -> Self {
        Self {
            normalize_vendor_properties: true,
            emit_standard_from_prefixed: true,
            emit_prefixed_from_standard: true,
        }
    }
}

// ----------------------------------------------------------------------------
// Scoping traits
// ----------------------------------------------------------------------------

pub trait CssScoper: Send + Sync + 'static {
    fn scope(&self, css: &str, scope_prefix: &str) -> String;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CssScoperEngineKind {
    #[default]
    Rule,
    LightningCss,
    Scss,
    Acss,
}

#[derive(Clone, Debug, Default)]
pub struct EngineCssScoper {
    kind: CssScoperEngineKind,
}

impl EngineCssScoper {
    #[inline]
    #[must_use]
    pub const fn new(kind: CssScoperEngineKind) -> Self {
        Self { kind }
    }

    #[inline]
    #[must_use]
    pub const fn kind(&self) -> CssScoperEngineKind {
        self.kind
    }
}

impl CssScoper for EngineCssScoper {
    #[inline]
    fn scope(&self, css: &str, scope_prefix: &str) -> String {
        match self.kind {
            CssScoperEngineKind::Rule => {
                super::transform::CssTransform::scope_each_rule(css, scope_prefix)
            }
            CssScoperEngineKind::LightningCss => {
                super::transform::lightning_scope(css, scope_prefix, true).unwrap_or_else(|| {
                    super::transform::CssTransform::scope_each_rule(css, scope_prefix)
                })
            }
            CssScoperEngineKind::Scss | CssScoperEngineKind::Acss => {
                super::transform::CssTransform::scope_each_rule(css, scope_prefix)
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct DefaultCssScoper;
impl CssScoper for DefaultCssScoper {
    fn scope(&self, css: &str, scope_prefix: &str) -> String {
        super::transform::CssTransform::scope_each_rule(css, scope_prefix)
    }
}

// ----------------------------------------------------------------------------
// Main config
// ----------------------------------------------------------------------------

#[derive(Clone)]
pub struct CssInJsConfig {
    pub normalize_explicit_custom_props: bool,
    pub vendor_compat: VendorCompatCfg,

    pub hash_version_tag: Option<String>,
    pub unique_hash_prefix: Option<String>,
    pub unique_hash_len: Option<usize>,
    pub cache_key_prefix: Option<String>,
    pub cache_key_len: Option<usize>,
    pub class_prefix: Option<String>,
    pub class_hash_len: Option<usize>,

    pub style_node_id_prefix: Option<String>,
    pub style_node_id_attr: Option<String>,
    pub style_node_owner_key: Option<String>,
    pub runtime_dom_injection: bool,

    pub compact_sync: bool,
    pub emit_node_attrs: bool,
    pub nonce_attr: Option<String>,
    pub extra_attrs: Vec<(String, String)>,

    pub meta_attrs: CssInJsMetaAttrs,
    pub rewrite: CssRewriteCfg,
    pub scoper: Option<Arc<dyn CssScoper>>,
}

impl Default for CssInJsConfig {
    fn default() -> Self {
        Self {
            normalize_explicit_custom_props: false,
            vendor_compat: VendorCompatCfg::default(),
            hash_version_tag: None,
            unique_hash_prefix: None,
            unique_hash_len: None,
            cache_key_prefix: None,
            cache_key_len: None,
            class_prefix: None,
            class_hash_len: None,
            style_node_id_prefix: None,
            style_node_id_attr: None,
            style_node_owner_key: None,
            runtime_dom_injection: true,
            compact_sync: false,
            emit_node_attrs: false,
            nonce_attr: None,
            extra_attrs: Vec::new(),
            meta_attrs: CssInJsMetaAttrs::default(),
            rewrite: CssRewriteCfg::default(),
            scoper: None,
        }
    }
}

impl std::fmt::Debug for CssInJsConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CssInJsConfig")
            .field(
                "normalize_explicit_custom_props",
                &self.normalize_explicit_custom_props,
            )
            .field("vendor_compat", &self.vendor_compat)
            .field("hash_version_tag", &self.hash_version_tag)
            .field("unique_hash_prefix", &self.unique_hash_prefix)
            .field("unique_hash_len", &self.unique_hash_len)
            .field("cache_key_prefix", &self.cache_key_prefix)
            .field("cache_key_len", &self.cache_key_len)
            .field("class_prefix", &self.class_prefix)
            .field("class_hash_len", &self.class_hash_len)
            .field("style_node_id_prefix", &self.style_node_id_prefix)
            .field("style_node_id_attr", &self.style_node_id_attr)
            .field("style_node_owner_key", &self.style_node_owner_key)
            .field("runtime_dom_injection", &self.runtime_dom_injection)
            .field("compact_sync", &self.compact_sync)
            .field("emit_node_attrs", &self.emit_node_attrs)
            .field("nonce_attr", &self.nonce_attr)
            .field("extra_attrs", &self.extra_attrs)
            .field("meta_attrs", &self.meta_attrs)
            .field("rewrite", &self.rewrite)
            .field("has_scoper", &self.scoper.is_some())
            .finish()
    }
}

impl PartialEq for CssInJsConfig {
    fn eq(&self, other: &Self) -> bool {
        self.normalize_explicit_custom_props == other.normalize_explicit_custom_props
            && self.vendor_compat == other.vendor_compat
            && self.hash_version_tag == other.hash_version_tag
            && self.unique_hash_prefix == other.unique_hash_prefix
            && self.unique_hash_len == other.unique_hash_len
            && self.cache_key_prefix == other.cache_key_prefix
            && self.cache_key_len == other.cache_key_len
            && self.class_prefix == other.class_prefix
            && self.class_hash_len == other.class_hash_len
            && self.style_node_id_prefix == other.style_node_id_prefix
            && self.style_node_id_attr == other.style_node_id_attr
            && self.style_node_owner_key == other.style_node_owner_key
            && self.runtime_dom_injection == other.runtime_dom_injection
            && self.compact_sync == other.compact_sync
            && self.emit_node_attrs == other.emit_node_attrs
            && self.nonce_attr == other.nonce_attr
            && self.extra_attrs == other.extra_attrs
            && self.meta_attrs == other.meta_attrs
            && self.rewrite == other.rewrite
            && match (&self.scoper, &other.scoper) {
                (None, None) => true,
                (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                _ => false,
            }
    }
}

impl Eq for CssInJsConfig {}

impl CssInJsConfig {
    #[inline]
    pub fn set_scoper_engine(&mut self, engine: CssScoperEngineKind) {
        self.scoper = Some(Arc::new(EngineCssScoper::new(engine)));
    }

    #[inline]
    #[must_use]
    pub fn with_scoper_engine(mut self, engine: CssScoperEngineKind) -> Self {
        self.set_scoper_engine(engine);
        self
    }
}
