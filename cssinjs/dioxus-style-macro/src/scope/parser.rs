//! CSS parsing + scoping.
//!
//! This implementation intentionally uses a real CSS AST (Lightning CSS) so that
//! nested at-rules like `@media`, `@supports`, and `@layer` are handled with full
//! syntactic correctness.
//!
//! Scoping strategy (stable + deterministic):
//! - We enable Lightning CSS **CSS Modules** and set the rename pattern to
//!   `{scope}_{local}`.
//! - This scopes:
//!   - class names (`.foo` -> `.scope_foo`)
//!   - ids (`#bar` -> `#scope_bar`)
//!   - (by default) keyframes, grid names, container names, and other custom
//!     identifiers (safe and collision-free).
//!
//! Note: unlike the legacy string-scanner, this does **not** attempt to scope
//! bare element selectors (`div { ... }`) using `[data-scope="..."]`. In a CSS
//! Modules workflow, selectors are expected to be class/id anchored.

use std::collections::HashSet;

/// Parsed CSS with scoping applied.
#[derive(Debug, Clone)]
pub struct ScopedCss {
    pub scoped: String,
    /// Unique exported names observed in input (unscoped names, stable sorted).
    pub class_names: Vec<String>,
}

/// Parse + scope CSS with a unique prefix.
///
/// The interface is intentionally infallible to keep the surrounding
/// `StyleCompiler + Expander` pipeline stable.
///
/// - When parsing succeeds, the output is a fully-scoped CSS string.
/// - If the CSS cannot be parsed, the original CSS is returned unchanged and
///   `class_names` is best-effort extracted.
#[must_use]
pub fn parse_and_scope(css: &str, scope: &str, minify: bool) -> ScopedCss {
    // LightningCSS ties option lifetimes to the input source (`'i`).
    // To avoid lifetime coupling between `css` and `scope` (separate &str args),
    // we move both into owned Strings so they share a single local lifetime.
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

pub(super) fn scope_with_lightningcss(
    css: &str,
    scope: &str,
    minify: bool,
) -> Result<ScopedCss, ()> {
    use lightningcss::css_modules::{Config as CssModulesConfig, Pattern};
    use lightningcss::stylesheet::{ParserOptions, PrinterOptions, StyleSheet};

    // `{scope}_{local}` (stable + deterministic, matches Expander's const generation).
    // We use Pattern::parse so we don't need to construct smallvec segments ourselves.
    // Keep the backing string alive until after parsing/printing.
    let pattern_str = format!("{scope}_[local]");
    let pattern = Pattern::parse(&pattern_str).map_err(|_| ())?;

    // CSS Modules config:
    // - keep dashed identifiers (e.g. custom properties) unmodified by default.
    // - leave `pure=false` to preserve legacy behavior (no hard errors).
    let modules = CssModulesConfig {
        pattern,
        dashed_idents: false,
        pure: false,
        ..CssModulesConfig::default()
    };

    let opts = ParserOptions {
        css_modules: Some(modules),
        error_recovery: true,
        filename: "inline.css".into(),
        ..ParserOptions::default()
    };

    let sheet = StyleSheet::parse(css, opts).map_err(|_| ())?;

    let po = PrinterOptions {
        minify,
        ..PrinterOptions::default()
    };

    let res = sheet.to_css(po).map_err(|_| ())?;

    // Collect exported names (class/id exports) for macro constants.
    let mut names: Vec<String> = res
        .exports
        .map(|m| m.into_keys().collect())
        .unwrap_or_default();

    names.sort_unstable();
    names.dedup();

    Ok(ScopedCss {
        scoped: res.code,
        class_names: names,
    })
}

// =============================================================================================
// Best-effort fallback: extract `.class` occurrences from raw CSS.
// =============================================================================================

pub(super) fn extract_class_names_fallback(css: &str) -> Vec<String> {
    let mut out: HashSet<String> = HashSet::new();

    // Very conservative: `.foo` where foo = [a-zA-Z0-9_-]+.
    // This is only used when the AST parser fails.
    let mut it = css.chars().peekable();
    while let Some(ch) = it.next() {
        if ch != '.' {
            continue;
        }
        let mut name = String::new();
        while let Some(&c) = it.peek() {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                name.push(c);
                it.next();
            } else {
                break;
            }
        }
        if !name.is_empty() {
            out.insert(name);
        }
    }

    let mut v: Vec<String> = out.into_iter().collect();
    v.sort_unstable();
    v
}
