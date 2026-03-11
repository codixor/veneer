use std::sync::Mutex;

use cssinjs::{
    BundleBuildOptions, BundleExtractCache, BundleExtractState, CssInJsRuntime, STYLE_REGISTRY,
    StyleEntry, build_bundle, build_bundle_once, build_bundle_once_with_cache, register_or_update,
    write_bundle_files,
};

static TEST_LOCK: Mutex<()> = Mutex::new(());

fn clear_runtime_state(runtime: CssInJsRuntime) {
    runtime.clear_dev_styles();
    runtime.clear_dev_tokens();
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
}

#[test]
fn bundle_snapshot_is_deterministic() {
    let _guard = TEST_LOCK.lock().expect("bundle test lock");
    let runtime = CssInJsRuntime;
    clear_runtime_state(runtime);

    let dev = runtime.upsert_dev_style("bundle", "button", ".btn{color:red;}");
    assert!(dev.is_some());

    let first = build_bundle(BundleBuildOptions::default());
    let second = build_bundle(BundleBuildOptions::default());

    assert_eq!(first.css, second.css);
    assert_eq!(first.metadata.css_xxh3_64, second.metadata.css_xxh3_64);
    assert_eq!(
        first.metadata.total_record_count,
        second.metadata.total_record_count
    );

    clear_runtime_state(runtime);
}

#[test]
fn bundle_writer_emits_css_summary() {
    let _guard = TEST_LOCK.lock().expect("bundle test lock");
    let runtime = CssInJsRuntime;
    clear_runtime_state(runtime);

    let root = std::env::temp_dir().join(format!(
        "cssinjs-bundle-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |v| v.as_nanos())
    ));
    let css_path = root.join("assets/style.css");

    let summary = write_bundle_files(css_path.as_path(), BundleBuildOptions::default())
        .expect("bundle write should succeed");

    assert!(css_path.exists());

    let css = std::fs::read_to_string(css_path.as_path()).expect("css file should be readable");
    assert_eq!(summary.css_len, css.len());
    assert_eq!(summary.css_path, css_path);
    assert_eq!(summary.css_xxh3_64.len(), 16);

    let _ = std::fs::remove_dir_all(root);
    clear_runtime_state(runtime);
}

#[test]
fn bundle_once_emits_only_new_styles() {
    let _guard = TEST_LOCK.lock().expect("bundle test lock");
    let runtime = CssInJsRuntime;
    clear_runtime_state(runtime);

    let first_style = runtime.upsert_dev_style("bundle-once", "a", ".a{color:red;}");
    assert!(first_style.is_some());

    let mut state = BundleExtractState::default();
    let first = build_bundle_once(&mut state, BundleBuildOptions::default());
    assert!(first.css.contains(".a"));
    assert!(!first.css.is_empty());
    assert!(state.extracted_count() > 0);

    let second = build_bundle_once(&mut state, BundleBuildOptions::default());
    assert!(second.css.is_empty());
    assert_eq!(second.metadata.total_record_count, 0);

    let second_style = runtime.upsert_dev_style("bundle-once", "b", ".b{color:blue;}");
    assert!(second_style.is_some());
    let third = build_bundle_once(&mut state, BundleBuildOptions::default());
    assert!(third.css.contains(".b"));
    assert!(!third.css.contains(".a"));

    clear_runtime_state(runtime);
}

#[test]
fn bundle_can_emit_cache_path_marker_css() {
    let _guard = TEST_LOCK.lock().expect("bundle test lock");
    let runtime = CssInJsRuntime;
    clear_runtime_state(runtime);

    let style = runtime.upsert_dev_style("bundle-marker", "title", ".title{color:green;}");
    assert!(style.is_some());

    let output = build_bundle(BundleBuildOptions {
        emit_cache_path_marker: true,
        ..BundleBuildOptions::default()
    });

    assert!(output.css.contains(".data-ant-cssinjs-cache-path"));
    assert!(output.metadata.cache_path_css_len > 0);
    clear_runtime_state(runtime);
}

#[test]
fn bundle_once_emits_runtime_style_changes_for_same_cache_key() {
    let _guard = TEST_LOCK.lock().expect("bundle test lock");
    let runtime = CssInJsRuntime;
    clear_runtime_state(runtime);

    assert!(register_or_update(StyleEntry::new(
        "runtime-once",
        "runtime-once-scope",
        ".runtime-once{color:red;}"
    )));

    let mut state = BundleExtractState::default();
    let first = build_bundle_once(&mut state, BundleBuildOptions::default());
    assert!(first.css.contains("color:red"));
    assert!(first.metadata.total_record_count > 0);

    let second = build_bundle_once(&mut state, BundleBuildOptions::default());
    assert!(second.css.is_empty());
    assert_eq!(second.metadata.total_record_count, 0);

    assert!(register_or_update(StyleEntry::new(
        "runtime-once",
        "runtime-once-scope",
        ".runtime-once{color:blue;}"
    )));

    let third = build_bundle_once(&mut state, BundleBuildOptions::default());
    assert!(third.css.contains("color:blue"));
    assert!(!third.css.contains("color:red"));
    assert!(third.metadata.total_record_count > 0);

    clear_runtime_state(runtime);
}

#[test]
fn bundle_once_cache_entities_are_isolated() {
    let _guard = TEST_LOCK.lock().expect("bundle test lock");
    let runtime = CssInJsRuntime;
    clear_runtime_state(runtime);

    let style = runtime.upsert_dev_style("cache-entity", "title", ".title{color:red;}");
    assert!(style.is_some());

    let mut cache = BundleExtractCache::default();
    let a1 = build_bundle_once_with_cache(&mut cache, "entity-a", BundleBuildOptions::default());
    assert!(a1.css.contains("color:red"));

    let a2 = build_bundle_once_with_cache(&mut cache, "entity-a", BundleBuildOptions::default());
    assert!(a2.css.is_empty());

    let b1 = build_bundle_once_with_cache(&mut cache, "entity-b", BundleBuildOptions::default());
    assert!(b1.css.contains("color:red"));
    assert_eq!(cache.entity_count(), 2);

    let style_update = runtime.upsert_dev_style("cache-entity", "title", ".title{color:blue;}");
    assert!(style_update.is_some());

    let a3 = build_bundle_once_with_cache(&mut cache, "entity-a", BundleBuildOptions::default());
    assert!(a3.css.contains("color:blue"));

    let b2 = build_bundle_once_with_cache(&mut cache, "entity-b", BundleBuildOptions::default());
    assert!(b2.css.contains("color:blue"));

    assert!(cache.remove("entity-b"));
    assert_eq!(cache.entity_count(), 1);
    cache.clear();
    assert_eq!(cache.entity_count(), 0);

    clear_runtime_state(runtime);
}

#[test]
fn bundle_once_emits_dev_token_changes_for_same_key() {
    let _guard = TEST_LOCK.lock().expect("bundle test lock");
    let runtime = CssInJsRuntime;
    clear_runtime_state(runtime);

    let mut light = cssinjs::CssVarTokenMap::new();
    light.insert("colorPrimary".to_string(), "#1677ff".into());
    light.insert("borderRadius".to_string(), 8.into());

    let first_tokens = runtime.upsert_dev_tokens("bundle-token", "theme", &light);
    assert!(first_tokens.is_some());

    let mut state = BundleExtractState::default();
    let first = build_bundle_once(&mut state, BundleBuildOptions::default());
    assert!(
        first
            .css
            .contains("--dev-bundle-token-color-primary:#1677ff;")
    );
    assert!(first.metadata.total_record_count > 0);

    let second = build_bundle_once(&mut state, BundleBuildOptions::default());
    assert!(second.css.is_empty());
    assert_eq!(second.metadata.total_record_count, 0);

    let mut dark = cssinjs::CssVarTokenMap::new();
    dark.insert("colorPrimary".to_string(), "#111111".into());
    dark.insert("borderRadius".to_string(), 6.into());

    let second_tokens = runtime.upsert_dev_tokens("bundle-token", "theme", &dark);
    assert!(second_tokens.is_some());

    let third = build_bundle_once(&mut state, BundleBuildOptions::default());
    assert!(
        third
            .css
            .contains("--dev-bundle-token-color-primary:#111111;")
    );
    assert!(
        !third
            .css
            .contains("--dev-bundle-token-color-primary:#1677ff;")
    );
    assert!(third.metadata.total_record_count > 0);

    clear_runtime_state(runtime);
}
