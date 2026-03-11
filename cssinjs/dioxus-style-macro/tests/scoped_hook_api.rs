use cssinjs::ScopedStyleSpec;
use dioxus_style_macro::{acss, scoped_style};

#[test]
fn scoped_style_macro_returns_scoped_style_spec() {
    let spec: ScopedStyleSpec =
        scoped_style!(".button { color: red; } .title { font-weight: 600; }");
    assert!(!spec.scope().is_empty());
    assert!(spec.class("button").is_some());
    assert!(spec.class("title").is_some());
}

#[test]
fn acss_macro_returns_spec_with_class_map() {
    let spec: ScopedStyleSpec = acss!("color:red", "font-weight:700");
    assert!(!spec.classes_joined().is_empty());
    let first = spec
        .classes()
        .first()
        .map(|entry| entry.key)
        .unwrap_or_default();
    assert!(!first.is_empty());
    assert_eq!(spec.class(first), Some(first));
}
