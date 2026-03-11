//! Canonical cssinjs runtime engine (migrated from dioxus-style).
#![allow(unused_imports)]
#![allow(dead_code)]

mod css_bundle;
mod cssinjs;
mod precompile_records;
mod runtime_injector;
pub mod style_provider;

pub use css_bundle::{FullCssBundle, FullCssBundleOptions, FullCssBundleSnapshot};
pub use cssinjs::*;
pub use precompile_records::{
    PrecompileStyleRecord, PrecompileStyleRecords, PrecompileStyleSnapshot,
};
pub use runtime_injector::{
    HeadMeta, HeadStyleConfig, RuntimeStyleCssEntry, RuntimeStyleRecord, STYLE_REGISTRY,
    ScopedStyle, SsrGlobalStyle, SsrScopeStyle, SsrThemeStyle, StyleEntry, StyleRegistry,
    StyleTier, export_ssr_global_style, export_ssr_scope_styles, export_ssr_theme_style,
    flush_head, head_style_config, inject_style, inject_styles, inject_styles_arc,
    register_or_update, runtime_style_css_entries, runtime_style_records, set_head_style_config,
    should_emit_scope_hash_attr,
};
pub use style_provider::{
    CssScoper as StyleCssScoper, DefaultCssScoper as DefaultStyleCssScoper, HashPriority,
    StyleConfig, StyleHandle, StyleProvider, use_style,
};

pub use crate::compiler::hash::{
    Hash64Builder, HashCtorParams, HashProfileTarget, HashProfiles, hash_profile_for,
    hash_profiles, reset_hash_profiles, set_hash_profile_for, set_hash_profile_global,
    set_hash_profiles,
};
