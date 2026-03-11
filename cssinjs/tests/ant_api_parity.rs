use std::sync::{Arc, Mutex};

use cssinjs::{
    BundleBuildOptions, CSSInterpolation, CSSObject, CacheTokenOptions, CalcMode, CssInJs,
    CssLintContext, CssVarTokenMap, HashPriority, Keyframes, Px2RemOptions, StyleContext,
    UseCssVarRegisterOptions, UseStyleRegisterOptions, auto_prefix_transformer, create_cache,
    create_theme, debug_take_lint_warnings_for_tests, extract_style, gen_calc, get_computed_token,
    legacy_logical_properties_transformer, legacy_not_selector_linter, logical_properties_linter,
    nan_linter, parent_selector_linter, px2rem_transformer, token_to_css_var, use_cache_token,
    use_css_var_register, use_style_register,
};

static TEST_LOCK: Mutex<()> = Mutex::new(());

fn reset_runtime() {
    CssInJs::debug_reset_runtime_for_tests();
    let _ = debug_take_lint_warnings_for_tests();
}

#[test]
fn keyframes_name_follows_ant_hash_pattern() {
    let keyframes = Keyframes::new("fade-in", "from{opacity:0;}to{opacity:1;}".into());
    assert_eq!(keyframes.get_name(None), "fade-in");
    assert_eq!(keyframes.get_name(Some("abc123")), "fade-in-abc123");
}

#[test]
fn use_style_register_dedups_same_identity() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset_runtime();

    let opts = UseStyleRegisterOptions {
        style_id: Arc::<str>::from("ant-btn"),
        css: Arc::<str>::from(".ant-btn{color:red;}"),
        identity_scope: Some(Arc::<str>::from("style|ant-btn")),
        hash_class: Some(Arc::<str>::from("ant-btn-hash")),
        hash_priority: HashPriority::Low,
        ..UseStyleRegisterOptions::default()
    };

    let first = use_style_register(opts.clone()).expect("first register");
    let second = use_style_register(opts).expect("second register");
    assert_eq!(first.cache_key, second.cache_key);
    assert_eq!(CssInJs::len(), 1);
}

#[test]
fn create_cache_and_extract_style_support_once_semantics() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset_runtime();

    let _ = use_style_register(UseStyleRegisterOptions {
        style_id: Arc::<str>::from("ant-card"),
        css: Arc::<str>::from(".ant-card{padding:16px;}"),
        identity_scope: Some(Arc::<str>::from("style|ant-card")),
        hash_class: Some(Arc::<str>::from("ant-card-hash")),
        ..UseStyleRegisterOptions::default()
    })
    .expect("register");

    let mut cache = create_cache();
    let first = extract_style(
        &mut cache,
        "cache-a",
        cssinjs::ExtractStyleOptions {
            once: true,
            bundle: BundleBuildOptions::default(),
        },
    );
    assert!(first.contains("ant-card"));

    let second = extract_style(
        &mut cache,
        "cache-a",
        cssinjs::ExtractStyleOptions {
            once: true,
            bundle: BundleBuildOptions::default(),
        },
    );
    assert!(second.trim().is_empty());
}

#[test]
fn create_theme_and_use_cache_token_follow_theme_pipeline() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset_runtime();

    let theme = create_theme(vec![Arc::new(|token: &CssVarTokenMap| {
        let mut next = token.clone();
        next.insert("colorPrimary".to_string(), "#52c41a".into());
        next
    })]);

    let mut token = CssVarTokenMap::new();
    token.insert("colorPrimary".to_string(), "#1677ff".into());
    token.insert("borderRadius".to_string(), 8.into());

    let out = use_cache_token(
        &theme,
        &token,
        cssinjs::CacheTokenOptions {
            key: Arc::<str>::from("ant-v6"),
            prefix: Some(Arc::<str>::from("ant")),
            ..cssinjs::CacheTokenOptions::default()
        },
    );

    assert_eq!(theme.derivative_count(), 1);
    assert_eq!(
        out.themed_token.get("colorPrimary").map(|v| v.to_string()),
        Some(String::from("#52c41a"))
    );
    assert!(out.output.css_vars_css.contains("--ant-color-primary"));
}

#[test]
fn use_style_register_replacement_keeps_relative_order() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset_runtime();

    let first = UseStyleRegisterOptions {
        style_id: Arc::<str>::from("order-first"),
        css: Arc::<str>::from(".order-first{color:#111111;}"),
        identity_scope: Some(Arc::<str>::from("style|order-first")),
        hash_class: Some(Arc::<str>::from("order-first-hash")),
        ..UseStyleRegisterOptions::default()
    };
    let second = UseStyleRegisterOptions {
        style_id: Arc::<str>::from("order-second"),
        css: Arc::<str>::from(".order-second{color:#222222;}"),
        identity_scope: Some(Arc::<str>::from("style|order-second")),
        hash_class: Some(Arc::<str>::from("order-second-hash")),
        ..UseStyleRegisterOptions::default()
    };

    let _ = use_style_register(first.clone()).expect("register first");
    let _ = use_style_register(second).expect("register second");

    let first_css = CssInJs::css_arc().to_string();
    let first_pos = first_css.find("#111111").expect("first marker css");
    let second_pos = first_css.find("#222222").expect("second marker css");
    assert!(first_pos < second_pos);

    let updated_first = UseStyleRegisterOptions {
        css: Arc::<str>::from(".order-first{color:#333333;}"),
        ..first
    };
    let _ = use_style_register(updated_first).expect("update first");

    let second_css = CssInJs::css_arc().to_string();
    let updated_first_pos = second_css
        .find("#333333")
        .expect("updated first marker css");
    let second_pos = second_css.find("#222222").expect("second marker css");
    assert!(updated_first_pos < second_pos);
    assert!(!second_css.contains("#111111"));

    let records = CssInJs::records();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].style_id, "order-first");
    assert_eq!(records[1].style_id, "order-second");
}

#[test]
fn style_context_applies_layer_nonce_and_priority() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset_runtime();

    let context = StyleContext {
        hash_priority: HashPriority::High,
        layer: Some(Arc::<str>::from("antd")),
        nonce: Some(Arc::<str>::from("nonce-ant")),
    };

    let _ = use_style_register(UseStyleRegisterOptions {
        style_id: Arc::<str>::from("ctx-style"),
        css: Arc::<str>::from(".ctx-style{margin:0;}"),
        identity_scope: Some(Arc::<str>::from("style|ctx-style")),
        hash_class: Some(Arc::<str>::from("ctx-style-hash")),
        hash_priority: HashPriority::Low,
        context: Some(context.clone()),
        ..UseStyleRegisterOptions::default()
    })
    .expect("register with style context");

    let css = CssInJs::css_arc().to_string();
    assert!(css.contains("@layer antd"));
    assert!(css.contains(".ctx-style-hash"));
    assert!(!css.contains(":where(.ctx-style-hash)"));

    let style_record = CssInJs::records()
        .into_iter()
        .find(|record| record.style_id == "ctx-style")
        .expect("ctx style record");
    assert_eq!(style_record.layer.as_deref(), Some("antd"));
    assert_eq!(style_record.nonce.as_deref(), Some("nonce-ant"));

    let theme = create_theme(Vec::new());
    let mut token = CssVarTokenMap::new();
    token.insert("colorPrimary".to_string(), "#1677ff".into());

    let _ = use_cache_token(
        &theme,
        &token,
        CacheTokenOptions {
            key: Arc::<str>::from("ctx-vars"),
            style_id: Some(Arc::<str>::from("ctx-vars-style")),
            prefix: Some(Arc::<str>::from("ant")),
            hash_class: Some(Arc::<str>::from("ctx-vars-hash")),
            hash_priority: HashPriority::Low,
            context: Some(context),
            ..CacheTokenOptions::default()
        },
    );

    let vars_record = CssInJs::records()
        .into_iter()
        .find(|record| record.style_id == "ctx-vars-style")
        .expect("ctx vars record");
    assert_eq!(vars_record.layer.as_deref(), Some("antd"));
    assert_eq!(vars_record.nonce.as_deref(), Some("nonce-ant"));
}

#[test]
fn use_css_var_register_matches_ant_hash_and_css_var_shape() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset_runtime();

    let mut token = CssVarTokenMap::new();
    token.insert("colorPrimary".to_string(), "#1677ff".into());
    token.insert("borderRadiusSM".to_string(), 4.into());

    let first = use_css_var_register(
        &token,
        UseCssVarRegisterOptions {
            path: vec![Arc::<str>::from("button")],
            key: Arc::<str>::from("ant-button"),
            prefix: Some(Arc::<str>::from("ant")),
            scope: vec![Arc::<str>::from("scope-a")],
            hash_id: Some(Arc::<str>::from("hash-a")),
            ..UseCssVarRegisterOptions::default()
        },
    );
    let second = use_css_var_register(
        &token,
        UseCssVarRegisterOptions {
            path: vec![Arc::<str>::from("button")],
            key: Arc::<str>::from("ant-button"),
            prefix: Some(Arc::<str>::from("ant")),
            scope: vec![Arc::<str>::from("scope-a")],
            hash_id: Some(Arc::<str>::from("hash-a")),
            ..UseCssVarRegisterOptions::default()
        },
    );

    assert_eq!(first.style_id, second.style_id);
    assert!(first.css_vars_css.contains("--ant-color-primary"));
    assert!(first.css_vars_css.contains("--ant-border-radius-sm"));
    assert!(
        first
            .css_vars_css
            .contains(":where(.hash-a).ant-button.scope-a")
    );
    assert_eq!(
        first.merged_token.get("colorPrimary").map(String::as_str),
        Some("var(--ant-color-primary)")
    );
}

#[test]
fn get_computed_token_matches_ant_merge_order_and_format() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset_runtime();

    let theme = create_theme(vec![Arc::new(|token: &CssVarTokenMap| {
        let mut next = token.clone();
        next.insert("colorPrimary".to_string(), "#52c41a".into());
        next.insert("padding".to_string(), 16.into());
        next
    })]);

    let mut origin = CssVarTokenMap::new();
    origin.insert("colorPrimary".to_string(), "#1677ff".into());
    origin.insert("padding".to_string(), 12.into());

    let mut override_token = CssVarTokenMap::new();
    override_token.insert("colorPrimary".to_string(), "#ff4d4f".into());

    let computed = get_computed_token(
        &origin,
        &override_token,
        &theme,
        Some(Arc::new(|token: &CssVarTokenMap| {
            let mut next = token.clone();
            next.insert("formatted".to_string(), "yes".into());
            next
        })),
    );

    assert_eq!(
        computed.get("colorPrimary").map(|value| value.to_string()),
        Some(String::from("#ff4d4f"))
    );
    assert_eq!(
        computed.get("padding").map(|value| value.to_string()),
        Some(String::from("16"))
    );
    assert_eq!(
        computed.get("formatted").map(|value| value.to_string()),
        Some(String::from("yes"))
    );
}

#[test]
fn token_to_css_var_and_gen_calc_are_available_at_top_level() {
    let css_var = token_to_css_var("borderRadiusSM", Some("ant"));
    assert_eq!(css_var, "--ant-border-radius-sm");

    let calc = gen_calc(CalcMode::Js, Default::default());
    let value = calc(8.into()).add(4);
    assert_eq!(value.numeric(), Some(12.0));
    assert_eq!(value.css(), "calc(8 + 4)");

    let css_calc = gen_calc(CalcMode::Css, Default::default());
    let css_value = css_calc("100%".into()).sub("12px");
    assert_eq!(css_value.numeric(), None);
    assert_eq!(css_value.css(), "calc(100% - 12px)");
}

#[test]
fn auto_prefix_transformer_is_identity_and_logical_transformer_maps_properties() {
    let mut css = CSSObject::new();
    css.insert("marginBlock".to_string(), CSSInterpolation::from("4px 8px"));
    css.insert(
        "borderInlineStartWidth".to_string(),
        CSSInterpolation::Number(2.0),
    );

    let identity = auto_prefix_transformer();
    let logical = legacy_logical_properties_transformer();
    let identity_out = identity(css.clone());
    assert_eq!(identity_out, css);

    let transformed = logical(css);
    assert_eq!(
        transformed.get("marginTop"),
        Some(&CSSInterpolation::from("4px"))
    );
    assert_eq!(
        transformed.get("marginBottom"),
        Some(&CSSInterpolation::from("8px"))
    );
    assert_eq!(
        transformed.get("borderLeftWidth"),
        Some(&CSSInterpolation::Number(2.0))
    );
}

#[test]
fn px2rem_transformer_converts_values_and_media_queries() {
    let mut css = CSSObject::new();
    css.insert("width".to_string(), CSSInterpolation::Number(32.0));
    css.insert(
        "@media (min-width: 32px)".to_string(),
        CSSInterpolation::Object(CSSObject::new()),
    );

    let transform = px2rem_transformer(Px2RemOptions {
        media_query: true,
        ..Px2RemOptions::default()
    });
    let transformed = transform(css);

    assert_eq!(
        transformed.get("width"),
        Some(&CSSInterpolation::from("2rem"))
    );
    assert!(transformed.contains_key("@media (min-width: 2rem)"));
}

#[test]
fn linters_emit_expected_ant_messages() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset_runtime();

    let info = CssLintContext::new(
        Some(Arc::<str>::from("button")),
        None,
        vec![Arc::<str>::from(".ant-btn:not(a#hero.primary)")],
    );

    logical_properties_linter()("marginLeft", "12px", &info);
    legacy_not_selector_linter()("color", "red", &info);
    parent_selector_linter()(
        "color",
        "red",
        &CssLintContext::new(
            Some(Arc::<str>::from("button")),
            None,
            vec![Arc::<str>::from("&.one&.two")],
        ),
    );
    nan_linter()("width", "NaNpx", &info);

    let warnings = debug_take_lint_warnings_for_tests();
    assert_eq!(warnings.len(), 4);
    assert!(
        warnings
            .iter()
            .any(|warning| warning.contains("marginLeft"))
    );
    assert!(warnings.iter().any(|warning| warning.contains(":not")));
    assert!(
        warnings
            .iter()
            .any(|warning| warning.contains("more than one `&`"))
    );
    assert!(
        warnings
            .iter()
            .any(|warning| warning.contains("Unexpected 'NaN'"))
    );
}
