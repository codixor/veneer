use crate::platform::CssInJsCapabilities;

#[must_use]
pub fn detect_capabilities() -> CssInJsCapabilities {
    CssInJsCapabilities {
        runtime_injection: false,
        head_bridge: false,
        document_head: true,
        live_edit: false,
        revision_events: true,
    }
}

pub fn install_platform_hooks() {
    // Native head/style updates are handled through dioxus-document.
}
