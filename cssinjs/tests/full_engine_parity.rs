use std::sync::{Arc, Mutex};

use cssinjs::{
    BundleBuildOptions, BundleExtractState, CssInJs, CssInJsConfig, CssInJsRuntime,
    CssVarRegisterInput, CssVarTokenMap, HeadStyleConfig, STYLE_REGISTRY, StyleEntry, StyleTier,
    build_bundle, build_bundle_once, register_or_update, set_head_style_config,
};

static TEST_LOCK: Mutex<()> = Mutex::new(());

fn test_guard() -> std::sync::MutexGuard<'static, ()> {
    match TEST_LOCK.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn clear_runtime_state(runtime: CssInJsRuntime) {
    runtime.clear_dev_styles();
    runtime.clear();

    if let Some(lock) = STYLE_REGISTRY.get() {
        match lock.write() {
            Ok(mut registry) => registry.clear_force(),
            Err(poisoned) => {
                let mut registry = poisoned.into_inner();
                registry.clear_force();
            }
        }
    }

    let _ = CssInJs::set_config(CssInJsConfig::default());
    let _ = set_head_style_config(HeadStyleConfig::default());
}

#[test]
fn full_engine_runtime_injector_lifecycle_mount_update_unmount() {
    let _guard = test_guard();
    let runtime = CssInJsRuntime;
    clear_runtime_state(runtime);

    let mut once = BundleExtractState::default();

    assert!(register_or_update(
        StyleEntry::new(
            "parity-runtime-lifecycle",
            "parity-runtime-lifecycle",
            ".parity-runtime-lifecycle{opacity:0.12;}"
        )
        .with_tier(StyleTier::Runtime)
    ));

    let first = build_bundle_once(&mut once, BundleBuildOptions::default());
    assert!(first.css.contains("opacity:0.12"));
    assert_eq!(first.metadata.runtime_injector_count, 1);
    assert_eq!(first.metadata.cssinjs_record_count, 0);

    let second = build_bundle_once(&mut once, BundleBuildOptions::default());
    assert!(second.css.is_empty());
    assert_eq!(second.metadata.total_record_count, 0);

    assert!(register_or_update(
        StyleEntry::new(
            "parity-runtime-lifecycle",
            "parity-runtime-lifecycle",
            ".parity-runtime-lifecycle{opacity:0.94;}"
        )
        .with_tier(StyleTier::Runtime)
    ));

    let third = build_bundle_once(&mut once, BundleBuildOptions::default());
    assert!(third.css.contains("opacity:0.94"));
    assert!(!third.css.contains("opacity:0.12"));
    assert_eq!(third.metadata.runtime_injector_count, 1);
    assert_eq!(third.metadata.cssinjs_record_count, 0);

    let full_after_update = build_bundle(BundleBuildOptions::default());
    assert!(full_after_update.css.contains("opacity:0.94"));
    assert!(!full_after_update.css.contains("opacity:0.12"));
    assert_eq!(full_after_update.metadata.runtime_injector_count, 1);

    if let Some(lock) = STYLE_REGISTRY.get() {
        match lock.write() {
            Ok(mut registry) => registry.clear_force(),
            Err(poisoned) => poisoned.into_inner().clear_force(),
        }
    }

    let full_after_unmount = build_bundle(BundleBuildOptions::default());
    assert!(!full_after_unmount.css.contains(".parity-runtime-lifecycle"));
    assert_eq!(full_after_unmount.metadata.runtime_injector_count, 0);

    clear_runtime_state(runtime);
}

#[test]
fn full_engine_hmr_replace_has_no_stale_fragments() {
    let _guard = test_guard();
    let runtime = CssInJsRuntime;
    clear_runtime_state(runtime);

    assert!(register_or_update(
        StyleEntry::new(
            "parity-hmr-runtime",
            "parity-hmr-runtime",
            ".parity-hmr-runtime{border-top-color:#111111;}"
        )
        .with_tier(StyleTier::Runtime)
    ));
    assert!(
        runtime
            .upsert_dev_style("parity-hmr-dev", "title", ".parity-hmr-dev{color:#333333;}",)
            .is_some()
    );

    let before = build_bundle(BundleBuildOptions::default());
    assert!(before.css.contains("#111111"));
    assert!(before.css.contains("#333333"));
    assert_eq!(before.metadata.runtime_injector_count, 1);
    assert_eq!(before.metadata.cssinjs_record_count, 1);

    assert!(register_or_update(
        StyleEntry::new(
            "parity-hmr-runtime",
            "parity-hmr-runtime",
            ".parity-hmr-runtime{border-top-color:#222222;}"
        )
        .with_tier(StyleTier::Runtime)
    ));
    assert!(
        runtime
            .upsert_dev_style("parity-hmr-dev", "title", ".parity-hmr-dev{color:#444444;}",)
            .is_some()
    );

    let after = build_bundle(BundleBuildOptions::default());
    assert!(after.css.contains("#222222"));
    assert!(after.css.contains("#444444"));
    assert!(!after.css.contains("#111111"));
    assert!(!after.css.contains("#333333"));
    assert_eq!(after.metadata.runtime_injector_count, 1);
    assert_eq!(after.metadata.cssinjs_record_count, 1);

    clear_runtime_state(runtime);
}

#[test]
fn full_engine_theme_switch_is_incremental_and_stable() {
    let _guard = test_guard();
    let runtime = CssInJsRuntime;
    clear_runtime_state(runtime);

    let mut once = BundleExtractState::default();
    let cfg = CssVarRegisterInput {
        path: vec![Arc::<str>::from("theme"), Arc::<str>::from("engine")],
        key: Arc::<str>::from("engine-theme"),
        style_id: Some(Arc::<str>::from("engine-theme-vars")),
        prefix: Some(Arc::<str>::from("engine")),
        scope: vec![Arc::<str>::from("theme-host")],
        token_hash: Some(Arc::<str>::from("theme-seed-v1")),
        hash_class: Some(Arc::<str>::from("theme-runtime-hash")),
        ..CssVarRegisterInput::default()
    };

    let mut light = CssVarTokenMap::new();
    light.insert("colorPrimary".to_string(), "#1677ff".into());
    light.insert("borderRadius".to_string(), 8.into());

    let first_register = CssInJs::register_css_vars(&cfg, &light);
    assert_eq!(
        first_register.hash_class.as_deref(),
        Some("theme-runtime-hash")
    );
    let first_cache_key = first_register
        .registration
        .as_ref()
        .map(|value| value.cache_key.clone())
        .expect("first theme registration");

    let first_once = build_bundle_once(&mut once, BundleBuildOptions::default());
    assert!(first_once.css.contains("#1677ff"));
    assert!(first_once.css.contains("--engine-border-radius:8px"));
    assert_eq!(first_once.metadata.cssinjs_record_count, 1);

    let second_register = CssInJs::register_css_vars(&cfg, &light);
    let second_cache_key = second_register
        .registration
        .as_ref()
        .map(|value| value.cache_key.clone())
        .expect("second theme registration");
    assert_eq!(first_cache_key, second_cache_key);

    let second_once = build_bundle_once(&mut once, BundleBuildOptions::default());
    assert!(second_once.css.is_empty());
    assert_eq!(second_once.metadata.total_record_count, 0);

    let mut dark = CssVarTokenMap::new();
    dark.insert("colorPrimary".to_string(), "#111111".into());
    dark.insert("borderRadius".to_string(), 6.into());

    let dark_register = CssInJs::register_css_vars(&cfg, &dark);
    let dark_cache_key = dark_register
        .registration
        .as_ref()
        .map(|value| value.cache_key.clone())
        .expect("dark theme registration");
    assert_ne!(first_cache_key, dark_cache_key);
    assert_eq!(
        dark_register.hash_class.as_deref(),
        Some("theme-runtime-hash")
    );

    let dark_once = build_bundle_once(&mut once, BundleBuildOptions::default());
    assert!(dark_once.css.contains("#111111"));
    assert!(dark_once.css.contains("--engine-border-radius:6px"));
    assert!(!dark_once.css.contains("#1677ff"));
    assert_eq!(CssInJs::len(), 1);

    let full_dark = build_bundle(BundleBuildOptions::default());
    assert!(full_dark.css.contains("#111111"));
    assert!(!full_dark.css.contains("#1677ff"));
    assert_eq!(full_dark.metadata.cssinjs_record_count, 1);

    let light_again_register = CssInJs::register_css_vars(&cfg, &light);
    let light_again_cache_key = light_again_register
        .registration
        .as_ref()
        .map(|value| value.cache_key.clone())
        .expect("light-again theme registration");
    assert_ne!(dark_cache_key, light_again_cache_key);

    let light_again_once = build_bundle_once(&mut once, BundleBuildOptions::default());
    assert!(light_again_once.css.is_empty());
    assert_eq!(light_again_once.metadata.total_record_count, 0);
    assert_eq!(CssInJs::len(), 1);

    let full_light_again = build_bundle(BundleBuildOptions::default());
    assert!(full_light_again.css.contains("#1677ff"));
    assert!(!full_light_again.css.contains("#111111"));
    assert_eq!(full_light_again.metadata.cssinjs_record_count, 1);

    clear_runtime_state(runtime);
}
