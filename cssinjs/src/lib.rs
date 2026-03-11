#![forbid(unsafe_code)]

mod backend;
mod engine;
mod scoped_style;
mod statistics;

pub mod ant_api;
pub mod bundle;
pub mod compiler;
pub mod hooks;
pub mod platform;
pub mod plugin;
pub mod provider;
pub mod runtime;

pub use engine::style_provider;
pub use engine::{
    CssInJs, CssInJsConfig, CssInJsLifecycleCfg, CssInJsStyleInput,
    CssVarRegisterInput as CoreCssVarRegisterInput,
    CssVarRegisterOutput as CoreCssVarRegisterOutput, CssVarTokenMap as CoreCssVarTokenMap,
    CssVarTokenValue as CoreCssVarTokenValue, HashPriority, HeadStyleConfig, STYLE_REGISTRY,
    ScopedStyle, StyleConfig, StyleEntry, StyleHandle, StyleProvider, StyleTier, flush_head,
    head_style_config, inject_style, inject_styles, inject_styles_arc, register_or_update,
    runtime_style_records, set_head_style_config, should_emit_scope_hash_attr, use_style,
};

pub use ant_api::{
    AbstractCalculator, CSSInterpolation, CSSObject, CacheTokenOptions, CacheTokenResult, CalcMode,
    CalcOperand, CssLintContext, CssParseCfg, DerivativeFn, ExtractStyleOptions, Keyframes, Linter,
    Px2RemOptions, StyleCache, StyleContext, StyleProviderProps, StyleRegisterInput,
    StyleRegisterResult, Theme, Transformer, UseCssVarRegisterOptions, UseStyleRegisterOptions,
    auto_prefix_transformer, create_cache, create_theme, debug_take_lint_warnings_for_tests,
    extract_style, extract_style_output, gen_calc, get_computed_token,
    legacy_logical_properties_transformer, legacy_not_selector_linter, logical_properties_linter,
    merge_token, nan_linter, parent_selector_linter, px2rem_transformer, token_to_css_var,
    use_cache_token, use_css_var_register, use_style_register,
};
pub use bundle::{
    BundleBuildOptions, BundleExtractCache, BundleExtractState, BundleMetadata, BundleOutput,
    BundleWriteSummary, build_bundle, build_bundle_once, build_bundle_once_with_cache,
    write_bundle_files,
};
pub use hooks::{
    try_use_cssinjs, try_use_cssinjs_config, try_use_cssinjs_extract_state, use_cssinjs,
    use_cssinjs_config, use_cssinjs_extract_state, use_cssinjs_style, use_scoped_style,
};
pub use platform::{CssInJsCapabilities, detect_capabilities, install_platform_hooks};
pub use plugin::CssInJsPlugin;
pub use provider::{CssInJsCtx, CssInJsProvider, CssInJsProviderProps};
pub use runtime::{
    CssInJsRuntime, CssInJsRuntimeConfig, CssVarRegisterInput, CssVarRegisterOutput,
    CssVarTokenMap, CssVarTokenValue, DevStyleRecord, DevTokenRecord, unit,
};
pub use scoped_style::{IntoScopedStyleSpec, ScopedClassEntry, ScopedClassMap, ScopedStyleSpec};
pub use statistics::{
    StatisticEntry, StatisticToken, StatisticValue, TrackedTokenMap,
    debug_reset_statistics_for_tests, statistic, statistic_build, statistic_token,
};
