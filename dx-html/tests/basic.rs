use std::collections::{HashMap, HashSet};

use dx_html::{
    escape::{
        escape_attr, escape_text, push_optional_attr, write_bool_attr, write_data_attr,
        EscapedAttr, EscapedText,
    },
    sanitize::{
        sanitize_document, sanitize_html, sanitize_with, sanitize_with_preset, sanitizer,
        sanitizer_with_preset, SanitizerPreset,
    },
    script::{
        make_html_safe_script_json, push_js_assignment, push_json_script_tag,
        to_js_string_literal,
    },
};

#[test]
fn escapes_text_and_attributes() {
    assert_eq!(escape_text("<b>&\"</b>").as_ref(), "&lt;b&gt;&amp;\"&lt;/b&gt;");
    assert_eq!(escape_attr("hello \"world\" & more").as_ref(), "hello &quot;world&quot; &amp; more");
    assert_eq!(format!("{}", EscapedText("<ok>")), "&lt;ok&gt;");
    assert_eq!(format!("{}", EscapedAttr("\"id\"")), "&quot;id&quot;");
}

#[test]
fn writes_optional_boolean_and_data_attributes() {
    let mut out = String::new();
    push_optional_attr(&mut out, "class", Some(" app-shell "));
    write_bool_attr(&mut out, "hidden", true).unwrap();
    write_data_attr(&mut out, "theme", Some("dark")).unwrap();
    assert_eq!(out, " class=\"app-shell\" hidden data-theme=\"dark\"");
}

#[test]
fn sanitizes_untrusted_html() {
    let input = r#"<div><script>alert(1)</script><a href=\"javascript:alert(2)\">x</a><p>ok</p></div>"#;
    let output = sanitize_html(input);
    assert!(output.contains("<p>ok</p>"));
    assert!(!output.contains("<script>"));
    assert!(!output.contains("javascript:alert"));
}

#[test]
fn produces_document_without_buffering_contract_change() {
    let doc = sanitize_document("<b>x</b><script>y</script>");
    let output = doc.to_string();
    assert_eq!(output, "<b>x</b>");
}

#[test]
fn supports_custom_sanitizer_policy() {
    let mut builder = sanitizer();
    builder.add_tag_attributes("code", &["class"]);
    let html = r#"<code class=\"language-rust\">let x = 1;</code>"#;
    let output = sanitize_with(html, &builder);
    assert!(output.contains("class=\"language-rust\""));
}

#[test]
fn supports_rich_text_preset() {
    let html = r#"<pre class=\"language-rust\"><code class=\"language-rust\">let x = 1;</code></pre>"#;
    let output = sanitize_with_preset(html, SanitizerPreset::RichText).into_string();
    assert!(output.contains("language-rust"));
}

#[test]
fn supports_rich_text_preset_with_allowed_classes() {
    let mut classes: HashMap<&'static str, HashSet<&'static str>> = HashMap::new();
    classes.insert("code", HashSet::from(["language-rust"]));
    let builder = sanitizer_with_preset(SanitizerPreset::RichTextWithClasses {
        allowed_classes: classes,
    });
    let html = r#"<code class=\"language-rust language-ts\">let x = 1;</code>"#;
    let output = sanitize_with(html, &builder);
    assert!(output.contains("language-rust"));
}

#[test]
fn serializes_js_string_literal_for_html_script_context() {
    let value = to_js_string_literal("a\"b</script><c>").expect("string literal should serialize");
    assert!(value.contains("\\u003C/script\\u003E"));
}

#[test]
fn makes_json_html_safe() {
    let value = make_html_safe_script_json(r#"{"x":"</script>&"}"#);
    assert_eq!(value, r#"{"x":"\u003C/script\u003E\u0026"}"#);
}

#[test]
fn pushes_js_assignment() {
    let mut out = String::new();
    push_js_assignment(&mut out, "window.__DATA__", &serde_json::json!({"ok": true}))
        .expect("assignment should serialize");
    assert_eq!(out, r#"window.__DATA__ = {"ok":true};"#);
}

#[test]
fn pushes_json_script_tag() {
    let mut out = String::new();
    push_json_script_tag(&mut out, Some("boot-data"), &serde_json::json!({"ok": true}))
        .expect("script tag should serialize");
    assert_eq!(
        out,
        r#"<script id="boot-data" type="application/json">{"ok":true}</script>"#
    );
}
