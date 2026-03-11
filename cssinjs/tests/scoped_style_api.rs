use cssinjs::{ScopedClassEntry, ScopedClassMap, ScopedStyle, ScopedStyleSpec};

#[test]
fn scoped_style_spec_exposes_class_lookup_for_rsx() {
    const CLASSES: &[ScopedClassEntry] = &[
        ScopedClassEntry::new("button", "sc_demo_button"),
        ScopedClassEntry::new("title", "sc_demo_title"),
    ];
    let spec = ScopedStyleSpec::new(
        "sc_demo",
        ".sc_demo_button{color:red;}.sc_demo_title{font-weight:600;}",
        "sc_demo",
        "sc_demo_button sc_demo_title",
        CLASSES,
    );
    let map = ScopedClassMap::from_spec(spec);

    assert_eq!(map.scope(), "sc_demo");
    assert_eq!(map.class("button"), Some("sc_demo_button"));
    assert_eq!(map.class("title"), Some("sc_demo_title"));
    assert_eq!(map.class("missing"), None);
}

#[test]
fn scoped_style_into_spec_keeps_scope_identity() {
    let style = ScopedStyle::new("sc_api_case", ".sc_api_case_box{opacity:.9;}");
    let spec: ScopedStyleSpec = style.into();
    assert_eq!(spec.style_id(), "sc_api_case");
    assert_eq!(spec.scope(), "sc_api_case");
}
