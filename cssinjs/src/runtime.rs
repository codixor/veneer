use std::collections::HashMap;
use std::sync::Arc;
use std::sync::{OnceLock, RwLock};

use crate::backend as style_backend;

pub type CssVarRegisterInput = style_backend::CssVarRegisterInput;
pub type CssVarRegisterOutput = style_backend::CssVarRegisterOutput;
pub type CssVarTokenMap = style_backend::CssVarTokenMap;
pub type CssVarTokenValue = style_backend::CssVarTokenValue;

#[inline]
#[must_use]
pub fn unit(value: impl Into<CssVarTokenValue>) -> String {
    style_backend::CssInJs::unit(value)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DevStyleRecord {
    pub key: String,
    pub cache_key: String,
    pub class_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DevTokenRecord {
    pub key: String,
    pub cache_key: String,
    pub hash_class: String,
    pub style_id: String,
    pub css_var_key: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DevStyleEntry {
    cache_key: String,
    class_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DevTokenEntry {
    cache_key: String,
    hash_class: String,
    style_id: String,
    css_var_key: String,
}

pub type CssInJsRuntimeConfig = style_backend::CssInJsConfig;

fn dev_style_map() -> &'static RwLock<HashMap<String, DevStyleEntry>> {
    static STORE: OnceLock<RwLock<HashMap<String, DevStyleEntry>>> = OnceLock::new();
    STORE.get_or_init(|| RwLock::new(HashMap::new()))
}

fn dev_token_map() -> &'static RwLock<HashMap<String, DevTokenEntry>> {
    static STORE: OnceLock<RwLock<HashMap<String, DevTokenEntry>>> = OnceLock::new();
    STORE.get_or_init(|| RwLock::new(HashMap::new()))
}

fn dev_style_compose_key(component_key: &str, style_id: &str) -> String {
    format!("{}::{}", component_key.trim(), style_id.trim())
}

fn dev_token_compose_key(component_key: &str, token_id: &str) -> String {
    format!("{}::{}", component_key.trim(), token_id.trim())
}

fn dev_style_scope_key(component_key: &str, style_id: &str) -> String {
    format!("dev|{}|{}", component_key.trim(), style_id.trim())
}

fn normalize_dev_ident(value: &str, fallback: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            continue;
        }
        if ch == '-' || ch == '_' {
            out.push(ch);
            continue;
        }
        if ch.is_whitespace() || ch == ':' || ch == '/' || ch == '.' {
            out.push('-');
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() {
        fallback.to_string()
    } else {
        out
    }
}

const DEV_STYLE_CLASS_PREFIX: &str = "_R_";
const DEV_STYLE_CLASS_SUFFIX: &str = "_";
const DEV_STYLE_CLASS_HASH_LEN: usize = 6;
const DEV_STYLE_CLASS_ALPHABET: &[u8; 52] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

fn fnv1a_64(input: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for b in input {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn encode_alpha_hash_6(mut value: u64) -> String {
    let mut out = String::with_capacity(DEV_STYLE_CLASS_HASH_LEN);
    for _ in 0..DEV_STYLE_CLASS_HASH_LEN {
        // Extra mixing keeps neighboring keys from clustering in low bits.
        value = value.wrapping_mul(0x9e37_79b9_7f4a_7c15).rotate_left(11) ^ 0xbf58_476d_1ce4_e5b9;
        let index = (value % DEV_STYLE_CLASS_ALPHABET.len() as u64) as usize;
        out.push(char::from(DEV_STYLE_CLASS_ALPHABET[index]));
    }
    out
}

fn dev_style_class_name(key: &str) -> String {
    // Deterministic class name layout: `_R_[A-Za-z]{6}_`.
    let hash = fnv1a_64(key.as_bytes());
    let short = encode_alpha_hash_6(hash);
    format!("{DEV_STYLE_CLASS_PREFIX}{short}{DEV_STYLE_CLASS_SUFFIX}")
}

fn dev_token_hash_seed(token: &CssVarTokenMap) -> String {
    let mut payload = String::new();
    for (key, value) in token {
        payload.push_str(key.as_str());
        payload.push('=');
        payload.push_str(value.to_string().as_str());
        payload.push(';');
    }
    format!("{:016x}", fnv1a_64(payload.as_bytes()))
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CssInJsRuntime;

impl CssInJsRuntime {
    #[must_use]
    pub fn config(self) -> CssInJsRuntimeConfig {
        style_backend::CssInJs::config()
    }

    pub fn set_config(self, next: CssInJsRuntimeConfig) -> bool {
        style_backend::CssInJs::set_config(next)
    }

    #[must_use]
    pub fn css_arc(self) -> Arc<str> {
        style_backend::CssInJs::css_arc()
    }

    #[must_use]
    pub fn css(self) -> String {
        self.css_arc().to_string()
    }

    #[must_use]
    pub fn len(self) -> usize {
        style_backend::CssInJs::len()
    }

    #[must_use]
    pub fn is_empty(self) -> bool {
        style_backend::CssInJs::is_empty()
    }

    #[must_use]
    pub fn revision(self) -> u64 {
        style_backend::CssInJs::revision()
    }

    #[must_use]
    pub fn subscribe_revision_listener(
        self,
        listener: Arc<dyn Fn(u64) + Send + Sync + 'static>,
    ) -> u64 {
        style_backend::CssInJs::subscribe_revision_listener(listener)
    }

    #[must_use]
    pub fn unsubscribe_revision_listener(self, listener_id: u64) -> bool {
        style_backend::CssInJs::unsubscribe_revision_listener(listener_id)
    }

    pub fn clear(self) {
        style_backend::CssInJs::clear();
    }

    #[must_use]
    pub fn unregister_cache_key(self, cache_key: &str) -> bool {
        style_backend::CssInJs::unregister(cache_key.trim())
    }

    #[must_use]
    pub fn register_css_vars(
        self,
        config: &CssVarRegisterInput,
        token: &CssVarTokenMap,
    ) -> CssVarRegisterOutput {
        style_backend::CssInJs::register_css_vars(config, token)
    }

    #[must_use]
    pub fn upsert_dev_style(
        self,
        component_key: &str,
        style_id: &str,
        css: &str,
    ) -> Option<DevStyleRecord> {
        let component_key = component_key.trim();
        let style_id = style_id.trim();
        let css = css.trim();

        if component_key.is_empty() || style_id.is_empty() {
            return None;
        }
        if css.is_empty() {
            let _ = self.remove_dev_style(component_key, style_id);
            return None;
        }

        let key = dev_style_compose_key(component_key, style_id);
        let class_name = dev_style_class_name(&key);
        let input = style_backend::CssInJsStyleInput {
            style_id: Arc::<str>::from(format!("dev::{style_id}")),
            css: Arc::<str>::from(css.to_string()),
            identity_scope: Some(Arc::<str>::from(dev_style_scope_key(
                component_key,
                style_id,
            ))),
            hash_class: Some(Arc::<str>::from(class_name.clone())),
            layer: Some(Arc::<str>::from("dev")),
            ..style_backend::CssInJsStyleInput::default()
        };

        let reg = style_backend::CssInJs::register(input)?;
        let entry = DevStyleEntry {
            cache_key: reg.cache_key.clone(),
            class_name: class_name.clone(),
        };

        match dev_style_map().write() {
            Ok(mut map) => {
                map.insert(key.clone(), entry);
            }
            Err(poisoned) => {
                let mut map = poisoned.into_inner();
                map.insert(key.clone(), entry);
            }
        }

        Some(DevStyleRecord {
            key,
            cache_key: reg.cache_key,
            class_name,
        })
    }

    #[must_use]
    pub fn remove_dev_style(self, component_key: &str, style_id: &str) -> bool {
        let key = dev_style_compose_key(component_key, style_id);
        let removed = match dev_style_map().write() {
            Ok(mut map) => map.remove(&key),
            Err(poisoned) => {
                let mut map = poisoned.into_inner();
                map.remove(&key)
            }
        };

        if let Some(entry) = removed {
            style_backend::CssInJs::unregister(&entry.cache_key)
        } else {
            false
        }
    }

    pub fn clear_dev_styles(self) {
        let entries = match dev_style_map().write() {
            Ok(mut map) => map.drain().map(|(_, value)| value).collect::<Vec<_>>(),
            Err(poisoned) => {
                let mut map = poisoned.into_inner();
                map.drain().map(|(_, value)| value).collect::<Vec<_>>()
            }
        };

        for entry in entries {
            let _ = style_backend::CssInJs::unregister(&entry.cache_key);
        }
    }

    #[must_use]
    pub fn list_dev_styles(self) -> Vec<DevStyleRecord> {
        let mut out = match dev_style_map().read() {
            Ok(map) => map
                .iter()
                .map(|(key, value)| DevStyleRecord {
                    key: key.clone(),
                    cache_key: value.cache_key.clone(),
                    class_name: value.class_name.clone(),
                })
                .collect::<Vec<_>>(),
            Err(poisoned) => {
                let map = poisoned.into_inner();
                map.iter()
                    .map(|(key, value)| DevStyleRecord {
                        key: key.clone(),
                        cache_key: value.cache_key.clone(),
                        class_name: value.class_name.clone(),
                    })
                    .collect::<Vec<_>>()
            }
        };
        out.sort_by(|a, b| a.key.cmp(&b.key));
        out
    }

    #[must_use]
    pub fn upsert_dev_tokens(
        self,
        component_key: &str,
        token_id: &str,
        token: &CssVarTokenMap,
    ) -> Option<DevTokenRecord> {
        let component_key = component_key.trim();
        let token_id = token_id.trim();

        if component_key.is_empty() || token_id.is_empty() {
            return None;
        }
        if token.is_empty() {
            let _ = self.remove_dev_tokens(component_key, token_id);
            return None;
        }

        let key = dev_token_compose_key(component_key, token_id);
        let component_ident = normalize_dev_ident(component_key, "component");
        let token_ident = normalize_dev_ident(token_id, "token");
        let scope_class = dev_style_class_name(format!("dev-token::{key}").as_str());

        let cfg = CssVarRegisterInput {
            path: vec![
                Arc::<str>::from("dev"),
                Arc::<str>::from(component_ident.clone()),
                Arc::<str>::from(token_ident.clone()),
            ],
            key: Arc::<str>::from(format!("dev-token-{token_ident}")),
            style_id: Some(Arc::<str>::from(format!(
                "dev::token::{component_ident}::{token_ident}"
            ))),
            prefix: Some(Arc::<str>::from(format!("dev-{component_ident}"))),
            scope: vec![Arc::<str>::from(format!("dev-scope-{component_ident}"))],
            token_hash: Some(Arc::<str>::from(dev_token_hash_seed(token))),
            hash_class: Some(Arc::<str>::from(scope_class.clone())),
            layer: Some(Arc::<str>::from("dev")),
            ..CssVarRegisterInput::default()
        };

        let out = style_backend::CssInJs::register_css_vars(&cfg, token);
        let cache_key = out
            .registration
            .as_ref()
            .map(|value| value.cache_key.clone())
            .unwrap_or_default();
        let hash_class = out.hash_class.unwrap_or(scope_class);
        let entry = DevTokenEntry {
            cache_key: cache_key.clone(),
            hash_class: hash_class.clone(),
            style_id: out.style_id.clone(),
            css_var_key: out.css_var_key.clone(),
        };

        let previous = match dev_token_map().write() {
            Ok(mut map) => map.insert(key.clone(), entry),
            Err(poisoned) => {
                let mut map = poisoned.into_inner();
                map.insert(key.clone(), entry)
            }
        };

        if let Some(previous) = previous
            && !previous.cache_key.is_empty()
            && previous.cache_key != cache_key
        {
            let _ = style_backend::CssInJs::unregister(previous.cache_key.as_str());
        }

        Some(DevTokenRecord {
            key,
            cache_key,
            hash_class,
            style_id: out.style_id,
            css_var_key: out.css_var_key,
        })
    }

    #[must_use]
    pub fn remove_dev_tokens(self, component_key: &str, token_id: &str) -> bool {
        let key = dev_token_compose_key(component_key, token_id);
        let removed = match dev_token_map().write() {
            Ok(mut map) => map.remove(&key),
            Err(poisoned) => {
                let mut map = poisoned.into_inner();
                map.remove(&key)
            }
        };

        if let Some(entry) = removed {
            if entry.cache_key.is_empty() {
                true
            } else {
                style_backend::CssInJs::unregister(entry.cache_key.as_str())
            }
        } else {
            false
        }
    }

    pub fn clear_dev_tokens(self) {
        let entries = match dev_token_map().write() {
            Ok(mut map) => map.drain().map(|(_, value)| value).collect::<Vec<_>>(),
            Err(poisoned) => {
                let mut map = poisoned.into_inner();
                map.drain().map(|(_, value)| value).collect::<Vec<_>>()
            }
        };

        for entry in entries {
            if !entry.cache_key.is_empty() {
                let _ = style_backend::CssInJs::unregister(entry.cache_key.as_str());
            }
        }
    }

    #[must_use]
    pub fn list_dev_tokens(self) -> Vec<DevTokenRecord> {
        let mut out = match dev_token_map().read() {
            Ok(map) => map
                .iter()
                .map(|(key, value)| DevTokenRecord {
                    key: key.clone(),
                    cache_key: value.cache_key.clone(),
                    hash_class: value.hash_class.clone(),
                    style_id: value.style_id.clone(),
                    css_var_key: value.css_var_key.clone(),
                })
                .collect::<Vec<_>>(),
            Err(poisoned) => {
                let map = poisoned.into_inner();
                map.iter()
                    .map(|(key, value)| DevTokenRecord {
                        key: key.clone(),
                        cache_key: value.cache_key.clone(),
                        hash_class: value.hash_class.clone(),
                        style_id: value.style_id.clone(),
                        css_var_key: value.css_var_key.clone(),
                    })
                    .collect::<Vec<_>>()
            }
        };
        out.sort_by(|a, b| a.key.cmp(&b.key));
        out
    }
}
