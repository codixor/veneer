/// Smoke test: verify cssinjs public types and functions are accessible.
/// This ensures the liveview build path does not break the cssinjs API surface.

#[test]
fn cssinjs_core_types_are_constructible() {
    // HeadStyleConfig is a key configuration struct for style injection.
    let cfg = cssinjs::HeadStyleConfig::default();
    // Verify default values are sensible (not panicking is the main assertion).
    let _ = cfg;
}

#[test]
fn cssinjs_capabilities_detect_does_not_panic() {
    let caps = cssinjs::detect_capabilities();
    #[cfg(all(not(target_arch = "wasm32"), feature = "ssr", feature = "liveview"))]
    {
        assert!(caps.head_bridge);
        assert!(caps.document_head);
        assert!(caps.live_edit);
        assert!(caps.revision_events);
        assert!(!caps.runtime_injection);
    }
    #[cfg(all(
        not(target_arch = "wasm32"),
        feature = "liveview",
        not(feature = "ssr")
    ))]
    {
        assert!(caps.head_bridge);
        assert!(caps.document_head);
        assert!(caps.live_edit);
        assert!(caps.revision_events);
        assert!(!caps.runtime_injection);
    }
    #[cfg(all(
        not(target_arch = "wasm32"),
        feature = "ssr",
        not(feature = "liveview")
    ))]
    {
        assert!(!caps.head_bridge);
        assert!(!caps.document_head);
        assert!(!caps.live_edit);
        assert!(!caps.revision_events);
        assert!(!caps.runtime_injection);
    }
    #[cfg(all(
        not(target_arch = "wasm32"),
        not(feature = "liveview"),
        not(feature = "ssr")
    ))]
    {
        assert!(!caps.head_bridge);
        assert!(caps.document_head);
        assert!(!caps.live_edit);
        assert!(caps.revision_events);
        assert!(!caps.runtime_injection);
    }
    #[cfg(any(
        target_arch = "wasm32",
        feature = "ssr",
        all(
            not(target_arch = "wasm32"),
            feature = "liveview",
            not(feature = "ssr")
        ),
        all(not(target_arch = "wasm32"), feature = "liveview", feature = "ssr")
    ))]
    {
        let _ = caps.document_head;
    }
}

#[test]
fn cssinjs_install_platform_hooks_is_callable() {
    cssinjs::install_platform_hooks();
}

#[test]
fn cssinjs_hash_priority_variants_exist() {
    // Verify the HashPriority enum variants are accessible.
    let _low = cssinjs::HashPriority::Low;
    let _high = cssinjs::HashPriority::High;
}

#[test]
fn cssinjs_style_tier_default_exists() {
    let tier = cssinjs::StyleTier::Base;
    let _ = tier;
}
