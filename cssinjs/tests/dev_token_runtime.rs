use cssinjs::{CssInJsRuntime, CssVarTokenMap};

#[test]
fn scoped_dev_tokens_upsert_update_and_remove() {
    let runtime = CssInJsRuntime;
    runtime.clear();
    runtime.clear_dev_tokens();

    assert!(runtime.list_dev_tokens().is_empty());

    let mut light = CssVarTokenMap::new();
    light.insert("colorPrimary".to_string(), "#1677ff".into());
    light.insert("borderRadius".to_string(), 8.into());

    let first = runtime.upsert_dev_tokens("dashboard", "theme", &light);
    assert!(first.is_some());
    let first = match first {
        Some(value) => value,
        None => panic!("first token registration should exist"),
    };

    assert_eq!(first.key, "dashboard::theme");
    assert!(!first.hash_class.is_empty());
    assert!(!first.style_id.is_empty());
    assert!(!first.css_var_key.is_empty());
    assert!(
        runtime
            .css()
            .contains("--dev-dashboard-color-primary:#1677ff;")
    );

    let mut dark = CssVarTokenMap::new();
    dark.insert("colorPrimary".to_string(), "#111111".into());
    dark.insert("borderRadius".to_string(), 6.into());

    let second = runtime.upsert_dev_tokens("dashboard", "theme", &dark);
    assert!(second.is_some());
    let second = match second {
        Some(value) => value,
        None => panic!("second token registration should exist"),
    };

    assert_eq!(second.key, "dashboard::theme");
    assert_eq!(second.hash_class, first.hash_class);

    let css = runtime.css();
    assert!(css.contains("--dev-dashboard-color-primary:#111111;"));
    assert!(!css.contains("--dev-dashboard-color-primary:#1677ff;"));

    let all = runtime.list_dev_tokens();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].key, "dashboard::theme");

    assert!(runtime.remove_dev_tokens("dashboard", "theme"));
    assert!(runtime.list_dev_tokens().is_empty());
}
