use std::fs;
use std::path::PathBuf;

#[test]
fn strict_mode_scoped_transform_has_no_direct_style_injection_by_default() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(manifest_dir.join("src/lib.rs"))
        .expect("failed to read macro crate src/lib.rs");

    assert!(source.contains("if cfg!(feature = \"legacy-inline-style-fallback\")"));
    assert!(source.contains("let _ = css;"));
    assert!(source.contains("inject_style(css)"));
}
