//! style_provider.rs — Grade-10 enterprise generic StyleProvider for Dioxus 0.7.3 (Rust 2024 / 1.9.3).
//!
//! Changes requested:
//! - Fully generic defaults: **no "ant"/"anticon" defaults**.
//! - prefix/icon/css-var prefixes are optional (`Option<Arc<str>>`).
//! - Rewriter only runs when the user explicitly sets them (`Some(...)`).
//! - No orphan functions: all logic in impl blocks.

use dioxus::prelude::*;
use std::sync::Arc;

use crate::{
    CssInJs, StyleEntry, StyleTier, flush_head, head_style_config, inject_styles,
    register_or_update, set_head_style_config,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HashPriority {
    Low,
    High,
}

/// Provider config.
/// Everything is optional; the provider only rewrites/annotates when options are set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StyleConfig {
    /// Optional component class prefix. If set, rewrites `.default_component_prefix-` to `.{prefix}-`.
    pub prefix_cls: Option<Arc<str>>,
    /// Optional icon class prefix. If set, rewrites `.default_icon_prefix-` to `.{icon_prefix}-`.
    pub icon_prefix_cls: Option<Arc<str>>,
    /// Optional CSS variable prefix. If set, rewrites `--default_css_var_prefix-` to `--{css_var_prefix}-`.
    pub css_var_prefix: Option<Arc<str>>,

    /// These "defaults to rewrite from" are also optional.
    /// If you want automatic rewriting from a known upstream (like "ant"), set them.
    pub rewrite_from_prefix_cls: Option<Arc<str>>,
    pub rewrite_from_icon_prefix_cls: Option<Arc<str>>,
    pub rewrite_from_css_var_prefix: Option<Arc<str>>,

    pub hash_priority: HashPriority,
    pub layer: Option<Arc<str>>,
    pub nonce: Option<Arc<str>>,
    pub theme_seed: u64,
}

impl Default for StyleConfig {
    fn default() -> Self {
        Self {
            prefix_cls: None,
            icon_prefix_cls: None,
            css_var_prefix: None,

            // No implicit rewrite sources either (fully generic).
            rewrite_from_prefix_cls: None,
            rewrite_from_icon_prefix_cls: None,
            rewrite_from_css_var_prefix: None,

            hash_priority: HashPriority::Low,
            layer: None,
            nonce: None,
            theme_seed: 0,
        }
    }
}

/// Swap scoping implementation without changing the StyleProvider API.
pub trait CssScoper: Send + Sync + 'static {
    fn transform(&self, raw_css: &str, cfg: &StyleConfig, scope_class: &str) -> String;
}

/// Default scoper:
/// - optional prefix rewrite (only when both `rewrite_from_*` and target `*_prefix` are set)
/// - scopes selectors using :where(.scope) or .scope (string scoping)
/// - optional @layer wrapping
#[derive(Clone, Default)]
pub struct DefaultCssScoper;

impl CssScoper for DefaultCssScoper {
    fn transform(&self, raw_css: &str, cfg: &StyleConfig, scope_class: &str) -> String {
        let rewritten = CssTransform::rewrite_prefixed_css(raw_css, cfg);

        let scope_prefix = match cfg.hash_priority {
            HashPriority::Low => format!(":where(.{scope_class})"),
            HashPriority::High => format!(".{scope_class}"),
        };

        let scoped = CssTransform::scope_each_rule(&rewritten, &scope_prefix);

        if let Some(layer) = cfg
            .layer
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            format!("@layer {layer} {{\n{scoped}\n}}\n")
        } else {
            scoped
        }
    }
}

#[derive(Clone)]
pub struct StyleHandle {
    cfg: Signal<StyleConfig>,
    scoper: Arc<dyn CssScoper>,
}

#[derive(Props, Clone)]
pub struct StyleProviderProps {
    #[props(optional)]
    pub config: Option<StyleConfig>,

    /// Optional custom scoper. Defaults to `DefaultCssScoper`.
    #[props(optional)]
    pub scoper: Option<Arc<dyn CssScoper>>,

    pub children: Element,
}

impl PartialEq for StyleProviderProps {
    fn eq(&self, other: &Self) -> bool {
        if self.config != other.config {
            return false;
        }
        if self.children != other.children {
            return false;
        }
        match (&self.scoper, &other.scoper) {
            (None, None) => true,
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

#[component]
pub fn StyleProvider(props: StyleProviderProps) -> Element {
    let cfg: Signal<StyleConfig> = use_signal(|| props.config.clone().unwrap_or_default());

    let scoper: Arc<dyn CssScoper> = use_hook(move || {
        props
            .scoper
            .clone()
            .unwrap_or_else(|| Arc::new(DefaultCssScoper))
    });

    // Keep cfg signal updated if props change
    {
        let next_cfg = props.config.clone().unwrap_or_default();
        let mut cfg_sig = cfg;
        use_effect(use_reactive!(|(next_cfg,)| {
            if *cfg_sig.peek() != next_cfg {
                cfg_sig.set(next_cfg.clone());
            }
        }));
    }

    // Sync nonce into runtime injector (generic: only nonce)
    {
        let cfg_snapshot = cfg.read().clone();
        use_effect(use_reactive!(|(cfg_snapshot,)| {
            let mut head_cfg = head_style_config();
            head_cfg.nonce = cfg_snapshot.nonce.as_ref().map(|v| v.to_string());
            let _ = set_head_style_config(head_cfg);
        }));
    }

    let handle = StyleHandle { cfg, scoper };
    let _ = use_context_provider(|| handle.clone());

    rsx! {
        {props.children}
    }
}

#[must_use]
pub fn use_style() -> StyleHandle {
    consume_context::<StyleHandle>()
}

impl StyleHandle {
    /// Construct a `StyleHandle` from its constituent parts.
    ///
    /// Used by `dioxus-config`'s style bridge to create a handle without
    /// going through the `StyleProvider` component.
    pub fn from_parts(cfg: Signal<StyleConfig>, scoper: Arc<dyn CssScoper>) -> Self {
        Self { cfg, scoper }
    }

    #[inline]
    #[must_use]
    pub fn config(&self) -> StyleConfig {
        self.cfg.read().clone()
    }

    #[inline]
    pub fn set_config(&self, next: StyleConfig) {
        let mut cfg_sig = self.cfg;
        cfg_sig.set(next);
    }

    /// Stable scope class, deterministic across platforms and runs.
    #[inline]
    #[must_use]
    pub fn scope_class(&self) -> String {
        let c = self.cfg.read();

        // Hash only what matters. None is stable by hashing empty markers.
        let h = StableHash::fnv1a64(&[
            b"p:",
            c.prefix_cls.as_deref().unwrap_or("").as_bytes(),
            b"|ip:",
            c.icon_prefix_cls.as_deref().unwrap_or("").as_bytes(),
            b"|vp:",
            c.css_var_prefix.as_deref().unwrap_or("").as_bytes(),
            b"|fp:",
            c.rewrite_from_prefix_cls
                .as_deref()
                .unwrap_or("")
                .as_bytes(),
            b"|fip:",
            c.rewrite_from_icon_prefix_cls
                .as_deref()
                .unwrap_or("")
                .as_bytes(),
            b"|fvp:",
            c.rewrite_from_css_var_prefix
                .as_deref()
                .unwrap_or("")
                .as_bytes(),
            b"|seed:",
            &c.theme_seed.to_le_bytes(),
            b"|hp:",
            match c.hash_priority {
                HashPriority::Low => b"low",
                HashPriority::High => b"high",
            },
            b"|layer:",
            c.layer.as_deref().unwrap_or("").as_bytes(),
        ]);

        format!("css-{h:016x}")
    }

    #[inline]
    #[must_use]
    pub fn css_text(&self) -> String {
        inject_styles()
    }

    #[inline]
    pub fn clear(&self) {
        let _ = self;
    }

    /// Ensures this scoped CSS is present in the registry (idempotent).
    pub fn ensure_scoped(&self, style_id: &'static str, raw_css: &'static str) {
        let cfg = self.cfg.read().clone();
        let scope = self.scope_class();
        let layer = cfg.layer.as_ref().map(|v| v.to_string());

        // Provider-level transform (optional prefix rewrite + scoping/layer policy).
        let transformed = self.scoper.transform(raw_css, &cfg, scope.as_str());
        // Runtime-level rewrite unification with macro/scoped-style path.
        let (runtime_rewritten, runtime_sig, runtime_rewrite_enabled) =
            CssInJs::rewrite_for_runtime(transformed.as_str());

        let provider_sig = Self::rewrite_signature(&cfg);
        let rewrite_signature =
            Self::merge_rewrite_signatures(provider_sig.as_str(), runtime_sig.as_deref());
        let rewrite_enabled = !rewrite_signature.is_empty() || runtime_rewrite_enabled;

        let entry = StyleEntry::new(style_id, scope.clone(), runtime_rewritten)
            .with_tier(StyleTier::Runtime)
            .with_layer(layer)
            .with_rewrite_signature((rewrite_enabled).then_some(rewrite_signature))
            .with_hash(Some(scope))
            .with_rewrite_enabled(rewrite_enabled);

        if register_or_update(entry) {
            flush_head();
        }
    }

    #[inline]
    fn rewrite_signature(cfg: &StyleConfig) -> String {
        let mut key = String::with_capacity(96);
        if let Some(v) = cfg.prefix_cls.as_deref() {
            key.push_str("|p=");
            key.push_str(v.as_ref());
        }
        if let Some(v) = cfg.icon_prefix_cls.as_deref() {
            key.push_str("|ip=");
            key.push_str(v.as_ref());
        }
        if let Some(v) = cfg.css_var_prefix.as_deref() {
            key.push_str("|vp=");
            key.push_str(v.as_ref());
        }

        if let Some(v) = cfg.rewrite_from_prefix_cls.as_deref() {
            key.push_str("|fp=");
            key.push_str(v.as_ref());
        }
        if let Some(v) = cfg.rewrite_from_icon_prefix_cls.as_deref() {
            key.push_str("|fip=");
            key.push_str(v.as_ref());
        }
        if let Some(v) = cfg.rewrite_from_css_var_prefix.as_deref() {
            key.push_str("|fvp=");
            key.push_str(v.as_ref());
        }

        if let Some(layer) = cfg.layer.as_deref() {
            key.push_str("|layer=");
            key.push_str(layer.as_ref());
        }
        key
    }

    #[inline]
    fn merge_rewrite_signatures(provider_sig: &str, runtime_sig: Option<&str>) -> String {
        let provider_sig = provider_sig.trim();
        let runtime_sig = runtime_sig.map(str::trim).filter(|v| !v.is_empty());
        match (provider_sig.is_empty(), runtime_sig) {
            (true, None) => String::new(),
            (false, None) => provider_sig.to_string(),
            (true, Some(sig)) => sig.to_string(),
            (false, Some(sig)) => format!("{provider_sig}|{sig}"),
        }
    }
}

// ============================================================================
// Internal helpers (no module-level fns)
// ============================================================================

struct StableHash;

impl StableHash {
    /// Stable FNV-1a 64-bit hash.
    #[inline]
    pub fn fnv1a64(chunks: &[&[u8]]) -> u64 {
        const OFF: u64 = 0xcbf29ce484222325;
        const PRIME: u64 = 0x100000001b3;
        let mut h = OFF;
        for chunk in chunks {
            for b in *chunk {
                h ^= *b as u64;
                h = h.wrapping_mul(PRIME);
            }
        }
        h
    }
}

struct CssTransform;

impl CssTransform {
    /// Rewrite prefixes only when user supplied both:
    /// - rewrite_from_* (the source prefix to rewrite FROM)
    /// - *_prefix (the target prefix to rewrite TO)
    ///
    /// If any side is None/empty, that rewrite is skipped.
    pub fn rewrite_prefixed_css(raw: &str, cfg: &StyleConfig) -> String {
        let mut out = raw.to_string();

        // 1) class prefix rewrite: ".{from}-" => ".{to}-"
        if let (Some(from), Some(to)) = (
            cfg.rewrite_from_prefix_cls
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty()),
            cfg.prefix_cls
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty()),
        ) && from != to
        {
            let from_dot = format!(".{from}-");
            let to_dot = format!(".{to}-");
            out = out.replace(from_dot.as_str(), to_dot.as_str());

            // Also rewrite class tokens in attribute selectors: "from-" and " from-"
            let qstart_from = format!("\"{from}-");
            let qstart_to = format!("\"{to}-");
            out = out.replace(qstart_from.as_str(), qstart_to.as_str());

            let qcontains_from = format!("\" {from}-");
            let qcontains_to = format!("\" {to}-");
            out = out.replace(qcontains_from.as_str(), qcontains_to.as_str());

            let sqstart_from = format!("'{from}-");
            let sqstart_to = format!("'{to}-");
            out = out.replace(sqstart_from.as_str(), sqstart_to.as_str());

            let sqcontains_from = format!("' {from}-");
            let sqcontains_to = format!("' {to}-");
            out = out.replace(sqcontains_from.as_str(), sqcontains_to.as_str());
        }

        // 2) icon prefix rewrite: ".{from}-" => ".{to}-"
        if let (Some(from), Some(to)) = (
            cfg.rewrite_from_icon_prefix_cls
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty()),
            cfg.icon_prefix_cls
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty()),
        ) && from != to
        {
            let from_dot = format!(".{from}-");
            let to_dot = format!(".{to}-");
            out = out.replace(from_dot.as_str(), to_dot.as_str());
        }

        // 3) css var prefix rewrite: "--{from}-" => "--{to}-"
        if let (Some(from), Some(to)) = (
            cfg.rewrite_from_css_var_prefix
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty()),
            cfg.css_var_prefix
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty()),
        ) && from != to
        {
            let from_var = format!("--{from}-");
            let to_var = format!("--{to}-");
            out = out.replace(from_var.as_str(), to_var.as_str());
        }

        out
    }

    /// Naive scoper (string-based). For 100% nesting correctness use a lightningcss-based scoper.
    pub fn scope_each_rule(css: &str, scope_prefix: &str) -> String {
        let mut out = String::new();

        for block in css.split('}') {
            let Some((sel, body)) = block.split_once('{') else {
                continue;
            };
            let sel = sel.trim();
            let body = body.trim();
            if sel.is_empty() || body.is_empty() {
                continue;
            }

            // don't scope keyframes header
            if sel.starts_with("@keyframes") || sel.starts_with("@-webkit-keyframes") {
                out.push_str(sel);
                out.push('{');
                out.push_str(body);
                out.push_str("}\n");
                continue;
            }

            // keep at-rules as-is (nested correctness requires AST scoping)
            if sel.starts_with("@media")
                || sel.starts_with("@supports")
                || sel.starts_with("@container")
                || sel.starts_with("@layer")
            {
                out.push_str(sel);
                out.push('{');
                out.push_str(body);
                out.push_str("}\n");
                continue;
            }

            let scoped_sel = sel
                .split(',')
                .map(|s| format!("{scope_prefix} {}", s.trim()))
                .collect::<Vec<_>>()
                .join(", ");

            out.push_str(&scoped_sel);
            out.push('{');
            out.push_str(body);
            out.push_str("}\n");
        }

        out
    }
}
