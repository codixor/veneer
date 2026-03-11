use std::sync::{Arc, Mutex};

use cssinjs::{
    CssInJs, CssParseCfg, StyleRegisterInput, UseStyleRegisterOptions, use_style_register,
};

static TEST_LOCK: Mutex<()> = Mutex::new(());

fn reset_runtime() {
    CssInJs::debug_reset_runtime_for_tests();
}

#[test]
fn use_style_register_explicit_order_controls_css_and_record_order() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset_runtime();

    let _ = use_style_register(UseStyleRegisterOptions {
        style_id: Arc::<str>::from("ordered-last"),
        css: Arc::<str>::from(".ordered-last{color:#333333;}"),
        identity_scope: Some(Arc::<str>::from("style|ordered-last")),
        hash_class: Some(Arc::<str>::from("ordered-last-hash")),
        order: 300,
        ..UseStyleRegisterOptions::default()
    })
    .expect("register last");

    let _ = use_style_register(UseStyleRegisterOptions {
        style_id: Arc::<str>::from("ordered-first"),
        css: Arc::<str>::from(".ordered-first{color:#111111;}"),
        identity_scope: Some(Arc::<str>::from("style|ordered-first")),
        hash_class: Some(Arc::<str>::from("ordered-first-hash")),
        order: 100,
        ..UseStyleRegisterOptions::default()
    })
    .expect("register first");

    let _ = use_style_register(UseStyleRegisterOptions {
        style_id: Arc::<str>::from("ordered-middle"),
        css: Arc::<str>::from(".ordered-middle{color:#222222;}"),
        identity_scope: Some(Arc::<str>::from("style|ordered-middle")),
        hash_class: Some(Arc::<str>::from("ordered-middle-hash")),
        order: 200,
        ..UseStyleRegisterOptions::default()
    })
    .expect("register middle");

    let css = CssInJs::css_arc().to_string();
    let first_pos = css.find("#111111").expect("first css");
    let middle_pos = css.find("#222222").expect("middle css");
    let last_pos = css.find("#333333").expect("last css");
    assert!(first_pos < middle_pos && middle_pos < last_pos);

    let records = CssInJs::records();
    assert_eq!(
        records
            .iter()
            .map(|record| (record.style_id.as_str(), record.order))
            .collect::<Vec<_>>(),
        vec![
            ("ordered-first", 100),
            ("ordered-middle", 200),
            ("ordered-last", 300),
        ]
    );
}

#[test]
fn use_style_register_order_update_reorders_without_duplicate_or_stale_css() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset_runtime();

    let _ = use_style_register(UseStyleRegisterOptions {
        style_id: Arc::<str>::from("order-a"),
        css: Arc::<str>::from(".order-a{color:#aaaaaa;}"),
        identity_scope: Some(Arc::<str>::from("style|order-a")),
        hash_class: Some(Arc::<str>::from("order-a-hash")),
        order: 200,
        ..UseStyleRegisterOptions::default()
    })
    .expect("register a");

    let _ = use_style_register(UseStyleRegisterOptions {
        style_id: Arc::<str>::from("order-b"),
        css: Arc::<str>::from(".order-b{color:#bbbbbb;}"),
        identity_scope: Some(Arc::<str>::from("style|order-b")),
        hash_class: Some(Arc::<str>::from("order-b-hash")),
        order: 100,
        ..UseStyleRegisterOptions::default()
    })
    .expect("register b");

    let _ = use_style_register(UseStyleRegisterOptions {
        style_id: Arc::<str>::from("order-a"),
        css: Arc::<str>::from(".order-a{color:#cccccc;}"),
        identity_scope: Some(Arc::<str>::from("style|order-a")),
        hash_class: Some(Arc::<str>::from("order-a-hash")),
        order: 50,
        ..UseStyleRegisterOptions::default()
    })
    .expect("update a");

    let css = CssInJs::css_arc().to_string();
    let updated_a = css.find("#cccccc").expect("updated a css");
    let b = css.find("#bbbbbb").expect("b css");
    assert!(updated_a < b);
    assert!(!css.contains("#aaaaaa"));

    let records = CssInJs::records();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].style_id, "order-a");
    assert_eq!(records[0].order, 50);
    assert_eq!(records[1].style_id, "order-b");
    assert_eq!(records[1].order, 100);
}

#[test]
fn style_register_input_order_controls_object_registration_order() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset_runtime();

    let first = StyleRegisterInput {
        path: vec![Arc::<str>::from("button"), Arc::<str>::from("late")],
        style_id: Some(Arc::<str>::from("object-late")),
        order: 300,
        ..StyleRegisterInput::default()
    };
    let second = StyleRegisterInput {
        path: vec![Arc::<str>::from("button"), Arc::<str>::from("early")],
        style_id: Some(Arc::<str>::from("object-early")),
        order: 100,
        ..StyleRegisterInput::default()
    };

    let _ = CssInJs::register_style_with_path(&first, ".object-late{border-color:#333333;}")
        .expect("register late");
    let _ = CssInJs::register_style_with_path(&second, ".object-early{border-color:#111111;}")
        .expect("register early");

    let css = CssInJs::css_arc().to_string();
    let early_pos = css.find("#111111").expect("early css");
    let late_pos = css.find("#333333").expect("late css");
    assert!(early_pos < late_pos);

    let records = CssInJs::records();
    assert_eq!(records[0].style_id, "object-early");
    assert_eq!(records[0].order, 100);
    assert_eq!(records[1].style_id, "object-late");
    assert_eq!(records[1].order, 300);

    let parsed = CssInJs::register_style_object_with_path(
        &StyleRegisterInput {
            path: vec![Arc::<str>::from("layered"), Arc::<str>::from("dep")],
            style_id: Some(Arc::<str>::from("layered-style")),
            order: 250,
            ..StyleRegisterInput::default()
        },
        &CssParseCfg::default(),
        {
            let mut inner = cssinjs::CSSObject::new();
            inner.insert("display".to_string(), "block".into());

            let mut outer = cssinjs::CSSObject::new();
            outer.insert(".layered-style".to_string(), inner.into());
            outer.into()
        },
        None,
    );
    assert!(parsed.main.is_some());
}
