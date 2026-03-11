#![forbid(unsafe_code)]

pub mod acss;
pub mod hash;
pub mod scss;

pub use acss::{AcssCompileOutput, AcssCompiler};
use lightningcss::stylesheet::{ParserOptions, PrinterOptions, StyleSheet};
pub use scss::ScssCompiler;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StyleKind {
    Css,
    Scss,
    Acss,
}

pub struct StyleCompiler;

impl StyleCompiler {
    /// Detect kind using file extension and inline markers.
    #[inline]
    pub fn detect(content: &str, file_path: Option<&str>) -> StyleKind {
        if file_path.is_some_and(AcssCompiler::is_file) || AcssCompiler::is_inline_marker(content) {
            return StyleKind::Acss;
        }

        if file_path.is_some_and(ScssCompiler::is_file) || Self::looks_like_inline_scss(content) {
            return StyleKind::Scss;
        }

        StyleKind::Css
    }

    /// Canonical compile contract shared by css/scss/acss front-ends.
    ///
    /// All style kinds pass through the same post-compile normalization stage.
    pub fn compile(
        content: &str,
        file_path: Option<&str>,
        minify: bool,
        extra_load_paths: &[PathBuf],
    ) -> Result<String, String> {
        let parser_filename = file_path.unwrap_or("inline.css");
        match Self::detect(content, file_path) {
            StyleKind::Css => Ok(Self::normalize_css(content, minify, parser_filename)),
            StyleKind::Acss => AcssCompiler::compile(content, minify)
                .map(|css| Self::normalize_css(css.as_str(), minify, parser_filename)),
            StyleKind::Scss => {
                let fp = file_path.map(PathBuf::from);
                ScssCompiler::compile(
                    content,
                    fp.as_deref().map(Path::new),
                    minify,
                    extra_load_paths,
                )
                .map(|css| Self::normalize_css(css.as_str(), minify, parser_filename))
            }
        }
    }

    /// Shared normalization stage for css/scss/acss after front-end compilation.
    #[inline]
    pub fn normalize_css(content: &str, minify: bool, parser_filename: &str) -> String {
        let opts = ParserOptions {
            error_recovery: true,
            filename: parser_filename.into(),
            ..ParserOptions::default()
        };

        let Ok(sheet) = StyleSheet::parse(content, opts) else {
            return content.to_string();
        };

        let po = PrinterOptions {
            minify,
            ..PrinterOptions::default()
        };

        match sheet.to_css(po) {
            Ok(out) => out.code,
            Err(_) => content.to_string(),
        }
    }

    fn looks_like_inline_scss(content: &str) -> bool {
        let c = content;
        c.contains('$')
            || c.contains("@mixin")
            || c.contains("@include")
            || c.contains("@use")
            || c.contains("@forward")
            || c.contains("@extend")
            || c.contains('&')
            || Self::has_nesting(c)
    }

    fn has_nesting(content: &str) -> bool {
        let mut depth: u32 = 0;
        let mut chars = content.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                '{' => {
                    depth = depth.saturating_add(1);
                    if depth > 1 {
                        return true;
                    }
                }
                '}' => depth = depth.saturating_sub(1),
                '"' | '\'' => {
                    let quote = ch;
                    while let Some(c) = chars.next() {
                        if c == quote {
                            break;
                        }
                        if c == '\\' {
                            let _ = chars.next();
                        }
                    }
                }
                '/' => {
                    if chars.peek() == Some(&'*') {
                        let _ = chars.next();
                        while let Some(c) = chars.next() {
                            if c == '*' && chars.peek() == Some(&'/') {
                                let _ = chars.next();
                                break;
                            }
                        }
                    } else if chars.peek() == Some(&'/') {
                        for c in chars.by_ref() {
                            if c == '\n' {
                                break;
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        false
    }
}
