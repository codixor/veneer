use crate::platform::CssInJsCapabilities;

#[must_use]
pub fn detect_capabilities() -> CssInJsCapabilities {
    CssInJsCapabilities {
        runtime_injection: true,
        head_bridge: true,
        document_head: false,
        live_edit: true,
        revision_events: true,
    }
}

pub fn install_platform_hooks() {
    // Web hook point for cssinjs runtime effects.
}
