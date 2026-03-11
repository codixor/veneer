//! Macro compile bridge to canonical `cssinjs::compiler` contract.

use std::path::{Path, PathBuf};

use syn::LitStr;

pub type StyleKind = cssinjs::compiler::StyleKind;

pub struct StyleCompiler;

impl StyleCompiler {
    #[inline]
    pub fn detect(content: &str, file_path: Option<&str>) -> StyleKind {
        cssinjs::compiler::StyleCompiler::detect(content, file_path)
    }

    #[inline]
    pub fn compile(
        content: &str,
        file_path: Option<&str>,
        minify: bool,
        extra_load_paths: &[PathBuf],
    ) -> syn::Result<String> {
        cssinjs::compiler::StyleCompiler::compile(content, file_path, minify, extra_load_paths)
            .map_err(|e| syn::Error::new(proc_macro2::Span::call_site(), e))
    }

    #[inline]
    pub fn normalize_css(content: &str, minify: bool, parser_filename: &str) -> String {
        cssinjs::compiler::StyleCompiler::normalize_css(content, minify, parser_filename)
    }

    /// Emit an include tracker token that is portable.
    ///
    /// - For relative paths: `include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", "path"))`
    /// - For absolute paths: `include_str!("/abs/path")`
    pub fn include_tracker_tokens(lit: &LitStr, resolved_abs: &str) -> proc_macro2::TokenStream {
        let raw = lit.value();
        let req = raw.trim();
        if Path::new(req).is_absolute() {
            let abs = LitStr::new(req, lit.span());
            quote::quote! { let _tracker = include_str!(#abs); }
        } else {
            // Use the user literal string (relative), not the canonicalized abs.
            // This makes the expansion portable across machines.
            let rel = LitStr::new(req, lit.span());
            let _ = resolved_abs; // kept for potential debugging; do not embed.
            quote::quote! { let _tracker = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", #rel)); }
        }
    }
}
