//! Proc-macro implementations for scoped styling + SCSS/ACSS.
//!
//! Exposes implementations used by the exported proc-macros in `lib.rs`:
//! - scoped_style!("path-or-inline")
//! - scoped_style!(module_name, "path")
//! - css!("declarations...")

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use std::path::PathBuf;
use syn::{
    Ident, LitStr, Token,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

use crate::compile::{StyleCompiler, StyleKind};
use crate::scope::{ScopeEngineKind, parse_and_scope, parse_and_scope_with_engine};
use cssinjs::compiler::hash::ScopeHasher;

enum ScopedStyleArgs {
    Single(LitStr),
    Named {
        module: Ident,
        _comma: Token![,],
        path: LitStr,
    },
}

impl Parse for ScopedStyleArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(LitStr) {
            Ok(Self::Single(input.parse()?))
        } else {
            Ok(Self::Named {
                module: input.parse()?,
                _comma: input.parse()?,
                path: input.parse()?,
            })
        }
    }
}

/// Implementation of `scoped_style!(...)`.
pub fn scoped_style_impl(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as ScopedStyleArgs);

    match args {
        ScopedStyleArgs::Single(s) => Expander::scoped_style_from_literal(None, s),
        ScopedStyleArgs::Named { module, path, .. } => {
            Expander::scoped_style_from_literal(Some(module), path)
        }
    }
}

/// Implementation of `css!("color:red;")`.
pub fn css_impl(input: TokenStream) -> TokenStream {
    let input_str = parse_macro_input!(input as LitStr);
    Expander::inline_css_utility(input_str)
}

struct Expander;

impl Expander {
    fn scoped_style_from_literal(module: Option<Ident>, lit: LitStr) -> TokenStream {
        let minify = cfg!(not(debug_assertions));

        let source = match SourceResolver::from_literal(&lit) {
            Ok(s) => s,
            Err(e) => return e.to_compile_error().into(),
        };

        let style_kind = StyleCompiler::detect(source.content.as_str(), source.file_abs.as_deref());
        let extra_load_paths = SourceResolver::default_extra_load_paths();

        let compiled_css = match StyleCompiler::compile(
            source.content.as_str(),
            source.file_abs.as_deref(),
            minify,
            &extra_load_paths,
        ) {
            Ok(css) => css,
            Err(e) => return e.to_compile_error().into(),
        };

        // Scope hash should include the *logical* path string (user input) to keep semantics stable.
        let scope = if source.is_file {
            ScopeHasher::generate(&compiled_css, Some(source.raw_for_hash.as_str()))
        } else {
            ScopeHasher::generate(&compiled_css, None)
        };

        let scope_engine = match style_kind {
            StyleKind::Css => ScopeEngineKind::LightningCss,
            StyleKind::Scss => ScopeEngineKind::Scss,
            StyleKind::Acss => ScopeEngineKind::Acss,
        };
        let scoped = parse_and_scope_with_engine(&compiled_css, &scope, minify, scope_engine);
        let scoped_css = scoped.scoped;
        let scoped_class_names = scoped.class_names;
        let scoped_class_entries = scoped_class_names
            .iter()
            .map(|class_name| {
                let scoped_class = format!("{scope}_{class_name}");
                quote! { ::cssinjs::ScopedClassEntry::new(#class_name, #scoped_class) }
            })
            .collect::<Vec<_>>();
        let scoped_class_joined = scoped_class_names
            .iter()
            .map(|class_name| format!("{scope}_{class_name}"))
            .collect::<Vec<_>>()
            .join(" ");

        let include_tracker_stmt = source
            .include_tracker
            .as_ref()
            .map(|t| t.tokens())
            .unwrap_or_else(|| quote! {});

        let expanded = if let Some(module_ident) = module {
            let class_consts = scoped_class_names
                .iter()
                .filter_map(|class_name| {
                    RustIdent::from_css_class(class_name).map(|ident| {
                        let scoped_class = format!("{scope}_{class_name}");
                        quote! { pub const #ident: &str = #scoped_class; }
                    })
                })
                .collect::<Vec<_>>();

            quote! {
                mod #module_ident {
                    #(#class_consts)*

                    pub fn get_scope() -> ::cssinjs::ScopedStyleSpec {
                        use ::std::sync::OnceLock;
                        static STYLE_INSTANCE: OnceLock<::cssinjs::ScopedStyle> = OnceLock::new();
                        const STYLE_CLASSES: &[::cssinjs::ScopedClassEntry] = &[#(#scoped_class_entries),*];

                        let _ = STYLE_INSTANCE.get_or_init(|| {
                            #include_tracker_stmt
                            ::cssinjs::ScopedStyle::new(#scope, #scoped_css)
                        });

                        ::cssinjs::ScopedStyleSpec::new(
                            #scope,
                            #scoped_css,
                            #scope,
                            #scoped_class_joined,
                            STYLE_CLASSES,
                        )
                    }
                }
                #module_ident::get_scope()
            }
        } else {
            quote! {
                {
                    use ::std::sync::OnceLock;
                    static STYLE_INSTANCE: OnceLock<::cssinjs::ScopedStyle> = OnceLock::new();
                    const STYLE_CLASSES: &[::cssinjs::ScopedClassEntry] = &[#(#scoped_class_entries),*];

                    let _ = STYLE_INSTANCE.get_or_init(|| {
                        #include_tracker_stmt
                        ::cssinjs::ScopedStyle::new(#scope, #scoped_css)
                    });

                    ::cssinjs::ScopedStyleSpec::new(
                        #scope,
                        #scoped_css,
                        #scope,
                        #scoped_class_joined,
                        STYLE_CLASSES,
                    )
                }
            }
        };

        TokenStream::from(expanded)
    }

    fn inline_css_utility(input_str: LitStr) -> TokenStream {
        let css_content = input_str.value();

        // This macro is for inline declarations.
        if SourceResolver::looks_like_file_path(&css_content) {
            return syn::Error::new(
                input_str.span(),
                "css!(...) expects inline CSS declarations, not a file path. Use scoped_style!(\"file.scss\") instead.",
            )
            .to_compile_error()
            .into();
        }

        let minify = cfg!(not(debug_assertions));

        let compiled = match StyleCompiler::compile(&css_content, None, minify, &[]) {
            Ok(v) => v,
            Err(e) => return e.to_compile_error().into(),
        };

        let scope = ScopeHasher::generate(&compiled, None);

        let class_name = format!("{scope}_inline");
        let wrapped_css = format!(".{class_name} {{ {compiled} }}");

        let scoped = parse_and_scope(&wrapped_css, &scope, minify);
        let final_css = scoped.scoped;
        let class_entries = scoped
            .class_names
            .iter()
            .map(|class_name| {
                let scoped_class = format!("{scope}_{class_name}");
                quote! { ::cssinjs::ScopedClassEntry::new(#class_name, #scoped_class) }
            })
            .collect::<Vec<_>>();
        let classes_joined = scoped
            .class_names
            .iter()
            .map(|class_name| format!("{scope}_{class_name}"))
            .collect::<Vec<_>>()
            .join(" ");

        TokenStream::from(quote! {
            {
                use ::std::sync::OnceLock;
                static STYLE_INSTANCE: OnceLock<::cssinjs::ScopedStyle> = OnceLock::new();
                const STYLE_CLASSES: &[::cssinjs::ScopedClassEntry] = &[#(#class_entries),*];

                let _ = STYLE_INSTANCE.get_or_init(|| {
                    ::cssinjs::ScopedStyle::new(#scope, #final_css)
                });

                ::cssinjs::ScopedStyleSpec::new(
                    #scope,
                    #final_css,
                    #scope,
                    #classes_joined,
                    STYLE_CLASSES,
                )
            }
        })
    }
}

// ============================================================================
// Source resolution
// ============================================================================

struct ResolvedSource {
    is_file: bool,
    raw_for_hash: String,
    file_abs: Option<String>,
    content: String,
    include_tracker: Option<IncludeTracker>,
}

impl ResolvedSource {
    fn inline(raw: String) -> Self {
        Self {
            is_file: false,
            raw_for_hash: String::new(),
            file_abs: None,
            content: raw,
            include_tracker: None,
        }
    }
}

struct IncludeTracker {
    lit: LitStr,
    resolved_abs: String,
}

impl IncludeTracker {
    fn tokens(&self) -> proc_macro2::TokenStream {
        StyleCompiler::include_tracker_tokens(&self.lit, &self.resolved_abs)
    }
}

struct SourceResolver;

impl SourceResolver {
    fn from_literal(lit: &LitStr) -> syn::Result<ResolvedSource> {
        let raw = lit.value();
        let raw_trim = raw.trim();

        if !Self::looks_like_file_path(raw_trim) {
            return Ok(ResolvedSource::inline(raw));
        }

        let (abs, content) = Self::resolve_stylesheet_path(lit)?;
        let include_tracker = abs.clone().map(|p| IncludeTracker {
            lit: lit.clone(),
            resolved_abs: p,
        });

        Ok(ResolvedSource {
            is_file: true,
            raw_for_hash: raw,
            file_abs: abs,
            content,
            include_tracker,
        })
    }

    #[inline]
    fn looks_like_file_path(s: &str) -> bool {
        let t = s.trim();
        if t.is_empty() {
            return false;
        }

        let lower = t.to_ascii_lowercase();
        if lower.ends_with(".css")
            || lower.ends_with(".scss")
            || lower.ends_with(".sass")
            || lower.ends_with(".acss")
        {
            return true;
        }

        let has_sep = t.contains('/') || t.contains('\\');
        if !has_sep {
            return false;
        }

        if t.contains('{')
            || t.contains('}')
            || t.contains(';')
            || t.contains('\n')
            || t.contains('\r')
        {
            return false;
        }

        true
    }

    fn resolve_stylesheet_path(lit: &LitStr) -> syn::Result<(Option<String>, String)> {
        let raw = lit.value();
        let requested = PathBuf::from(raw.trim());

        let mut tried: Vec<PathBuf> = Vec::with_capacity(8);

        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").ok().map(PathBuf::from);
        let current_dir = std::env::current_dir().ok();

        let mut bases: Vec<PathBuf> = Vec::new();
        if let Some(m) = manifest_dir.clone() {
            bases.push(m.clone());
            bases.push(m.join("src"));
            if let Some(parent) = m.parent() {
                bases.push(parent.to_path_buf());
            }
        }
        if let Some(cwd) = current_dir {
            bases.push(cwd);
        }

        let candidates = if requested.is_absolute() {
            vec![requested.clone()]
        } else {
            bases.into_iter().map(|b| b.join(&requested)).collect()
        };

        for cand in candidates {
            tried.push(cand.clone());
            if let Ok(content) = std::fs::read_to_string(&cand) {
                let abs = cand.canonicalize().unwrap_or(cand);
                return Ok((Some(abs.to_string_lossy().to_string()), content));
            }
        }

        let mut msg = String::new();
        msg.push_str("Failed to find CSS/SCSS file.\n");
        msg.push_str("Requested: ");
        msg.push_str(raw.as_str());
        msg.push('\n');
        msg.push_str("Tried:\n");
        for p in tried {
            msg.push_str("  - ");
            msg.push_str(p.to_string_lossy().as_ref());
            msg.push('\n');
        }

        Err(syn::Error::new(lit.span(), msg))
    }

    fn default_extra_load_paths() -> Vec<PathBuf> {
        let mut extra: Vec<PathBuf> = Vec::new();
        if let Ok(m) = std::env::var("CARGO_MANIFEST_DIR") {
            let m = PathBuf::from(m);
            extra.push(m.join("assets"));
            extra.push(m.join("styles"));
            extra.push(m.join("scss"));
        }
        extra
    }
}

// ============================================================================
// Rust identifier normalization
// ============================================================================

struct RustIdent;

impl RustIdent {
    fn from_css_class(class_name: &str) -> Option<Ident> {
        let mut normalized = String::with_capacity(class_name.len() + 4);
        for (idx, ch) in class_name.chars().enumerate() {
            let mapped = if ch.is_ascii_alphanumeric() || ch == '_' {
                ch.to_ascii_lowercase()
            } else {
                '_'
            };
            if idx == 0 && mapped.is_ascii_digit() {
                normalized.push('_');
            }
            normalized.push(mapped);
        }

        let normalized = normalized.trim_end_matches('_');
        if normalized.is_empty() {
            return None;
        }

        let ident_str = if Self::is_rust_keyword(normalized) {
            format!("r#{normalized}")
        } else {
            normalized.to_string()
        };
        Some(format_ident!("{ident_str}"))
    }

    fn is_rust_keyword(name: &str) -> bool {
        matches!(
            name,
            "as" | "break"
                | "const"
                | "continue"
                | "crate"
                | "else"
                | "enum"
                | "extern"
                | "false"
                | "fn"
                | "for"
                | "if"
                | "impl"
                | "in"
                | "let"
                | "loop"
                | "match"
                | "mod"
                | "move"
                | "mut"
                | "pub"
                | "ref"
                | "return"
                | "self"
                | "Self"
                | "static"
                | "struct"
                | "super"
                | "trait"
                | "true"
                | "type"
                | "unsafe"
                | "use"
                | "where"
                | "while"
                | "async"
                | "await"
                | "dyn"
                | "abstract"
                | "become"
                | "box"
                | "do"
                | "final"
                | "macro"
                | "override"
                | "priv"
                | "typeof"
                | "unsized"
                | "virtual"
                | "yield"
                | "try"
        )
    }
}
