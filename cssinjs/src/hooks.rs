use dioxus::prelude::*;

use crate::backend as style_backend;
use crate::scoped_style::{IntoScopedStyleSpec, ScopedClassMap};
use crate::{BundleExtractState, CssInJsCtx, CssInJsRuntime, CssInJsRuntimeConfig};

#[must_use]
pub fn use_cssinjs() -> CssInJsRuntime {
    use_context::<CssInJsCtx>().runtime
}

#[must_use]
pub fn try_use_cssinjs() -> Option<CssInJsRuntime> {
    try_use_context::<CssInJsCtx>().map(|ctx| ctx.runtime)
}

#[must_use]
pub fn use_cssinjs_config() -> Signal<CssInJsRuntimeConfig> {
    use_context::<CssInJsCtx>().config
}

#[must_use]
pub fn try_use_cssinjs_config() -> Option<Signal<CssInJsRuntimeConfig>> {
    try_use_context::<CssInJsCtx>().map(|ctx| ctx.config)
}

#[must_use]
pub fn use_cssinjs_extract_state() -> Signal<BundleExtractState> {
    use_context::<CssInJsCtx>().extract_state
}

#[must_use]
pub fn try_use_cssinjs_extract_state() -> Option<Signal<BundleExtractState>> {
    try_use_context::<CssInJsCtx>().map(|ctx| ctx.extract_state)
}

#[must_use]
pub fn use_cssinjs_style() -> style_backend::StyleHandle {
    style_backend::use_style()
}

#[must_use]
pub fn use_scoped_style<S>(style: S) -> ScopedClassMap
where
    S: IntoScopedStyleSpec + Copy + 'static,
{
    let style_spec = style.into_scoped_style_spec();
    let style_to_register = style_spec;
    use_hook(move || {
        if style_to_register.style_id().is_empty() || style_to_register.raw_css().is_empty() {
            return;
        }
        style_to_register.ensure_registered();
        let _ = crate::inject_style(style_to_register.scope());
    });
    ScopedClassMap::from_spec(style_spec)
}
