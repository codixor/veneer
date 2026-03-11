//! Scoping engine interface.
//!
//! This provides a single interface for the three style pipelines currently
//! supported by the macro crate:
//! - plain CSS / LightningCSS
//! - SCSS
//! - ACSS
//!
//! Today, all engines delegate to the same LightningCSS AST scoper for
//! deterministic behavior. The interface keeps room for per-kind behavior
//! without changing macro call-sites.

use super::parser::{ScopedCss, extract_class_names_fallback, scope_with_lightningcss};

/// Strategy selector for scoped CSS generation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ScopeEngineKind {
    /// Default parser + scoper based on LightningCSS.
    #[default]
    LightningCss,
    /// SCSS-preprocessed CSS scoping strategy.
    Scss,
    /// ACSS-preprocessed CSS scoping strategy.
    Acss,
}

pub(crate) trait ScopeEngine {
    fn scope(&self, css: &str, scope: &str, minify: bool) -> ScopedCss;
}

#[inline]
pub(crate) fn parse_and_scope_with_engine(
    css: &str,
    scope: &str,
    minify: bool,
    engine: ScopeEngineKind,
) -> ScopedCss {
    match engine {
        ScopeEngineKind::LightningCss => LightningCssScopeEngine.scope(css, scope, minify),
        ScopeEngineKind::Scss => ScssScopeEngine.scope(css, scope, minify),
        ScopeEngineKind::Acss => AcssScopeEngine.scope(css, scope, minify),
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct LightningCssScopeEngine;

impl ScopeEngine for LightningCssScopeEngine {
    #[inline]
    fn scope(&self, css: &str, scope: &str, minify: bool) -> ScopedCss {
        let css_src = css.to_string();
        let scope_src = scope.to_string();
        match scope_with_lightningcss(css_src.as_str(), scope_src.as_str(), minify) {
            Ok(ok) => ok,
            Err(_) => ScopedCss {
                scoped: css.to_string(),
                class_names: extract_class_names_fallback(css),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct ScssScopeEngine;

impl ScopeEngine for ScssScopeEngine {
    #[inline]
    fn scope(&self, css: &str, scope: &str, minify: bool) -> ScopedCss {
        // SCSS is precompiled before scoping; we use the same AST scoper.
        LightningCssScopeEngine.scope(css, scope, minify)
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct AcssScopeEngine;

impl ScopeEngine for AcssScopeEngine {
    #[inline]
    fn scope(&self, css: &str, scope: &str, minify: bool) -> ScopedCss {
        // ACSS is precompiled before scoping; we use the same AST scoper.
        LightningCssScopeEngine.scope(css, scope, minify)
    }
}
