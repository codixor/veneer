//! Object style representation and parser.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use super::CssInJs;
use super::CssInJsLease;
use super::CssInJsLifecycleCfg;
use super::CssInJsRegistration;
use super::CssInJsStyleInput;
use super::config::store as config_store;
use super::hash;
use crate::style_provider::HashPriority;

// ----------------------------------------------------------------------------
// Core types
// ----------------------------------------------------------------------------

pub type CssObject = BTreeMap<String, CssInterpolation>;
pub type CssTransformer = Arc<dyn Fn(CssObject) -> CssObject + Send + Sync + 'static>;
pub type CssLinter = Arc<dyn Fn(&str, &str, &CssLintContext) + Send + Sync + 'static>;

#[derive(Clone, Debug, PartialEq)]
pub struct CssKeyframes {
    pub name: Arc<str>,
    pub style: Box<CssInterpolation>,
}

impl CssKeyframes {
    #[inline]
    #[must_use]
    pub fn new(name: impl Into<Arc<str>>, style: CssInterpolation) -> Self {
        Self {
            name: name.into(),
            style: Box::new(style),
        }
    }

    #[inline]
    #[must_use]
    pub fn get_name(&self, hash_id: Option<&str>) -> String {
        let base = self.name.as_ref().trim();
        if let Some(hash_id) = hash_id.map(str::trim).filter(|v| !v.is_empty()) {
            format!("{base}-{hash_id}")
        } else {
            base.to_string()
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum CssInterpolation {
    Null,
    Bool(bool),
    Number(f64),
    Str(Arc<str>),
    Object(CssObject),
    Array(Vec<CssInterpolation>),
    Keyframes(CssKeyframes),
}

impl CssInterpolation {
    #[inline]
    #[must_use]
    pub fn object(
        entries: impl IntoIterator<Item = (impl Into<String>, CssInterpolation)>,
    ) -> Self {
        let mut out = CssObject::new();
        for (k, v) in entries {
            out.insert(k.into(), v);
        }
        Self::Object(out)
    }
}

macro_rules! impl_from {
    ($($t:ty),*) => {
        $(
            impl From<$t> for CssInterpolation {
                #[inline]
                fn from(v: $t) -> Self {
                    Self::Str(Arc::<str>::from(v))
                }
            }
        )*
    };
}
impl_from!(&str, String, Arc<str>);

macro_rules! impl_from_num {
    ($($t:ty),*) => {
        $(
            impl From<$t> for CssInterpolation {
                #[inline]
                fn from(v: $t) -> Self {
                    Self::Number(v as f64)
                }
            }
        )*
    };
}
impl_from_num!(f64, f32, i32, u32);

impl From<bool> for CssInterpolation {
    #[inline]
    fn from(v: bool) -> Self {
        Self::Bool(v)
    }
}
impl From<CssObject> for CssInterpolation {
    #[inline]
    fn from(v: CssObject) -> Self {
        Self::Object(v)
    }
}
impl From<Vec<CssInterpolation>> for CssInterpolation {
    #[inline]
    fn from(v: Vec<CssInterpolation>) -> Self {
        Self::Array(v)
    }
}
impl From<CssKeyframes> for CssInterpolation {
    #[inline]
    fn from(v: CssKeyframes) -> Self {
        Self::Keyframes(v)
    }
}

// ----------------------------------------------------------------------------
// Parser configuration and context
// ----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CssLayerCfg {
    pub name: Arc<str>,
    pub dependencies: Vec<Arc<str>>,
}

impl Default for CssLayerCfg {
    fn default() -> Self {
        Self {
            name: Arc::<str>::from("default"),
            dependencies: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CssLintContext {
    pub path: Option<Arc<str>>,
    pub hash_id: Option<Arc<str>>,
    pub parent_selectors: Vec<Arc<str>>,
}

impl CssLintContext {
    #[inline]
    #[must_use]
    pub fn new(
        path: Option<Arc<str>>,
        hash_id: Option<Arc<str>>,
        parent_selectors: Vec<Arc<str>>,
    ) -> Self {
        Self {
            path,
            hash_id,
            parent_selectors,
        }
    }
}

#[derive(Clone)]
pub struct CssParseCfg {
    pub hash_id: Option<Arc<str>>,
    pub hash_priority: HashPriority,
    pub layer: Option<CssLayerCfg>,
    pub path: Option<Arc<str>>,
    pub transformers: Vec<CssTransformer>,
    pub linters: Vec<CssLinter>,
    pub unitless: BTreeSet<String>,
}

impl Default for CssParseCfg {
    fn default() -> Self {
        Self {
            hash_id: None,
            hash_priority: HashPriority::Low,
            layer: None,
            path: None,
            transformers: Vec::new(),
            linters: Vec::new(),
            unitless: BTreeSet::new(),
        }
    }
}

impl std::fmt::Debug for CssParseCfg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CssParseCfg")
            .field("hash_id", &self.hash_id)
            .field("hash_priority", &self.hash_priority)
            .field("layer", &self.layer)
            .field("path", &self.path)
            .field("transformers", &self.transformers.len())
            .field("linters", &self.linters.len())
            .field("unitless", &self.unitless)
            .finish()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CssParseInfo {
    pub root: bool,
    pub inject_hash: bool,
    pub parent_selectors: Vec<Arc<str>>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CssParseOutput {
    pub parsed_css: String,
    pub effect_style: BTreeMap<String, String>,
}

// ----------------------------------------------------------------------------
// Style register input
// ----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StyleRegisterInput {
    pub path: Vec<Arc<str>>,
    pub style_id: Option<Arc<str>>,
    pub order: i32,
    pub token_hash: Option<Arc<str>>,
    pub hashed: Option<bool>,
    pub css_var_key: Option<Arc<str>>,
    pub algorithm: Option<Arc<str>>,
    pub theme_scope: Option<Arc<str>>,
    pub layer: Option<Arc<str>>,
    pub nonce: Option<Arc<str>>,
    pub hash_class: Option<Arc<str>>,
    pub hash_priority: HashPriority,
}

impl Default for StyleRegisterInput {
    fn default() -> Self {
        Self {
            path: Vec::new(),
            style_id: None,
            order: 0,
            token_hash: None,
            hashed: None,
            css_var_key: None,
            algorithm: None,
            theme_scope: None,
            layer: None,
            nonce: None,
            hash_class: None,
            hash_priority: HashPriority::Low,
        }
    }
}

// ----------------------------------------------------------------------------
// Registration output
// ----------------------------------------------------------------------------

#[derive(Debug)]
pub struct StyleObjectRegistration {
    pub main: Option<CssInJsRegistration>,
    pub effects: Vec<CssInJsRegistration>,
    pub parsed: CssParseOutput,
    pub lease: Option<CssInJsLease>,
}

// ----------------------------------------------------------------------------
// Parser implementation (simplified – full version available in source)
// ----------------------------------------------------------------------------

pub(crate) struct CssParser;

impl CssParser {
    pub fn register_style_object_with_path(
        info: &StyleRegisterInput,
        parse_cfg: &CssParseCfg,
        interpolation: CssInterpolation,
        lifecycle: Option<CssInJsLifecycleCfg>,
    ) -> StyleObjectRegistration {
        let parsed = Self::parse_style(interpolation, parse_cfg, None);

        let mut effects = Vec::<CssInJsRegistration>::new();
        let mut lease_keys = Vec::<String>::new();

        for (effect_key, effect_css) in &parsed.effect_style {
            if effect_css.trim().is_empty() {
                continue;
            }

            let mut effect_path = info.path.clone();
            effect_path.push(Arc::<str>::from("effect"));
            effect_path.push(Arc::<str>::from(effect_key.as_str()));

            let mut effect_info = info.clone();
            effect_info.path = effect_path;
            effect_info.style_id = None;
            effect_info.layer = None;

            let reg = Self::register_style_with_path(&effect_info, effect_css);

            if let Some(reg) = reg {
                lease_keys.push(reg.cache_key.clone());
                effects.push(reg);
            }
        }

        let main = if parsed.parsed_css.trim().is_empty() {
            None
        } else {
            Self::register_style_with_path(info, parsed.parsed_css.as_str())
        };

        if let Some(main_reg) = main.as_ref() {
            lease_keys.push(main_reg.cache_key.clone());
        }

        let lease = if lifecycle.unwrap_or_default().auto_clear && !lease_keys.is_empty() {
            Some(CssInJsLease::new(lease_keys))
        } else {
            None
        };

        StyleObjectRegistration {
            main,
            effects,
            parsed,
            lease,
        }
    }

    pub fn register_style_with_path(
        info: &StyleRegisterInput,
        style_css: impl AsRef<str>,
    ) -> Option<CssInJsRegistration> {
        let style_css = style_css.as_ref().trim();
        if style_css.is_empty() {
            return None;
        }

        let cfg = config_store::get();

        let mut style_path = Vec::<String>::with_capacity(info.path.len() + 2);
        style_path.push(info.token_hash.as_deref().unwrap_or("").trim().to_string());
        style_path.extend(
            info.path
                .iter()
                .map(|v| v.as_ref().trim().to_string())
                .filter(|v| !v.is_empty()),
        );

        let style_id = info
            .style_id
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(ToString::to_string)
            .unwrap_or_else(|| hash::unique_hash(&cfg, &style_path, style_css));

        let mut input = CssInJsStyleInput::new(
            Arc::<str>::from(style_id),
            Arc::<str>::from(style_css.to_string()),
        );
        input.identity_scope = Some(Arc::<str>::from(style_path.join("|")));
        input.order = info.order;
        input.token_hash = info.token_hash.clone();
        input.hashed = info.hashed;
        input.css_var_key = info.css_var_key.clone();
        input.algorithm = info.algorithm.clone();
        input.theme_scope = info.theme_scope.clone();
        input.layer = info.layer.clone();
        input.nonce = info.nonce.clone();
        input.hash_class = info.hash_class.clone();
        input.hash_priority = info.hash_priority;

        CssInJs::register(input)
    }

    pub fn parse_style(
        interpolation: CssInterpolation,
        cfg: &CssParseCfg,
        parse_info: Option<CssParseInfo>,
    ) -> CssParseOutput {
        let mut info = parse_info.unwrap_or_default();
        if !info.root {
            info.root = true;
        }

        let mut effect_style = BTreeMap::<String, String>::new();
        let mut parsed_css =
            Self::parse_interpolation(&interpolation, cfg, &info, &mut effect_style);

        if info.root {
            if let Some(layer) = cfg.layer.as_ref() {
                let layer_name = layer.name.as_ref().trim();
                if !layer_name.is_empty() && !parsed_css.trim().is_empty() {
                    parsed_css = format!("@layer {layer_name} {{{parsed_css}}}");
                }
                if !layer_name.is_empty() && !layer.dependencies.is_empty() {
                    let dep_lines = layer
                        .dependencies
                        .iter()
                        .map(|dep| dep.as_ref().trim())
                        .filter(|dep| !dep.is_empty())
                        .map(|dep| format!("@layer {dep}, {layer_name};"))
                        .collect::<Vec<_>>()
                        .join("\n");
                    if !dep_lines.is_empty() {
                        effect_style.insert(format!("@layer {layer_name}"), dep_lines);
                    }
                }
            }

            parsed_css = parsed_css.trim().to_string();
        }

        CssParseOutput {
            parsed_css,
            effect_style,
        }
    }

    fn parse_interpolation(
        interpolation: &CssInterpolation,
        cfg: &CssParseCfg,
        info: &CssParseInfo,
        effect_style: &mut BTreeMap<String, String>,
    ) -> String {
        match interpolation {
            CssInterpolation::Null | CssInterpolation::Bool(false) => String::new(),
            CssInterpolation::Bool(true) => String::new(),
            CssInterpolation::Str(text) => {
                if info.root {
                    text.to_string()
                } else {
                    format!("{{{}}}", text)
                }
            }
            CssInterpolation::Number(num) => {
                let n = super::util::format_css_number(*num);
                if info.root { n } else { format!("{{{n}}}") }
            }
            CssInterpolation::Array(items) => Self::parse_array(items, cfg, info, effect_style),
            CssInterpolation::Object(style_obj) => {
                Self::parse_object(style_obj, cfg, info, effect_style)
            }
            CssInterpolation::Keyframes(kf) => {
                let name = kf.get_name(cfg.hash_id.as_deref());
                if !effect_style.contains_key(name.as_str()) {
                    let css = Self::parse_interpolation(
                        kf.style.as_ref(),
                        cfg,
                        &CssParseInfo {
                            root: false,
                            inject_hash: false,
                            parent_selectors: info.parent_selectors.clone(),
                        },
                        effect_style,
                    );
                    if !css.trim().is_empty() {
                        effect_style.insert(name.clone(), format!("@keyframes {name}{css}"));
                    }
                }
                if info.root {
                    String::new()
                } else {
                    format!("{{animation-name:{name};}}")
                }
            }
        }
    }

    fn parse_array(
        items: &[CssInterpolation],
        cfg: &CssParseCfg,
        info: &CssParseInfo,
        effect_style: &mut BTreeMap<String, String>,
    ) -> String {
        let mut style = String::new();
        for item in items {
            let next = Self::parse_interpolation(item, cfg, info, effect_style);
            if next.is_empty() {
                continue;
            }
            style.push_str(next.as_str());
            if info.root && !style.ends_with('\n') {
                style.push('\n');
            }
        }
        style
    }

    fn parse_object(
        style_obj: &CssObject,
        cfg: &CssParseCfg,
        info: &CssParseInfo,
        effect_style: &mut BTreeMap<String, String>,
    ) -> String {
        let mut merged = style_obj.clone();
        for transformer in &cfg.transformers {
            merged = transformer(merged);
        }

        let hash_id = cfg
            .hash_id
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty());
        let mut properties = String::new();
        let mut nested = String::new();

        for (key, value) in merged {
            let key_trimmed = key.trim();
            if key_trimmed.is_empty() {
                continue;
            }

            if Self::is_nested_style(key_trimmed, &value) {
                let mut merged_key = key_trimmed.to_string();
                let mut sub_inject_hash = false;
                let mut next_root = false;

                if (info.root || info.inject_hash) && hash_id.is_some() {
                    if merged_key.starts_with('@') {
                        sub_inject_hash = true;
                    } else if merged_key == "&" {
                        merged_key = Self::inject_selector_hash("", hash_id, cfg.hash_priority);
                    } else {
                        merged_key = Self::inject_selector_hash(
                            merged_key.as_str(),
                            hash_id,
                            cfg.hash_priority,
                        );
                    }
                } else if info.root
                    && hash_id.is_none()
                    && (merged_key == "&" || merged_key.is_empty())
                {
                    merged_key.clear();
                    next_root = true;
                }

                if !merged_key.starts_with('@') {
                    merged_key =
                        super::util::escape_selector_digit_class_prefixes(merged_key.as_str());
                }

                let mut next_parent = info.parent_selectors.clone();
                if !merged_key.is_empty() {
                    next_parent.push(Arc::<str>::from(merged_key.as_str()));
                }

                let child = if merged_key.starts_with('@') {
                    let child = Self::parse_interpolation(
                        &value,
                        cfg,
                        &CssParseInfo {
                            root: true,
                            inject_hash: sub_inject_hash,
                            parent_selectors: info.parent_selectors.clone(),
                        },
                        effect_style,
                    );
                    if child.trim().is_empty() {
                        String::new()
                    } else {
                        format!("{merged_key}{{{}}}", child.trim())
                    }
                } else {
                    Self::parse_interpolation(
                        &value,
                        cfg,
                        &CssParseInfo {
                            root: next_root,
                            inject_hash: sub_inject_hash,
                            parent_selectors: next_parent,
                        },
                        effect_style,
                    )
                };

                if !child.is_empty() {
                    nested.push_str(child.as_str());
                }
                continue;
            }

            Self::append_property(
                &mut properties,
                key_trimmed,
                &value,
                cfg,
                &CssLintContext::new(
                    cfg.path.clone(),
                    cfg.hash_id.clone(),
                    info.parent_selectors.clone(),
                ),
                effect_style,
            );
        }

        let mut out = String::new();

        if !properties.is_empty() {
            if info.parent_selectors.is_empty() {
                if info.root {
                    out.push_str(properties.as_str());
                } else {
                    out.push('{');
                    out.push_str(properties.as_str());
                    out.push('}');
                }
            } else {
                out.push_str(Self::resolve_parent_selector_path(&info.parent_selectors).as_str());
                out.push('{');
                out.push_str(properties.as_str());
                out.push('}');
            }
        }

        out.push_str(nested.as_str());
        out
    }

    fn append_property(
        out: &mut String,
        key: &str,
        value: &CssInterpolation,
        cfg: &CssParseCfg,
        lint_ctx: &CssLintContext,
        effect_style: &mut BTreeMap<String, String>,
    ) {
        let runtime = config_store::get();
        let style_names = Self::key_to_css_properties(key, &runtime);
        if style_names.is_empty() {
            return;
        }

        match value {
            CssInterpolation::Null | CssInterpolation::Bool(false) => {}
            CssInterpolation::Bool(true) => Self::append_style_values(
                out,
                &style_names,
                "true",
                cfg,
                lint_ctx,
                runtime.normalize_explicit_custom_props,
            ),
            CssInterpolation::Number(num) => Self::append_numeric_style_values(
                out,
                &style_names,
                *num,
                cfg,
                lint_ctx,
                runtime.normalize_explicit_custom_props,
            ),
            CssInterpolation::Str(text) => {
                Self::append_style_values(
                    out,
                    &style_names,
                    text.as_ref(),
                    cfg,
                    lint_ctx,
                    runtime.normalize_explicit_custom_props,
                );
            }
            CssInterpolation::Array(values) => {
                for entry in values {
                    match entry {
                        CssInterpolation::Number(num) => Self::append_numeric_style_values(
                            out,
                            &style_names,
                            *num,
                            cfg,
                            lint_ctx,
                            runtime.normalize_explicit_custom_props,
                        ),
                        CssInterpolation::Str(text) => {
                            Self::append_style_values(
                                out,
                                &style_names,
                                text.as_ref(),
                                cfg,
                                lint_ctx,
                                runtime.normalize_explicit_custom_props,
                            );
                        }
                        CssInterpolation::Keyframes(kf)
                            if key.eq_ignore_ascii_case("animationName") =>
                        {
                            let name = kf.get_name(cfg.hash_id.as_deref());
                            if !effect_style.contains_key(name.as_str()) {
                                let css = Self::parse_interpolation(
                                    kf.style.as_ref(),
                                    cfg,
                                    &CssParseInfo {
                                        root: false,
                                        inject_hash: false,
                                        parent_selectors: lint_ctx.parent_selectors.clone(),
                                    },
                                    effect_style,
                                );
                                if !css.trim().is_empty() {
                                    effect_style
                                        .insert(name.clone(), format!("@keyframes {name}{css}"));
                                }
                            }
                            Self::append_style_values(
                                out,
                                &style_names,
                                &name,
                                cfg,
                                lint_ctx,
                                runtime.normalize_explicit_custom_props,
                            );
                        }
                        _ => {}
                    }
                }
            }
            CssInterpolation::Keyframes(kf) => {
                if key.eq_ignore_ascii_case("animationName") {
                    let name = kf.get_name(cfg.hash_id.as_deref());
                    if !effect_style.contains_key(name.as_str()) {
                        let css = Self::parse_interpolation(
                            kf.style.as_ref(),
                            cfg,
                            &CssParseInfo {
                                root: false,
                                inject_hash: false,
                                parent_selectors: lint_ctx.parent_selectors.clone(),
                            },
                            effect_style,
                        );
                        if !css.trim().is_empty() {
                            effect_style.insert(name.clone(), format!("@keyframes {name}{css}"));
                        }
                    }
                    Self::append_style_values(
                        out,
                        &style_names,
                        &name,
                        cfg,
                        lint_ctx,
                        runtime.normalize_explicit_custom_props,
                    );
                }
            }
            CssInterpolation::Object(_) => {}
        }
    }

    fn append_style_value(
        out: &mut String,
        style_name: &str,
        style_value: &str,
        cfg: &CssParseCfg,
        lint_ctx: &CssLintContext,
        normalize_explicit_custom_props: bool,
    ) {
        let normalized_value =
            super::util::normalize_var_references(style_value, normalize_explicit_custom_props);
        for linter in &cfg.linters {
            linter(style_name, normalized_value.as_ref(), lint_ctx);
        }
        out.push_str(style_name);
        out.push(':');
        out.push_str(normalized_value.as_ref());
        out.push(';');
    }

    fn append_style_values(
        out: &mut String,
        style_names: &[String],
        style_value: &str,
        cfg: &CssParseCfg,
        lint_ctx: &CssLintContext,
        normalize_explicit_custom_props: bool,
    ) {
        for style_name in style_names {
            Self::append_style_value(
                out,
                style_name,
                style_value,
                cfg,
                lint_ctx,
                normalize_explicit_custom_props,
            );
        }
    }

    fn append_numeric_style_values(
        out: &mut String,
        style_names: &[String],
        value: f64,
        cfg: &CssParseCfg,
        lint_ctx: &CssLintContext,
        normalize_explicit_custom_props: bool,
    ) {
        for style_name in style_names {
            let normalized = Self::normalize_numeric_value(style_name, value, &cfg.unitless);
            Self::append_style_value(
                out,
                style_name,
                &normalized,
                cfg,
                lint_ctx,
                normalize_explicit_custom_props,
            );
        }
    }

    #[inline]
    fn is_nested_style(key: &str, value: &CssInterpolation) -> bool {
        match value {
            CssInterpolation::Object(_) => true,
            CssInterpolation::Array(entries) => entries
                .iter()
                .any(|e| matches!(e, CssInterpolation::Object(_))),
            _ => {
                key.starts_with('@')
                    || key == "&"
                    || key.starts_with(':')
                    || key.contains('&')
                    || key.contains(',')
                    || key.contains('.')
                    || key.contains('#')
                    || key.contains(' ')
            }
        }
    }

    #[inline]
    fn key_to_css_properties(key: &str, runtime: &super::config::CssInJsConfig) -> Vec<String> {
        let key = key.trim();
        if key.is_empty() {
            return Vec::new();
        }

        let needs_normalize = super::util::should_normalize_property_key_fast(
            key,
            runtime.normalize_explicit_custom_props,
        );

        let normalized = if runtime.vendor_compat.normalize_vendor_properties {
            if needs_normalize || (key.starts_with('-') && !key.starts_with("--")) {
                super::util::normalize_vendor_property_key(key)
            } else {
                key.to_string()
            }
        } else if needs_normalize {
            super::util::normalize_css_property_name(key, runtime.normalize_explicit_custom_props)
        } else {
            key.to_string()
        };

        let prop = if normalized.starts_with('-') || normalized.starts_with("--") {
            normalized
        } else if needs_normalize {
            super::util::normalize_css_property_name(key, runtime.normalize_explicit_custom_props)
        } else {
            normalized
        };

        super::util::expand_vendor_compat_properties(prop.as_str(), &runtime.vendor_compat)
    }

    #[inline]
    fn normalize_numeric_value(property: &str, value: f64, unitless: &BTreeSet<String>) -> String {
        if value == 0.0 {
            return "0".to_string();
        }
        let stripped = super::util::strip_vendor_prefix(property);
        if unitless.contains(property)
            || unitless.contains(stripped)
            || unitless.contains(&super::util::normalize_css_ident(property))
            || unitless.contains(&super::util::normalize_css_ident(stripped))
        {
            super::util::format_css_number(value)
        } else {
            format!("{}px", super::util::format_css_number(value))
        }
    }

    #[inline]
    fn inject_selector_hash(
        selector: &str,
        hash_id: Option<&str>,
        hash_priority: HashPriority,
    ) -> String {
        let Some(hash_id) = hash_id else {
            return selector.to_string();
        };
        let hash_id = hash_id.trim();
        if hash_id.is_empty() {
            return selector.to_string();
        }

        super::hash::inject_hash_into_selector(selector, hash_id, hash_priority)
    }

    fn resolve_parent_selector_path(parent_selectors: &[Arc<str>]) -> String {
        let mut out = String::new();
        for selector in parent_selectors {
            let selector = selector.as_ref();
            if out.is_empty() {
                out = selector.to_string();
            } else if selector.contains('&') {
                out = selector.replace('&', out.as_str());
            } else {
                out.push(' ');
                out.push_str(selector);
            }
        }
        out
    }
}
