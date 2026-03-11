use std::path::PathBuf;

use cssinjs::compiler::{AcssCompiler, StyleCompiler, StyleKind};

#[test]
fn detect_uses_frontend_markers() {
    assert_eq!(
        StyleCompiler::detect("@acss color:red;", None),
        StyleKind::Acss
    );
    assert_eq!(
        StyleCompiler::detect(".a { color: red; }", Some("x.acss")),
        StyleKind::Acss
    );
    assert_eq!(
        StyleCompiler::detect("$c:red;.a{color:$c;}", Some("x.scss")),
        StyleKind::Scss
    );
    assert_eq!(
        StyleCompiler::detect(".a { color: red; }", Some("x.css")),
        StyleKind::Css
    );
}

#[test]
fn css_compile_uses_shared_normalization_stage() {
    let source = ".box { color: red ; }";
    let empty: Vec<PathBuf> = Vec::new();
    let compiled = StyleCompiler::compile(source, Some("inline.css"), false, &empty)
        .expect("css compile should succeed");
    let normalized = StyleCompiler::normalize_css(source, false, "inline.css");

    assert_eq!(compiled, normalized);
}

#[test]
fn acss_compile_uses_shared_normalization_stage() {
    let source = "@acss color:red; hover:color:blue;";
    let empty: Vec<PathBuf> = Vec::new();
    let compiled =
        StyleCompiler::compile(source, None, false, &empty).expect("acss compile should succeed");
    let acss_css = AcssCompiler::compile(source, false).expect("acss front-end should succeed");
    let normalized = StyleCompiler::normalize_css(acss_css.as_str(), false, "inline.css");

    assert_eq!(compiled, normalized);
}

#[cfg(feature = "scss")]
#[test]
fn scss_compile_uses_shared_normalization_stage() {
    use std::path::Path;

    use cssinjs::compiler::ScssCompiler;

    let source = "$c: red; .box { color: $c; }";
    let empty: Vec<PathBuf> = Vec::new();
    let compiled = StyleCompiler::compile(source, Some("inline.scss"), false, &empty)
        .expect("scss compile should succeed");
    let scss_css = ScssCompiler::compile(source, Some(Path::new("inline.scss")), false, &empty)
        .expect("scss front-end should succeed");
    let normalized = StyleCompiler::normalize_css(scss_css.as_str(), false, "inline.scss");

    assert_eq!(compiled, normalized);
}
