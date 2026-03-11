use std::sync::{Arc, Mutex};

use cssinjs::{
    BundleBuildOptions, CssInJs, CssInJsStyleInput, CssVarRegisterInput, CssVarTokenMap,
    HashPriority, build_bundle,
};

static TEST_LOCK: Mutex<()> = Mutex::new(());

fn reset_runtime() {
    CssInJs::debug_reset_runtime_for_tests();
}

#[test]
fn parity_style_register_flow_identity_dedup() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset_runtime();

    let mut base = CssInJsStyleInput::new("ant-btn", ".ant-btn{color:red;}");
    base.identity_scope = Some(Arc::<str>::from("style|ant-btn"));
    base.hash_class = Some(Arc::<str>::from("ant-btn-hash"));

    let first = CssInJs::register(base.clone()).expect("first register");
    let second = CssInJs::register(base).expect("second register");
    assert_eq!(first.cache_key, second.cache_key);
    assert_eq!(CssInJs::len(), 1);

    let mut replaced = CssInJsStyleInput::new("ant-btn", ".ant-btn{color:blue;}");
    replaced.identity_scope = Some(Arc::<str>::from("style|ant-btn"));
    replaced.hash_class = Some(Arc::<str>::from("ant-btn-hash"));

    let third = CssInJs::register(replaced).expect("replacement register");
    assert_ne!(first.cache_key, third.cache_key);
    assert_eq!(CssInJs::len(), 1);
}

#[test]
fn parity_css_var_register_flow_stability() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset_runtime();

    let cfg = CssVarRegisterInput {
        path: vec![Arc::<str>::from("token"), Arc::<str>::from("antd")],
        key: Arc::<str>::from("ant-v5"),
        style_id: Some(Arc::<str>::from("vars-ant-v5")),
        prefix: Some(Arc::<str>::from("ant")),
        scope: vec![Arc::<str>::from("scope-a")],
        ..CssVarRegisterInput::default()
    };

    let mut token = CssVarTokenMap::new();
    token.insert("colorPrimary".to_string(), "#1677ff".into());
    token.insert("borderRadius".to_string(), 8.into());

    let first = CssInJs::register_css_vars(&cfg, &token);
    let second = CssInJs::register_css_vars(&cfg, &token);

    assert_eq!(first.css_var_key, second.css_var_key);
    assert_eq!(first.style_id, second.style_id);
    assert!(first.css_vars_css.contains("--ant-color-primary"));
    assert!(first.css_vars_css.contains("--ant-border-radius"));

    let first_reg = first
        .registration
        .as_ref()
        .expect("first css-var registration");
    let second_reg = second
        .registration
        .as_ref()
        .expect("second css-var registration");
    assert_eq!(first_reg.cache_key, second_reg.cache_key);
}

#[test]
fn parity_extract_export_contains_style_and_css_var_records() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset_runtime();

    let _ = CssInJs::register(CssInJsStyleInput::new(
        "ant-card",
        ".ant-card{padding:16px;}",
    ))
    .expect("style registration");

    let cfg = CssVarRegisterInput {
        path: vec![Arc::<str>::from("token"), Arc::<str>::from("antd")],
        key: Arc::<str>::from("ant-v5"),
        style_id: Some(Arc::<str>::from("vars-ant-v5")),
        prefix: Some(Arc::<str>::from("ant")),
        ..CssVarRegisterInput::default()
    };

    let mut token = CssVarTokenMap::new();
    token.insert("colorPrimary".to_string(), "#1677ff".into());
    let _ = CssInJs::register_css_vars(&cfg, &token);

    let bundle = build_bundle(BundleBuildOptions::default());
    assert!(!bundle.css.is_empty());
    assert!(bundle.metadata.total_record_count >= 2);
    assert!(
        bundle
            .metadata
            .records
            .iter()
            .any(|record| record.css_var_key.is_some())
    );
}

#[test]
fn parity_v6_hash_priority_low_high_selector_behavior() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset_runtime();

    let mut low = CssInJsStyleInput::new("v6-low", ".ant-v6-low{color:red;}");
    low.hash_class = Some(Arc::<str>::from("v6-low-hash"));
    low.hash_priority = HashPriority::Low;
    let _ = CssInJs::register(low).expect("low hash priority register");

    let mut high = CssInJsStyleInput::new("v6-high", ".ant-v6-high{color:blue;}");
    high.hash_class = Some(Arc::<str>::from("v6-high-hash"));
    high.hash_priority = HashPriority::High;
    let _ = CssInJs::register(high).expect("high hash priority register");

    let css = CssInJs::css_arc().to_string();
    assert!(css.contains(":where(.v6-low-hash)"));
    assert!(css.contains(".v6-high-hash"));
    assert!(!css.contains(":where(.v6-high-hash)"));
}

#[test]
fn parity_v6_layer_wrapping_behavior() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset_runtime();

    let mut layered = CssInJsStyleInput::new("v6-layer", ".ant-v6-layer{display:block;}");
    layered.hash_class = Some(Arc::<str>::from("v6-layer-hash"));
    layered.layer = Some(Arc::<str>::from("antd"));
    let _ = CssInJs::register(layered).expect("layered style register");

    let css = CssInJs::css_arc().to_string();
    assert!(css.contains("@layer antd"));
    assert!(css.contains(".v6-layer-hash"));
}
