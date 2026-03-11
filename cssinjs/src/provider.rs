use dioxus::prelude::*;
use std::time::Duration;

use crate::BundleExtractState;
use crate::backend as style_backend;
use crate::{CssInJsRuntime, CssInJsRuntimeConfig};

#[derive(Clone, Copy)]
pub struct CssInJsCtx {
    pub runtime: CssInJsRuntime,
    pub config: Signal<CssInJsRuntimeConfig>,
    pub extract_state: Signal<BundleExtractState>,
}

#[derive(Props, Clone, PartialEq)]
pub struct CssInJsProviderProps {
    #[props(default)]
    pub config: Option<CssInJsRuntimeConfig>,
    #[props(default)]
    pub style_config: Option<style_backend::StyleConfig>,
    #[props(default = true)]
    pub bridge_to_header: bool,
    #[props(default = 16)]
    pub bridge_poll_ms: u64,
    #[props(default = "cssinjs-runtime".to_string())]
    pub bridge_style_key: String,
    #[props(default)]
    pub bridge_style_nonce: Option<String>,
    #[props(default)]
    pub ssr_inline: bool,
    #[props(default = "cssinjs-inline".to_string())]
    pub inline_style_key: String,
    pub children: Element,
}

#[component]
pub fn CssInJsProvider(props: CssInJsProviderProps) -> Element {
    use_hook(crate::platform::install_platform_hooks);

    let runtime = CssInJsRuntime;
    let mut initial_cfg = props.config.clone().unwrap_or_default();
    if props.bridge_to_header {
        initial_cfg.runtime_dom_injection = false;
    }

    let mut config_signal = use_signal(|| initial_cfg.clone());
    let extract_state_signal = use_signal(BundleExtractState::default);
    let _ = use_context_provider(|| CssInJsCtx {
        runtime,
        config: config_signal,
        extract_state: extract_state_signal,
    });

    let mut next_cfg = props.config.clone().unwrap_or_default();
    if props.bridge_to_header {
        next_cfg.runtime_dom_injection = false;
    }
    if *config_signal.peek() != next_cfg {
        config_signal.set(next_cfg);
    }

    use_effect(move || {
        let _ = runtime.set_config(config_signal());
    });

    let local_header_manager = use_signal(header::HeadManager::default);
    let header_manager = header::try_use_header().unwrap_or(local_header_manager);
    let mut last_bridge_css = use_signal(|| std::sync::Arc::<str>::from(""));
    let mut last_bridge_revision = use_signal(|| 0u64);
    let mut observed_revision = use_signal(|| runtime.revision());
    let mut inline_css = use_signal(String::new);
    let mut document_style_revision = use_signal(|| 0u64);
    let capabilities = crate::platform::detect_capabilities();
    let bridge_enabled = props.bridge_to_header;
    let document_head_enabled = bridge_enabled && capabilities.document_head;
    let header_bridge_enabled = bridge_enabled && !document_head_enabled;
    let bridge_style_key = props.bridge_style_key.clone();
    let bridge_style_nonce = props.bridge_style_nonce.clone();
    let inline_enabled = props.ssr_inline;
    let revision_events_enabled =
        capabilities.revision_events && (bridge_enabled || inline_enabled);
    let fallback_poll_enabled = !capabilities.revision_events
        && (bridge_enabled || inline_enabled)
        && props.bridge_poll_ms > 0;
    let fallback_poll_ms = if fallback_poll_enabled {
        props.bridge_poll_ms.max(16)
    } else {
        // Keep timer hook idle when revision event callbacks are available.
        3_600_000
    };
    let bridge_tick = futures_times::dioxus::use_now_ms(Duration::from_millis(fallback_poll_ms));
    let schedule_bridge_update = use_hook(dioxus_core::schedule_update);
    let mut revision_listener_id = use_signal(|| Option::<u64>::None);
    let runtime_revision = runtime.revision();
    if *observed_revision.peek() != runtime_revision {
        observed_revision.set(runtime_revision);
    }
    use_effect(move || {
        if !revision_events_enabled {
            let existing_listener_id = *revision_listener_id.peek();
            if let Some(listener_id) = existing_listener_id {
                let _ = runtime.unsubscribe_revision_listener(listener_id);
                revision_listener_id.set(None);
            }
            return;
        }
        if revision_listener_id.peek().is_some() {
            return;
        }
        let schedule_bridge_update = schedule_bridge_update.clone();
        let listener_id =
            runtime.subscribe_revision_listener(std::sync::Arc::new(move |_revision| {
                schedule_bridge_update();
            }));
        revision_listener_id.set(Some(listener_id));
    });
    {
        let revision_listener_id = revision_listener_id;
        use_drop(move || {
            if let Some(listener_id) = *revision_listener_id.peek() {
                let _ = runtime.unsubscribe_revision_listener(listener_id);
            }
        });
    }
    use_effect(move || {
        let observed = observed_revision();
        if fallback_poll_enabled {
            let _ = bridge_tick();
            let revision = runtime.revision();
            if *observed_revision.peek() != revision {
                observed_revision.set(revision);
            }
        }
        if !bridge_enabled && !inline_enabled {
            return;
        }
        if *last_bridge_revision.peek() == observed {
            return;
        }
        last_bridge_revision.set(observed);

        let css_arc = runtime.css_arc();
        let previous_css = last_bridge_css.peek().clone();
        if std::sync::Arc::ptr_eq(&previous_css, &css_arc) {
            return;
        }
        last_bridge_css.set(css_arc.clone());
        let css_text = css_arc.as_ref().to_string();

        if inline_enabled || document_head_enabled {
            inline_css.set(css_text.clone());
            if document_head_enabled {
                document_style_revision.set(observed);
            }
        }

        if !header_bridge_enabled {
            return;
        }

        let mut manager_signal = header_manager;
        let mut state = manager_signal.write();
        let mut next = state.current.clone();

        let key = bridge_style_key.clone();
        if css_text.trim().is_empty() {
            next.styles.remove(&key);
        } else {
            let mut style_node =
                header::StyleNode::new(key.clone(), css_text).with_type("text/css");
            if let Some(nonce) = bridge_style_nonce
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                style_node = style_node.with_nonce(nonce);
            }
            next.styles.insert(key, style_node);
        }

        state.stage(next);
        let _ = state.commit_to_platform();
    });

    let style_config = props.style_config.clone();
    let inline_enabled = props.ssr_inline;
    let inline_style_key = props.inline_style_key.clone();
    let inline_style_text = inline_css();
    let document_style_key = props.bridge_style_key.clone();
    let document_style_nonce = props.bridge_style_nonce.clone();
    let document_style_revision = document_style_revision();
    rsx! {
        style_backend::StyleProvider { config: style_config,
            if document_head_enabled && !inline_style_text.trim().is_empty() {
                dioxus::document::Style {
                    key: "{document_style_revision}",
                    id: document_style_key,
                    nonce: document_style_nonce,
                    "{inline_style_text}"
                }
            }
            if inline_enabled && !document_head_enabled && !inline_style_text.trim().is_empty() {
                style {
                    id: inline_style_key,
                    dangerous_inner_html: inline_style_text,
                }
            }
            {props.children}
        }
    }
}
