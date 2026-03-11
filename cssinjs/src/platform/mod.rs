#[cfg(all(not(target_arch = "wasm32"), feature = "liveview"))]
pub mod liveview;
pub mod native;
pub mod ssr;
pub mod web;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CssInJsCapabilities {
    pub runtime_injection: bool,
    pub head_bridge: bool,
    pub document_head: bool,
    pub live_edit: bool,
    pub revision_events: bool,
}

impl CssInJsCapabilities {
    #[must_use]
    pub const fn unavailable() -> Self {
        Self {
            runtime_injection: false,
            head_bridge: false,
            document_head: false,
            live_edit: false,
            revision_events: false,
        }
    }
}

#[must_use]
pub fn detect_capabilities() -> CssInJsCapabilities {
    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    {
        web::detect_capabilities()
    }
    #[cfg(all(target_arch = "wasm32", not(feature = "web")))]
    {
        CssInJsCapabilities::unavailable()
    }
    #[cfg(all(not(target_arch = "wasm32"), feature = "ssr", feature = "liveview"))]
    {
        // Conservative combined lane: keep liveview runtime capabilities while
        // unblocking additive ssr + liveview feature wiring.
        liveview::detect_capabilities()
    }
    #[cfg(all(
        not(target_arch = "wasm32"),
        feature = "liveview",
        not(feature = "ssr")
    ))]
    {
        liveview::detect_capabilities()
    }
    #[cfg(all(
        not(target_arch = "wasm32"),
        not(feature = "liveview"),
        feature = "ssr"
    ))]
    {
        ssr::detect_capabilities()
    }
    #[cfg(all(
        not(target_arch = "wasm32"),
        not(feature = "liveview"),
        not(feature = "ssr")
    ))]
    {
        native::detect_capabilities()
    }
}

pub fn install_platform_hooks() {
    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    {
        web::install_platform_hooks();
    }
    #[cfg(all(not(target_arch = "wasm32"), feature = "ssr", feature = "liveview"))]
    {
        ssr::install_platform_hooks();
        liveview::install_platform_hooks();
    }
    #[cfg(all(
        not(target_arch = "wasm32"),
        feature = "liveview",
        not(feature = "ssr")
    ))]
    {
        liveview::install_platform_hooks();
    }
    #[cfg(all(
        not(target_arch = "wasm32"),
        not(feature = "liveview"),
        feature = "ssr"
    ))]
    {
        ssr::install_platform_hooks();
    }
    #[cfg(all(
        not(target_arch = "wasm32"),
        not(feature = "liveview"),
        not(feature = "ssr")
    ))]
    {
        native::install_platform_hooks();
    }
}
