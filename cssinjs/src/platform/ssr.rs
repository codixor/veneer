use crate::platform::CssInJsCapabilities;

#[must_use]
pub fn detect_capabilities() -> CssInJsCapabilities {
    CssInJsCapabilities::unavailable()
}

pub fn install_platform_hooks() {
    // SSR hook point for cssinjs runtime effects.
}
