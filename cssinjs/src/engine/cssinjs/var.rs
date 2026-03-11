//! CSS variables engine: token conversion, serialization, and registration.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use super::CssInJs;
use super::CssInJsStyleInput;
use super::config::store as config_store;
use super::hash;
use crate::style_provider::HashPriority;

// ----------------------------------------------------------------------------
// Token types
// ----------------------------------------------------------------------------

pub type CssVarTokenMap = BTreeMap<String, CssVarTokenValue>;
pub type CssVarMergedTokenMap = BTreeMap<String, String>;

#[derive(Clone, Debug, PartialEq)]
pub enum CssVarTokenValue {
    Number(f64),
    Text(Arc<str>),
}

impl std::fmt::Display for CssVarTokenValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Number(num) => f.write_str(&super::util::format_css_number(*num)),
            Self::Text(text) => f.write_str(text),
        }
    }
}

macro_rules! impl_from_num {
    ($($t:ty),*) => {
        $(
            impl From<$t> for CssVarTokenValue {
                #[inline]
                fn from(v: $t) -> Self {
                    Self::Number(v as f64)
                }
            }
        )*
    };
}
impl_from_num!(f64, f32, i32, u32);

impl From<String> for CssVarTokenValue {
    #[inline]
    fn from(v: String) -> Self {
        Self::Text(Arc::<str>::from(v))
    }
}
impl From<&str> for CssVarTokenValue {
    #[inline]
    fn from(v: &str) -> Self {
        Self::Text(Arc::<str>::from(v))
    }
}
impl From<Arc<str>> for CssVarTokenValue {
    #[inline]
    fn from(v: Arc<str>) -> Self {
        Self::Text(v)
    }
}

// ----------------------------------------------------------------------------
// Serialization config
// ----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CssVarSerializeCfg {
    pub scope: Vec<Arc<str>>,
    pub hash_class: Option<Arc<str>>,
    pub hash_priority: HashPriority,
}

impl Default for CssVarSerializeCfg {
    fn default() -> Self {
        Self {
            scope: Vec::new(),
            hash_class: None,
            hash_priority: HashPriority::Low,
        }
    }
}

// ----------------------------------------------------------------------------
// Transformation config
// ----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CssVarTransformCfg {
    pub prefix: Option<Arc<str>>,
    pub ignore: BTreeSet<String>,
    pub unitless: BTreeSet<String>,
    pub preserve: BTreeSet<String>,
    pub scope: Vec<Arc<str>>,
    pub hash_class: Option<Arc<str>>,
    pub hash_priority: HashPriority,
}

impl Default for CssVarTransformCfg {
    fn default() -> Self {
        Self {
            prefix: None,
            ignore: BTreeSet::new(),
            unitless: BTreeSet::new(),
            preserve: BTreeSet::new(),
            scope: Vec::new(),
            hash_class: None,
            hash_priority: HashPriority::Low,
        }
    }
}

// ----------------------------------------------------------------------------
// Register input/output
// ----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CssVarRegisterInput {
    pub path: Vec<Arc<str>>,
    pub key: Arc<str>,
    pub style_id: Option<Arc<str>>,
    pub prefix: Option<Arc<str>>,
    pub unitless: BTreeSet<String>,
    pub ignore: BTreeSet<String>,
    pub preserve: BTreeSet<String>,
    pub scope: Vec<Arc<str>>,
    pub token_hash: Option<Arc<str>>,
    pub hash_class: Option<Arc<str>>,
    pub hash_priority: HashPriority,
    pub layer: Option<Arc<str>>,
    pub nonce: Option<Arc<str>>,
}

impl Default for CssVarRegisterInput {
    fn default() -> Self {
        Self {
            path: Vec::new(),
            key: Arc::<str>::from("vars"),
            style_id: None,
            prefix: None,
            unitless: BTreeSet::new(),
            ignore: BTreeSet::new(),
            preserve: BTreeSet::new(),
            scope: Vec::new(),
            token_hash: None,
            hash_class: None,
            hash_priority: HashPriority::Low,
            layer: None,
            nonce: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CssVarRegisterOutput {
    pub merged_token: CssVarMergedTokenMap,
    pub css_vars_css: String,
    pub style_id: String,
    pub css_var_key: String,
    pub hash_class: Option<String>,
    pub registration: Option<super::CssInJsRegistration>,
}

// ----------------------------------------------------------------------------
// Engine
// ----------------------------------------------------------------------------

pub(crate) struct CssVarEngine;

impl CssVarEngine {
    #[inline]
    pub fn unit(value: impl Into<CssVarTokenValue>) -> String {
        match value.into() {
            CssVarTokenValue::Number(num) => format!("{}px", super::util::format_css_number(num)),
            CssVarTokenValue::Text(text) => text.to_string(),
        }
    }

    pub fn token_to_css_var(token: &str, prefix: Option<&str>) -> String {
        let token = super::util::normalize_css_ident(token);
        let prefix = prefix.unwrap_or("").trim();
        if prefix.is_empty() {
            format!("--{token}")
        } else {
            format!("--{}-{token}", super::util::normalize_css_ident(prefix))
        }
    }

    pub fn serialize_css_vars(
        css_vars: &BTreeMap<String, String>,
        hash_id: &str,
        options: &CssVarSerializeCfg,
    ) -> String {
        let hash_id = hash_id.trim();
        if hash_id.is_empty() || css_vars.is_empty() {
            return String::new();
        }

        let base_selector = {
            let where_sel =
                Self::where_selector(options.hash_class.as_deref(), options.hash_priority);
            let hash_selector = super::hash::class_selector(hash_id);
            if where_sel.is_empty() {
                hash_selector
            } else {
                format!("{where_sel}{hash_selector}")
            }
        };

        let scopes: Vec<String> = options
            .scope
            .iter()
            .map(|v| v.as_ref().trim().to_string())
            .filter(|v| !v.is_empty())
            .collect();

        let selector = if scopes.is_empty() {
            base_selector
        } else {
            scopes
                .iter()
                .map(|scope| format!("{base_selector}{}", super::hash::class_selector(scope)))
                .collect::<Vec<_>>()
                .join(", ")
        };

        let mut body = String::new();
        for (key, value) in css_vars {
            let key = key.trim();
            let value = value.trim();
            if key.is_empty() || value.is_empty() {
                continue;
            }
            body.push_str(key);
            body.push(':');
            body.push_str(value);
            body.push(';');
        }

        if body.is_empty() {
            return String::new();
        }

        format!("{selector}{{{body}}}")
    }

    pub fn transform_token(
        token: &CssVarTokenMap,
        theme_key: &str,
        cfg: &CssVarTransformCfg,
    ) -> (CssVarMergedTokenMap, String) {
        let prefix = cfg
            .prefix
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty());
        let mut css_vars = BTreeMap::new();
        let mut merged = BTreeMap::new();

        for (key, value) in token {
            if key.trim().is_empty() {
                continue;
            }
            if cfg.preserve.contains(key) {
                merged.insert(key.clone(), value.to_string());
                continue;
            }
            if cfg.ignore.contains(key) {
                continue;
            }

            let css_var_name = Self::token_to_css_var(key.as_str(), prefix);
            let css_var_value = match value {
                CssVarTokenValue::Number(num) => {
                    if cfg.unitless.contains(key) {
                        super::util::format_css_number(*num)
                    } else {
                        format!("{}px", super::util::format_css_number(*num))
                    }
                }
                CssVarTokenValue::Text(text) => text.to_string(),
            };

            css_vars.insert(css_var_name.clone(), css_var_value);
            merged.insert(key.clone(), format!("var({css_var_name})"));
        }

        let css = Self::serialize_css_vars(
            &css_vars,
            theme_key,
            &CssVarSerializeCfg {
                scope: cfg.scope.clone(),
                hash_class: cfg.hash_class.clone(),
                hash_priority: cfg.hash_priority,
            },
        );

        (merged, css)
    }

    pub fn register_css_vars(
        config: &CssVarRegisterInput,
        token: &CssVarTokenMap,
    ) -> CssVarRegisterOutput {
        let transform_cfg = CssVarTransformCfg {
            prefix: config.prefix.clone(),
            ignore: config.ignore.clone(),
            unitless: config.unitless.clone(),
            preserve: config.preserve.clone(),
            scope: config.scope.clone(),
            hash_class: config.hash_class.clone(),
            hash_priority: config.hash_priority,
        };

        let (merged_token, css_vars_css) =
            Self::transform_token(token, config.key.as_ref(), &transform_cfg);

        let cfg = config_store::get();

        let scope_key = config
            .scope
            .iter()
            .map(|v| v.as_ref().trim())
            .filter(|v| !v.is_empty())
            .collect::<Vec<_>>()
            .join("@@");

        let mut style_path = Vec::<String>::with_capacity(config.path.len() + 4);
        style_path.extend(
            config
                .path
                .iter()
                .map(|v| v.as_ref().trim().to_string())
                .filter(|v| !v.is_empty()),
        );
        style_path.push(config.key.as_ref().trim().to_string());
        style_path.push(scope_key);
        style_path.push(
            config
                .token_hash
                .as_deref()
                .unwrap_or("")
                .trim()
                .to_string(),
        );

        let style_id = config
            .style_id
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(ToString::to_string)
            .unwrap_or_else(|| hash::unique_hash(&cfg, &style_path, css_vars_css.as_str()));

        let mut registration = None;
        let mut hash_class = config.hash_class.as_ref().map(|v| v.to_string());

        if !css_vars_css.trim().is_empty() {
            let mut input = CssInJsStyleInput::new(
                Arc::<str>::from(style_id.clone()),
                Arc::<str>::from(css_vars_css.clone()),
            );
            input.identity_scope = Some(Arc::<str>::from(style_path.join("|")));
            input.token_hash = config.token_hash.clone();
            input.nonce = config.nonce.clone();
            input.layer = config.layer.clone();
            input.hash_class = config.hash_class.clone();
            input.hash_priority = config.hash_priority;
            input.css_var_key = Some(config.key.clone());

            registration = CssInJs::register(input);
            if let Some(reg) = registration.as_ref() {
                hash_class = Some(reg.hash_class.clone());
            }
        }

        CssVarRegisterOutput {
            merged_token,
            css_vars_css,
            style_id,
            css_var_key: config.key.as_ref().to_string(),
            hash_class,
            registration,
        }
    }

    #[inline]
    fn where_selector(hash_class: Option<&str>, hash_priority: HashPriority) -> String {
        let Some(hash_class) = hash_class.map(str::trim).filter(|v| !v.is_empty()) else {
            return String::new();
        };
        hash::scoped_hash_selector(hash_class, hash_priority)
    }
}
