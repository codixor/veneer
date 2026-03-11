//! WASM‑specific DOM injection for the CSS‑in‑JS runtime.

use std::sync::{OnceLock, RwLock};

use wasm_bindgen::JsCast;

use super::CssInJs;
use super::CssInJsEntry;
use super::config::store as config_store;
use super::registry::CssInJsStyleRecord;
use super::registry::RegisterResult;

pub(crate) struct CssDomInjector;

impl CssDomInjector {
    fn pending_flag() -> &'static RwLock<bool> {
        static FLAG: OnceLock<RwLock<bool>> = OnceLock::new();
        FLAG.get_or_init(|| RwLock::new(false))
    }

    pub fn schedule_sync() {
        let already = match Self::pending_flag().write() {
            Ok(mut f) => {
                if *f {
                    true
                } else {
                    *f = true;
                    false
                }
            }
            Err(poisoned) => {
                let mut f = poisoned.into_inner();
                if *f {
                    true
                } else {
                    *f = true;
                    false
                }
            }
        };
        if already {
            return;
        }

        let Some(window) = web_sys::window() else {
            Self::clear_pending();
            return;
        };
        let cb = wasm_bindgen::closure::Closure::once_into_js(move || {
            CssDomInjector::clear_pending();
            let _ = CssDomInjector::sync_all();
        });
        let _ = window.request_animation_frame(cb.as_ref().unchecked_ref());
    }

    fn clear_pending() {
        if let Ok(mut f) = Self::pending_flag().write() {
            *f = false;
        }
    }

    pub fn sync_register_result(result: &RegisterResult) -> Option<()> {
        if let Some(old) = result.removed_old_key.as_deref() {
            let _ = Self::remove_node(old);
        }
        if !result.changed {
            return Some(());
        }

        if CssInJs::owner_key().is_some() {
            return Self::sync_all();
        }
        let _ = Self::sync_entry(&result.entry);
        Self::reorder_registered_nodes()
    }

    pub fn sync_all() -> Option<()> {
        let css = CssInJs::css_arc();
        let cfg = config_store::get();
        let owner_key = cfg
            .style_node_owner_key
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())?;

        let (document, head) = Self::doc_head()?;
        let el = Self::find_or_create_owner_node(&document, &cfg, owner_key)?;
        Self::apply_owner_node_attrs(&el, &cfg, owner_key);

        if el.text_content().as_deref() != Some(css.as_ref()) {
            el.set_text_content(Some(css.as_ref()));
        }
        let _ = head.append_child(&el);
        Some(())
    }

    pub fn sync_entry(entry: &CssInJsEntry) -> Option<()> {
        let (document, head) = Self::doc_head()?;
        let cfg = config_store::get();

        let el = Self::find_or_create_style_node(&document, &cfg, entry)?;
        Self::apply_node_attrs(&el, &cfg, entry);

        if el.text_content().as_deref() != Some(entry.rendered_css.as_ref()) {
            el.set_text_content(Some(entry.rendered_css.as_ref()));
        }
        let _ = head.append_child(&el);
        Some(())
    }

    fn reorder_registered_nodes() -> Option<()> {
        let (document, head) = Self::doc_head()?;
        let cfg = config_store::get();
        let records = CssInJs::records();
        let css_entries = CssInJs::css_entries();

        if records.len() != css_entries.len() {
            return None;
        }

        for (record, css_entry) in records.into_iter().zip(css_entries.into_iter()) {
            if record.cache_key != css_entry.cache_key {
                return None;
            }

            let el = Self::find_or_create_style_node_for_cache_key(
                &document,
                &cfg,
                record.cache_key.as_str(),
            )?;
            Self::apply_node_record_attrs(&el, &cfg, &record);

            if el.text_content().as_deref() != Some(css_entry.rendered_css.as_ref()) {
                el.set_text_content(Some(css_entry.rendered_css.as_ref()));
            }

            let _ = head.append_child(&el);
        }

        Some(())
    }

    fn doc_head() -> Option<(web_sys::Document, web_sys::HtmlHeadElement)> {
        let window = web_sys::window()?;
        let document = window.document()?;
        let head = document.head()?;
        Some((document, head))
    }

    fn find_or_create_owner_node(
        document: &web_sys::Document,
        cfg: &super::config::CssInJsConfig,
        owner_key: &str,
    ) -> Option<web_sys::Element> {
        if let Some(prefix) = cfg
            .style_node_id_prefix
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            let id = format!("{prefix}{owner_key}");
            if let Some(existing) = document.get_element_by_id(&id) {
                return Some(existing);
            }
            let created = document.create_element("style").ok()?;
            created.set_id(&id);
            return Some(created);
        }
        if let Some(attr) = cfg
            .style_node_id_attr
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            let selector = format!(r#"style[{}="{}"]"#, attr, owner_key);
            if let Ok(Some(existing)) = document.query_selector(&selector) {
                return Some(existing);
            }
            let created = document.create_element("style").ok()?;
            let _ = created.set_attribute(attr, owner_key);
            return Some(created);
        }
        let created = document.create_element("style").ok()?;
        let _ = created.set_attribute("data-cssinjs-owner", owner_key);
        Some(created)
    }

    fn apply_owner_node_attrs(
        el: &web_sys::Element,
        cfg: &super::config::CssInJsConfig,
        owner_key: &str,
    ) {
        Self::apply_nonce(el, cfg, None);

        if !cfg.emit_node_attrs {
            return;
        }
        for (k, v) in &cfg.extra_attrs {
            let k = k.trim();
            if !k.is_empty() {
                let _ = el.set_attribute(k, v);
            }
        }
        if let Some(attr) = cfg
            .meta_attrs
            .style_id_attr
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            let _ = el.set_attribute(attr, owner_key);
        }
    }

    fn find_or_create_style_node(
        document: &web_sys::Document,
        cfg: &super::config::CssInJsConfig,
        entry: &CssInJsEntry,
    ) -> Option<web_sys::Element> {
        Self::find_or_create_style_node_for_cache_key(document, cfg, entry.cache_key.as_str())
    }

    fn find_or_create_style_node_for_cache_key(
        document: &web_sys::Document,
        cfg: &super::config::CssInJsConfig,
        cache_key: &str,
    ) -> Option<web_sys::Element> {
        if let Some(id) = CssInJs::node_id(cache_key) {
            if let Some(existing) = document.get_element_by_id(&id) {
                return Some(existing);
            }
            let created = document.create_element("style").ok()?;
            created.set_id(&id);
            return Some(created);
        }
        if let Some(attr) = cfg
            .style_node_id_attr
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            let selector = format!(r#"style[{}="{}"]"#, attr, cache_key);
            if let Ok(Some(existing)) = document.query_selector(&selector) {
                return Some(existing);
            }
            let created = document.create_element("style").ok()?;
            let _ = created.set_attribute(attr, cache_key);
            return Some(created);
        }
        document.create_element("style").ok()
    }

    fn apply_node_attrs(
        el: &web_sys::Element,
        cfg: &super::config::CssInJsConfig,
        entry: &CssInJsEntry,
    ) {
        Self::set_nonce_attr(el, cfg, entry.nonce.as_deref());
        if !cfg.emit_node_attrs {
            return;
        }

        for (k, v) in &cfg.extra_attrs {
            let k = k.trim();
            if !k.is_empty() {
                let _ = el.set_attribute(k, v.as_str());
            }
        }
        let m = &cfg.meta_attrs;
        Self::set_opt_attr(el, m.enabled_attr.as_deref(), Some("true"));
        Self::set_opt_attr(
            el,
            m.style_id_attr.as_deref(),
            Some(entry.style_id.as_ref()),
        );
        Self::set_opt_attr(
            el,
            m.hash_class_attr.as_deref(),
            Some(entry.hash_class.as_ref()),
        );
    }

    fn apply_nonce(
        el: &web_sys::Element,
        cfg: &super::config::CssInJsConfig,
        entry: Option<&CssInJsEntry>,
    ) {
        Self::set_nonce_attr(el, cfg, entry.and_then(|value| value.nonce.as_deref()));
    }

    fn apply_node_record_attrs(
        el: &web_sys::Element,
        cfg: &super::config::CssInJsConfig,
        record: &CssInJsStyleRecord,
    ) {
        Self::set_nonce_attr(el, cfg, record.nonce.as_deref());
        if !cfg.emit_node_attrs {
            return;
        }

        for (k, v) in &cfg.extra_attrs {
            let k = k.trim();
            if !k.is_empty() {
                let _ = el.set_attribute(k, v.as_str());
            }
        }
        let m = &cfg.meta_attrs;
        Self::set_opt_attr(el, m.enabled_attr.as_deref(), Some("true"));
        Self::set_opt_attr(
            el,
            m.style_id_attr.as_deref(),
            Some(record.style_id.as_str()),
        );
        Self::set_opt_attr(el, m.hash_class_attr.as_deref(), Some(record.hash.as_str()));
    }

    fn set_nonce_attr(
        el: &web_sys::Element,
        cfg: &super::config::CssInJsConfig,
        value: Option<&str>,
    ) {
        let name = cfg
            .nonce_attr
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty());
        let Some(name) = name else {
            return;
        };
        let value = value.map(str::trim).filter(|v| !v.is_empty());
        if let Some(v) = value {
            let _ = el.set_attribute(name, v);
        } else {
            let _ = el.remove_attribute(name);
        }
    }

    pub fn remove_node(cache_key: &str) -> Option<()> {
        let (document, _) = Self::doc_head()?;
        let cfg = config_store::get();

        if let Some(id) = CssInJs::node_id(cache_key) {
            if let Some(node) = document.get_element_by_id(&id) {
                node.remove();
            }
            return Some(());
        }

        if let Some(attr) = cfg
            .style_node_id_attr
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            let selector = format!(r#"style[{}="{}"]"#, attr, cache_key);
            if let Ok(Some(node)) = document.query_selector(&selector) {
                node.remove();
            }
        }
        Some(())
    }

    fn set_opt_attr(el: &web_sys::Element, name: Option<&str>, value: Option<&str>) {
        let Some(name) = name.map(str::trim).filter(|v| !v.is_empty()) else {
            return;
        };
        let value = value.map(str::trim).filter(|v| !v.is_empty());
        if let Some(v) = value {
            let _ = el.set_attribute(name, v);
        } else {
            let _ = el.remove_attribute(name);
        }
    }
}
