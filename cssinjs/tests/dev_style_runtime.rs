use cssinjs::CssInJsRuntime;

#[test]
fn keyed_dev_style_upsert_and_remove() {
    let runtime = CssInJsRuntime;
    runtime.clear();
    runtime.clear_dev_styles();

    assert!(runtime.list_dev_styles().is_empty());

    let first = runtime.upsert_dev_style("dashboard", "title", ".title{color:red;}");
    assert!(first.is_some());

    let first = match first {
        Some(value) => value,
        None => panic!("first registration should exist"),
    };
    assert_eq!(first.key, "dashboard::title");
    assert!(first.class_name.starts_with("_R_"));
    assert!(first.class_name.ends_with('_'));
    let body = &first.class_name[3..first.class_name.len() - 1];
    assert_eq!(body.len(), 6);
    assert!(body.as_bytes().iter().all(|b| b.is_ascii_alphabetic()));

    let second = runtime.upsert_dev_style("dashboard", "title", ".title{color:blue;}");
    assert!(second.is_some());

    let second = match second {
        Some(value) => value,
        None => panic!("second registration should exist"),
    };
    assert_eq!(second.key, "dashboard::title");
    assert_eq!(second.class_name, first.class_name);

    let all = runtime.list_dev_styles();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].key, "dashboard::title");

    assert!(runtime.remove_dev_style("dashboard", "title"));
    assert!(runtime.list_dev_styles().is_empty());
}
