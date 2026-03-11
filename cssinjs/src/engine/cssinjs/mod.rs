//! Optimised CSS‑in‑JS runtime with a powerful, flexible API.
//!
//! This module provides a complete runtime for dynamic CSS generation,
//! including registration, scoping, hashing, and (on WASM) DOM injection.

#![forbid(unsafe_code)]

mod config;
mod hash;
mod object;
mod registry;
mod transform;
mod util;
mod var;
#[cfg(target_arch = "wasm32")]
mod wasm;

pub use config::*;
pub use object::*;
pub use registry::*;
pub use var::*;

#[cfg(feature = "arcswap")]
use arcswap::ArcSwapAny;
#[cfg(feature = "arcswap")]
use std::sync::atomic::AtomicBool;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock, RwLock};

// ----------------------------------------------------------------------------
// Global state
// ----------------------------------------------------------------------------

static CSSINJS_REGISTRY: OnceLock<RwLock<CssInJsRegistry>> = OnceLock::new();
#[cfg(feature = "arcswap")]
static CSSINJS_CACHED_OUTPUT: OnceLock<ArcSwapAny<Arc<CachedCss>>> = OnceLock::new();
#[cfg(feature = "arcswap")]
static CSSINJS_CACHE_DIRTY: AtomicBool = AtomicBool::new(true);
static CSSINJS_COMPOSE_COUNT: AtomicUsize = AtomicUsize::new(0);
static CSSINJS_STYLE_REVISION: AtomicU64 = AtomicU64::new(1);
type CssInJsRevisionListener = Arc<dyn Fn(u64) + Send + Sync + 'static>;
static CSSINJS_REVISION_LISTENERS: OnceLock<RwLock<Vec<(u64, CssInJsRevisionListener)>>> =
    OnceLock::new();
static CSSINJS_REVISION_LISTENER_ID: AtomicU64 = AtomicU64::new(1);

#[cfg(feature = "arcswap")]
#[derive(Debug)]
struct CachedCss(Arc<str>);

// ----------------------------------------------------------------------------
// Shared types
// ----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CssInJsLifecycleCfg {
    pub auto_clear: bool,
}
impl Default for CssInJsLifecycleCfg {
    fn default() -> Self {
        Self { auto_clear: true }
    }
}

#[derive(Debug)]
pub struct CssInJsLease {
    cache_keys: Vec<String>,
    active: bool,
}
impl CssInJsLease {
    #[inline]
    #[must_use]
    pub fn new(cache_keys: Vec<String>) -> Self {
        Self {
            cache_keys,
            active: true,
        }
    }

    #[inline]
    #[must_use]
    pub fn cache_keys(&self) -> &[String] {
        &self.cache_keys
    }

    #[inline]
    pub fn clear_now(&mut self) {
        if !self.active {
            return;
        }
        for key in self.cache_keys.clone() {
            let _ = CssInJs::unregister(key.as_str());
        }
        self.active = false;
    }

    #[inline]
    pub fn disarm(&mut self) {
        self.active = false;
    }
}
impl Drop for CssInJsLease {
    fn drop(&mut self) {
        self.clear_now();
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CssInJsStyleInput {
    pub style_id: Arc<str>,
    pub css: Arc<str>,
    pub identity_scope: Option<Arc<str>>,
    pub order: i32,

    pub token_hash: Option<Arc<str>>,
    pub hashed: Option<bool>,
    pub css_var_key: Option<Arc<str>>,
    pub algorithm: Option<Arc<str>>,
    pub theme_scope: Option<Arc<str>>,

    pub nonce: Option<Arc<str>>,
    pub layer: Option<Arc<str>>,

    pub hash_class: Option<Arc<str>>,
    pub hash_priority: HashPriority,

    /// Optional per-style rewrite override. If None, uses global `CssInJsConfig::rewrite`.
    pub rewrite: Option<Arc<CssRewriteCfg>>,
}
impl Default for CssInJsStyleInput {
    fn default() -> Self {
        Self {
            style_id: Arc::from(""),
            css: Arc::from(""),
            identity_scope: None,
            order: 0,

            token_hash: None,
            hashed: None,
            css_var_key: None,
            algorithm: None,
            theme_scope: None,

            nonce: None,
            layer: None,

            hash_class: None,
            hash_priority: HashPriority::Low,

            rewrite: None,
        }
    }
}
impl CssInJsStyleInput {
    #[inline]
    #[must_use]
    pub fn new(style_id: impl Into<Arc<str>>, css: impl Into<Arc<str>>) -> Self {
        Self {
            style_id: style_id.into(),
            css: css.into(),
            ..Self::default()
        }
    }
}

// ----------------------------------------------------------------------------
// Public facade
// ----------------------------------------------------------------------------

pub struct CssInJs;

impl CssInJs {
    #[inline]
    fn bump_revision() {
        let revision = CSSINJS_STYLE_REVISION
            .fetch_add(1, Ordering::Relaxed)
            .saturating_add(1);
        Self::notify_revision_listeners(revision);
    }

    #[inline]
    fn revision_listeners_lock() -> &'static RwLock<Vec<(u64, CssInJsRevisionListener)>> {
        CSSINJS_REVISION_LISTENERS.get_or_init(|| RwLock::new(Vec::new()))
    }

    fn notify_revision_listeners(revision: u64) {
        let listeners = match Self::revision_listeners_lock().read() {
            Ok(listeners) => listeners
                .iter()
                .map(|(_, listener)| Arc::clone(listener))
                .collect::<Vec<_>>(),
            Err(poisoned) => {
                let listeners = poisoned.into_inner();
                listeners
                    .iter()
                    .map(|(_, listener)| Arc::clone(listener))
                    .collect::<Vec<_>>()
            }
        };
        for listener in listeners {
            listener(revision);
        }
    }

    // --- Config access ------------------------------------------------------
    #[inline]
    fn registry_lock() -> &'static RwLock<CssInJsRegistry> {
        CSSINJS_REGISTRY.get_or_init(|| RwLock::new(CssInJsRegistry::new()))
    }

    #[cfg(feature = "arcswap")]
    #[inline]
    fn cached_output_store() -> &'static ArcSwapAny<Arc<CachedCss>> {
        CSSINJS_CACHED_OUTPUT
            .get_or_init(|| ArcSwapAny::new(Arc::new(CachedCss(Arc::<str>::from("")))))
    }

    #[inline]
    #[must_use]
    pub fn config() -> CssInJsConfig {
        config::store::get()
    }

    #[inline]
    pub fn set_config(next: CssInJsConfig) -> bool {
        config::store::set(next)
    }

    // --- Registry stats -----------------------------------------------------
    #[inline]
    #[must_use]
    pub fn len() -> usize {
        match Self::registry_lock().read() {
            Ok(g) => g.len(),
            Err(poisoned) => poisoned.into_inner().len(),
        }
    }

    #[inline]
    #[must_use]
    pub fn is_empty() -> bool {
        Self::len() == 0
    }

    /// Monotonic style revision counter.
    ///
    /// This only changes when registry content mutates (register/unregister/clear).
    /// Useful for cheap "did style output potentially change?" checks in runtimes.
    #[inline]
    #[must_use]
    pub fn revision() -> u64 {
        CSSINJS_STYLE_REVISION.load(Ordering::Relaxed)
    }

    /// Subscribe to style revision updates.
    ///
    /// The returned id can be passed to [`Self::unsubscribe_revision_listener`].
    #[inline]
    pub fn subscribe_revision_listener(listener: CssInJsRevisionListener) -> u64 {
        let id = CSSINJS_REVISION_LISTENER_ID.fetch_add(1, Ordering::Relaxed);
        match Self::revision_listeners_lock().write() {
            Ok(mut listeners) => listeners.push((id, listener)),
            Err(poisoned) => {
                let mut listeners = poisoned.into_inner();
                listeners.push((id, listener));
            }
        }
        id
    }

    /// Remove a previously subscribed style revision listener.
    #[inline]
    pub fn unsubscribe_revision_listener(listener_id: u64) -> bool {
        let remove_from = |listeners: &mut Vec<(u64, CssInJsRevisionListener)>| -> bool {
            if let Some(index) = listeners.iter().position(|(id, _)| *id == listener_id) {
                listeners.swap_remove(index);
                true
            } else {
                false
            }
        };

        match Self::revision_listeners_lock().write() {
            Ok(mut listeners) => remove_from(&mut listeners),
            Err(poisoned) => {
                let mut listeners = poisoned.into_inner();
                remove_from(&mut listeners)
            }
        }
    }

    // --- CSS composition ----------------------------------------------------
    #[inline]
    #[must_use]
    pub fn css_arc() -> Arc<str> {
        #[cfg(feature = "arcswap")]
        {
            if !CSSINJS_CACHE_DIRTY.load(Ordering::Acquire) {
                let cached = Self::cached_output_store().load_full();
                return Arc::clone(&cached.0);
            }

            let css = match Self::registry_lock().write() {
                Ok(g) => g.compose_css(),
                Err(poisoned) => poisoned.into_inner().compose_css(),
            };

            Self::cached_output_store().store(Arc::new(CachedCss(css.clone())));
            CSSINJS_CACHE_DIRTY.store(false, Ordering::Release);
            css
        }

        #[cfg(not(feature = "arcswap"))]
        if let Ok(g) = Self::registry_lock().read()
            && !g.dirty
        {
            return g.cached_output.clone();
        }

        #[cfg(not(feature = "arcswap"))]
        match Self::registry_lock().write() {
            Ok(mut g) => g.css_arc(),
            Err(poisoned) => poisoned.into_inner().css_arc(),
        }
    }

    // --- Records ------------------------------------------------------------
    #[inline]
    #[must_use]
    pub fn records() -> Vec<CssInJsStyleRecord> {
        match Self::registry_lock().read() {
            Ok(g) => g.records(),
            Err(poisoned) => poisoned.into_inner().records(),
        }
    }

    #[inline]
    #[must_use]
    pub fn css_entries() -> Vec<CssInJsCssEntry> {
        match Self::registry_lock().read() {
            Ok(g) => g.css_entries(),
            Err(poisoned) => poisoned.into_inner().css_entries(),
        }
    }

    // --- Rewrite helper -----------------------------------------------------
    #[inline]
    #[must_use]
    pub(crate) fn rewrite_for_runtime(raw_css: &str) -> (String, Option<String>, bool) {
        let cfg = config::store::get();
        let rw = &cfg.rewrite;

        let has_rewrite = rw.class_prefix_pairs.iter().any(|(from, to)| {
            let from = from.as_ref().trim();
            let to = to.as_ref().trim();
            !from.is_empty() && !to.is_empty() && from != to
        }) || rw.css_var_prefix_pairs.iter().any(|(from, to)| {
            let from = from.as_ref().trim();
            let to = to.as_ref().trim();
            !from.is_empty() && !to.is_empty() && from != to
        });

        if !has_rewrite {
            return (raw_css.to_string(), None, false);
        }

        let rewritten = transform::CssTransform::apply_rewrite_rules(raw_css, rw);
        let fp = hash::rewrite_fingerprint(rw, &cfg);
        (rewritten, Some(format!("rw={fp:016x}")), true)
    }

    // --- File I/O (non‑WASM) ------------------------------------------------
    #[cfg(not(target_arch = "wasm32"))]
    pub fn write_file(path: impl AsRef<std::path::Path>) -> std::io::Result<usize> {
        let css = Self::css_arc();
        std::fs::write(path, css.as_ref())?;
        Ok(css.len())
    }

    // --- Node ID helpers ----------------------------------------------------
    #[inline]
    #[must_use]
    pub fn node_id(cache_key: &str) -> Option<String> {
        config::store::with(|cfg| {
            cfg.style_node_id_prefix
                .as_deref()
                .map(str::trim)
                .filter(|p| !p.is_empty())
                .map(|p| format!("{p}{cache_key}"))
        })
    }

    #[inline]
    #[must_use]
    pub fn owner_key() -> Option<String> {
        config::store::with(|cfg| {
            cfg.style_node_owner_key
                .clone()
                .filter(|s| !s.trim().is_empty())
        })
    }

    // --- Registration -------------------------------------------------------
    pub fn register(input: CssInJsStyleInput) -> Option<CssInJsRegistration> {
        let result = match Self::registry_lock().write() {
            Ok(mut g) => g.register(input),
            Err(poisoned) => poisoned.into_inner().register(input),
        }?;

        if result.changed {
            Self::bump_revision();
        }

        #[cfg(feature = "arcswap")]
        if result.changed {
            CSSINJS_CACHE_DIRTY.store(true, Ordering::Release);
        }

        #[cfg(target_arch = "wasm32")]
        if config::store::with(|cfg| cfg.runtime_dom_injection) {
            if config::store::with(|cfg| cfg.compact_sync) {
                wasm::CssDomInjector::schedule_sync();
            } else {
                let _ = wasm::CssDomInjector::sync_register_result(&result);
            }
        }

        Some(CssInJsRegistration {
            cache_key: result.entry.cache_key.clone(),
            hash_class: result.entry.hash_class.to_string(),
        })
    }

    pub fn register_with_lifecycle(
        input: CssInJsStyleInput,
        lifecycle: Option<CssInJsLifecycleCfg>,
    ) -> Option<(CssInJsRegistration, Option<CssInJsLease>)> {
        let reg = Self::register(input)?;
        let lease = if lifecycle.unwrap_or_default().auto_clear {
            Some(CssInJsLease::new(vec![reg.cache_key.clone()]))
        } else {
            None
        };
        Some((reg, lease))
    }

    pub fn unregister(cache_key: &str) -> bool {
        let removed = match Self::registry_lock().write() {
            Ok(mut g) => g.unregister(cache_key),
            Err(poisoned) => poisoned.into_inner().unregister(cache_key),
        };

        if removed {
            Self::bump_revision();
        }

        #[cfg(feature = "arcswap")]
        if removed {
            CSSINJS_CACHE_DIRTY.store(true, Ordering::Release);
        }

        #[cfg(target_arch = "wasm32")]
        if removed && config::store::with(|cfg| cfg.runtime_dom_injection) {
            wasm::CssDomInjector::remove_node(cache_key);
        }

        removed
    }

    pub fn clear() {
        #[allow(unused_variables)]
        let removed_keys = match Self::registry_lock().write() {
            Ok(mut g) => g.clear(),
            Err(poisoned) => poisoned.into_inner().clear(),
        };

        if !removed_keys.is_empty() {
            Self::bump_revision();
        }

        #[cfg(feature = "arcswap")]
        if !removed_keys.is_empty() {
            Self::cached_output_store().store(Arc::new(CachedCss(Arc::<str>::from(""))));
            CSSINJS_CACHE_DIRTY.store(false, Ordering::Release);
        }

        #[cfg(target_arch = "wasm32")]
        if config::store::with(|cfg| cfg.runtime_dom_injection) {
            for key in removed_keys {
                wasm::CssDomInjector::remove_node(key.as_str());
            }
        }
    }

    // --- Diagnostics --------------------------------------------------------
    #[doc(hidden)]
    #[inline]
    #[must_use]
    pub fn debug_compose_count() -> usize {
        CSSINJS_COMPOSE_COUNT.load(Ordering::Relaxed)
    }

    #[doc(hidden)]
    #[inline]
    #[must_use]
    pub fn debug_cache_dirty() -> bool {
        #[cfg(feature = "arcswap")]
        {
            CSSINJS_CACHE_DIRTY.load(Ordering::Acquire)
        }

        #[cfg(not(feature = "arcswap"))]
        {
            match Self::registry_lock().read() {
                Ok(g) => g.dirty,
                Err(poisoned) => poisoned.into_inner().dirty,
            }
        }
    }

    #[doc(hidden)]
    pub fn debug_reset_runtime_for_tests() {
        Self::clear();
        let _ = Self::set_config(CssInJsConfig::default());
        CSSINJS_COMPOSE_COUNT.store(0, Ordering::Relaxed);
        CSSINJS_STYLE_REVISION.store(1, Ordering::Relaxed);

        #[cfg(feature = "arcswap")]
        {
            Self::cached_output_store().store(Arc::new(CachedCss(Arc::<str>::from(""))));
            CSSINJS_CACHE_DIRTY.store(false, Ordering::Release);
        }
    }

    // --- Object styles ------------------------------------------------------
    pub fn register_style_object_with_path(
        info: &StyleRegisterInput,
        parse_cfg: &CssParseCfg,
        interpolation: CssInterpolation,
        lifecycle: Option<CssInJsLifecycleCfg>,
    ) -> StyleObjectRegistration {
        object::CssParser::register_style_object_with_path(
            info,
            parse_cfg,
            interpolation,
            lifecycle,
        )
    }

    pub fn parse_style_object_css(
        parse_cfg: &CssParseCfg,
        interpolation: CssInterpolation,
    ) -> String {
        object::CssParser::parse_style(interpolation, parse_cfg, None).parsed_css
    }

    pub fn register_style_with_path(
        info: &StyleRegisterInput,
        style_css: impl AsRef<str>,
    ) -> Option<CssInJsRegistration> {
        object::CssParser::register_style_with_path(info, style_css)
    }

    // --- CSS vars -----------------------------------------------------------
    #[inline]
    #[must_use]
    pub fn unit(value: impl Into<CssVarTokenValue>) -> String {
        var::CssVarEngine::unit(value)
    }

    pub fn register_css_vars(
        config: &CssVarRegisterInput,
        token: &CssVarTokenMap,
    ) -> CssVarRegisterOutput {
        var::CssVarEngine::register_css_vars(config, token)
    }
}
