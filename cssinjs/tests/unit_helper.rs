use cssinjs::unit;

#[test]
fn unit_converts_numeric_to_px_and_keeps_text() {
    assert_eq!(unit(1), "1px");
    assert_eq!(unit(1.5), "1.5px");
    assert_eq!(unit("100%"), "100%");
    assert_eq!(unit("var(--menu-line-height)"), "var(--menu-line-height)");
}
