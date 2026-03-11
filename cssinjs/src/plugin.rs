use dioxus::prelude::*;
use plugins::PrimitivePlugin;

use crate::backend as style_backend;
use crate::provider::CssInJsProvider;
use crate::runtime::CssInJsRuntimeConfig;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CssInJsPlugin {
    pub config: Option<CssInJsRuntimeConfig>,
    pub style_config: Option<style_backend::StyleConfig>,
    pub bridge_to_header: bool,
    pub bridge_poll_ms: u64,
    pub bridge_style_key: Option<String>,
    pub bridge_style_nonce: Option<String>,
    pub ssr_inline: bool,
    pub inline_style_key: Option<String>,
}

impl Default for CssInJsPlugin {
    fn default() -> Self {
        Self {
            config: None,
            style_config: None,
            bridge_to_header: true,
            bridge_poll_ms: 16,
            bridge_style_key: None,
            bridge_style_nonce: None,
            ssr_inline: false,
            inline_style_key: None,
        }
    }
}

impl PrimitivePlugin for CssInJsPlugin {
    fn install(&self, children: Element) -> Element {
        let config = self.config.clone();
        let style_config = self.style_config.clone();
        let bridge_to_header = self.bridge_to_header;
        let bridge_poll_ms = if self.bridge_poll_ms == 0 {
            16
        } else {
            self.bridge_poll_ms
        };
        let bridge_style_key = self
            .bridge_style_key
            .clone()
            .unwrap_or_else(|| "cssinjs-runtime".to_string());
        let bridge_style_nonce = self.bridge_style_nonce.clone();
        let ssr_inline = self.ssr_inline;
        let inline_style_key = self
            .inline_style_key
            .clone()
            .unwrap_or_else(|| "cssinjs-inline".to_string());

        rsx! {
            CssInJsProvider {
                config,
                style_config,
                bridge_to_header,
                bridge_poll_ms,
                bridge_style_key,
                bridge_style_nonce,
                ssr_inline,
                inline_style_key,
                {children}
            }
        }
    }
}
