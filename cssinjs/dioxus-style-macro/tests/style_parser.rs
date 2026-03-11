#[path = "../src/scope/parser.rs"]
mod parser;

use parser::parse_and_scope;

#[test]
fn class_selector_scoping() {
    let css = ".button { color: red; }";
    let scoped = parse_and_scope(css, "sc_abc", false);
    assert!(scoped.scoped.contains(".sc_abc_button"));
    assert!(scoped.class_names.contains(&"button".to_string()));
}

#[test]
fn id_selector_scoping() {
    let css = "#header { color: blue; }";
    let scoped = parse_and_scope(css, "sc_abc", false);
    assert!(scoped.scoped.contains("#sc_abc_header"));
}

#[test]
fn element_selector_passthrough_in_css_modules_mode() {
    let css = "div { margin: 10px; }";
    let scoped = parse_and_scope(css, "sc_abc", false);
    assert!(scoped.scoped.contains("div"));
    assert!(!scoped.scoped.contains("data-scope=\"sc_abc\""));
}

#[test]
fn complex_selector_mixed() {
    let css = "div.container > .item + #special { color: green; }";
    let scoped = parse_and_scope(css, "sc_xyz", false);
    assert!(scoped.scoped.contains(".sc_xyz_container"));
    assert!(scoped.scoped.contains(".sc_xyz_item"));
    assert!(scoped.scoped.contains("#sc_xyz_special"));
}

#[test]
fn multiple_selectors_top_level_commas_only() {
    let css = ".btn, .button:not(.x,.y), #submit { color: red; }";
    let scoped = parse_and_scope(css, "sc_test", false);
    assert!(scoped.scoped.contains(".sc_test_btn"));
    assert!(
        scoped
            .scoped
            .contains(".sc_test_button:not(.sc_test_x,.sc_test_y)")
            || scoped
                .scoped
                .contains(".sc_test_button:not(.sc_test_x, .sc_test_y)")
    );
    assert!(scoped.scoped.contains("#sc_test_submit"));
}

#[test]
fn pseudo_classes() {
    let css = ".button:hover { background: blue; }";
    let scoped = parse_and_scope(css, "sc_abc", false);
    assert!(scoped.scoped.contains(".sc_abc_button:hover"));
}

#[test]
fn not_pseudo_class_scoping() {
    let css = ".button:not(.disabled) { color: red; }";
    let scoped = parse_and_scope(css, "sc_test", false);
    assert!(
        scoped
            .scoped
            .contains(".sc_test_button:not(.sc_test_disabled)")
    );
}

#[test]
fn has_is_where_pseudo_class_scoping() {
    let css = ".container:has(.child) { display: flex; }";
    let scoped = parse_and_scope(css, "sc_test", false);
    assert!(
        scoped
            .scoped
            .contains(".sc_test_container:has(.sc_test_child)")
    );
}

#[test]
fn nth_child_of_scoping() {
    let css = "li:nth-child(2n+1 of .hot, .cold) { color: red; }";
    let scoped = parse_and_scope(css, "sc_test", false);
    assert!(
        scoped
            .scoped
            .contains("li:nth-child(odd of .sc_test_hot,.sc_test_cold)")
            || scoped
                .scoped
                .contains("li:nth-child(odd of .sc_test_hot, .sc_test_cold)")
            || scoped
                .scoped
                .contains("li:nth-child(2n+1 of .sc_test_hot,.sc_test_cold)")
            || scoped
                .scoped
                .contains("li:nth-child(2n+1 of .sc_test_hot, .sc_test_cold)")
    );
}

#[test]
fn attribute_selectors() {
    let css = "input[type=\"text\"] { border: 1px solid; }";
    let scoped = parse_and_scope(css, "sc_test", false);
    assert!(scoped.scoped.contains("input[type=\"text\"]"));
}

#[test]
fn attribute_selector_with_bracket_inside_quotes() {
    let css = r#"input[placeholder="Enter ]value"] { border: 1px solid; }"#;
    let scoped = parse_and_scope(css, "sc_test", false);
    assert!(scoped.scoped.contains(r#"[placeholder="Enter ]value"]"#));
}

#[test]
fn at_rule_media_scopes_children() {
    let css = "@media (min-width: 600px) { .btn { color: red; } }";
    let scoped = parse_and_scope(css, "sc_test", false);
    assert!(scoped.scoped.contains("@media"));
    assert!(scoped.scoped.contains(".sc_test_btn"));
}

#[test]
fn keyframes_and_animation_name_scoped() {
    let css = "@keyframes spin { from { transform: rotate(0); } to { transform: rotate(360deg); } } .box { animation: spin 1s linear; }";
    let scoped = parse_and_scope(css, "sc_test", false);
    assert!(scoped.scoped.contains("@keyframes sc_test_spin"));
    assert!(scoped.scoped.contains("sc_test_spin"));
}

#[test]
fn parser_handles_braces_in_strings_and_url() {
    let css = r#"
            .a { content: "}"; }
            .b { background: url("data:image/svg+xml;utf8,<svg>{</svg>"); color: red; }
        "#;
    let scoped = parse_and_scope(css, "sc_test", false);
    assert!(scoped.scoped.contains(".sc_test_a"));
    assert!(scoped.scoped.contains(".sc_test_b"));
}

#[test]
fn minify_removes_comments_and_collapses_whitespace() {
    let css = r#"
            /* Comment */
            .button {
                color: red; /* inline */
            }
        "#;
    let scoped = parse_and_scope(css, "sc_test", true);
    assert!(!scoped.scoped.contains("/*"));
    assert!(!scoped.scoped.contains("*/"));
}

#[test]
fn empty_css() {
    let scoped = parse_and_scope("", "sc_test", false);
    assert!(scoped.scoped.trim().is_empty());
    assert!(scoped.class_names.is_empty());
}

#[test]
fn malformed_css_fallback_preserved() {
    let css = ". { color: red; }";
    let scoped = parse_and_scope(css, "sc_test", false);
    assert!(!scoped.scoped.is_empty());
}
