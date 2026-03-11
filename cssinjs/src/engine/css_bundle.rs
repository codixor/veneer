use crate::{CssInJs, inject_styles_arc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FullCssBundleOptions {
    pub include_runtime_injector: bool,
    pub include_cssinjs: bool,
}

impl Default for FullCssBundleOptions {
    fn default() -> Self {
        Self {
            include_runtime_injector: true,
            include_cssinjs: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FullCssBundleSnapshot {
    pub runtime_css_len: usize,
    pub cssinjs_css_len: usize,
    pub total_css_len: usize,
    pub css: String,
}

pub struct FullCssBundle;

impl FullCssBundle {
    #[inline]
    #[must_use]
    pub fn snapshot(options: FullCssBundleOptions) -> FullCssBundleSnapshot {
        let runtime_css = if options.include_runtime_injector {
            inject_styles_arc().to_string()
        } else {
            String::new()
        };
        let cssinjs_css = if options.include_cssinjs {
            CssInJs::css_arc().to_string()
        } else {
            String::new()
        };

        let runtime_css_len = runtime_css.len();
        let cssinjs_css_len = cssinjs_css.len();

        let mut out = String::with_capacity(runtime_css_len + cssinjs_css_len + 2);
        if !runtime_css.is_empty() {
            out.push_str(runtime_css.as_str());
            if !out.ends_with('\n') {
                out.push('\n');
            }
        }
        if !cssinjs_css.is_empty() {
            out.push_str(cssinjs_css.as_str());
            if !out.ends_with('\n') {
                out.push('\n');
            }
        }

        FullCssBundleSnapshot {
            runtime_css_len,
            cssinjs_css_len,
            total_css_len: out.len(),
            css: out,
        }
    }
}
