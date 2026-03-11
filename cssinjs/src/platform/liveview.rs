use crate::platform::CssInJsCapabilities;

#[must_use]
pub fn detect_capabilities() -> CssInJsCapabilities {
    CssInJsCapabilities {
        runtime_injection: false,
        head_bridge: true,
        document_head: true,
        live_edit: true,
        revision_events: true,
    }
}

pub fn install_platform_hooks() {
    // Liveview head/style updates are handled through dioxus-document.
}
