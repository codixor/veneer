//! Internal backend boundary for cssinjs runtime.
//!
//! Phase 15 consolidation rule: all core engine usage in `cssinjs/src/*`
//! should flow through this module. This keeps backend ownership isolated and
//! lets us swap internals during crate consolidation without touching public
//! runtime/provider surfaces.

pub(crate) use crate::engine::{
    CssInJs, CssInJsConfig, CssInJsStyleInput, CssVarRegisterInput, CssVarRegisterOutput,
    CssVarTokenMap, CssVarTokenValue, FullCssBundle, FullCssBundleOptions, PrecompileStyleRecord,
    PrecompileStyleRecords, StyleConfig, StyleHandle, StyleProvider,
};

pub(crate) use crate::engine::{runtime_style_css_entries, use_style};
