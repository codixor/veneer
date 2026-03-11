use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, OnceLock, RwLock};

use crate::backend as style_backend;
use crate::engine::CssVarEngine;
use crate::{
    BundleBuildOptions, BundleExtractCache, BundleOutput, CssInJsStyleInput, CssVarRegisterInput,
    CssVarRegisterOutput, CssVarTokenMap, HashPriority, build_bundle, build_bundle_once_with_cache,
};

pub use crate::engine::style_provider::StyleProviderProps;
pub use crate::engine::{
    CssInterpolation as CSSInterpolation, CssLintContext, CssLinter as Linter,
    CssObject as CSSObject, CssParseCfg, CssTransformer as Transformer, StyleRegisterInput,
};

pub type Keyframes = crate::engine::CssKeyframes;
pub type DerivativeFn = Arc<dyn Fn(&CssVarTokenMap) -> CssVarTokenMap + Send + Sync + 'static>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StyleContext {
    pub hash_priority: HashPriority,
    pub layer: Option<Arc<str>>,
    pub nonce: Option<Arc<str>>,
}

impl Default for StyleContext {
    fn default() -> Self {
        Self {
            hash_priority: HashPriority::Low,
            layer: None,
            nonce: None,
        }
    }
}

#[derive(Clone)]
pub struct Theme {
    name: Arc<str>,
    derivatives: Vec<DerivativeFn>,
}

impl Theme {
    #[must_use]
    pub fn new(derivatives: Vec<DerivativeFn>) -> Self {
        Self {
            name: Arc::<str>::from("theme"),
            derivatives,
        }
    }

    #[must_use]
    pub fn named(name: impl Into<Arc<str>>, derivatives: Vec<DerivativeFn>) -> Self {
        Self {
            name: name.into(),
            derivatives,
        }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    #[must_use]
    pub fn derivative_count(&self) -> usize {
        self.derivatives.len()
    }

    #[must_use]
    pub fn apply(&self, token: &CssVarTokenMap) -> CssVarTokenMap {
        let mut next = token.clone();
        for derivative in &self.derivatives {
            next = derivative(&next);
        }
        next
    }
}

#[must_use]
pub fn create_theme(derivatives: Vec<DerivativeFn>) -> Theme {
    Theme::new(derivatives)
}

#[must_use]
pub fn get_computed_token(
    origin_token: &CssVarTokenMap,
    override_token: &CssVarTokenMap,
    theme: &Theme,
    format_token: Option<DerivativeFn>,
) -> CssVarTokenMap {
    let mut merged = theme.apply(origin_token);
    merged.extend(override_token.clone());
    if let Some(format_token) = format_token {
        merged = format_token(&merged);
    }
    merged
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StyleCache {
    inner: BundleExtractCache,
}

impl StyleCache {
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    #[must_use]
    pub fn remove(&mut self, cache_id: &str) -> bool {
        self.inner.remove(cache_id)
    }

    #[must_use]
    pub fn entity_count(&self) -> usize {
        self.inner.entity_count()
    }
}

#[must_use]
pub fn create_cache() -> StyleCache {
    StyleCache::default()
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExtractStyleOptions {
    pub once: bool,
    pub bundle: BundleBuildOptions,
}

#[must_use]
pub fn extract_style_output(
    cache: &mut StyleCache,
    cache_id: &str,
    options: ExtractStyleOptions,
) -> BundleOutput {
    if options.once {
        build_bundle_once_with_cache(&mut cache.inner, cache_id, options.bundle)
    } else {
        build_bundle(options.bundle)
    }
}

#[must_use]
pub fn extract_style(
    cache: &mut StyleCache,
    cache_id: &str,
    options: ExtractStyleOptions,
) -> String {
    extract_style_output(cache, cache_id, options).css
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StyleRegisterResult {
    pub cache_key: String,
    pub hash_class: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UseStyleRegisterOptions {
    pub style_id: Arc<str>,
    pub css: Arc<str>,
    pub identity_scope: Option<Arc<str>>,
    pub order: i32,
    pub token_hash: Option<Arc<str>>,
    pub hashed: Option<bool>,
    pub css_var_key: Option<Arc<str>>,
    pub algorithm: Option<Arc<str>>,
    pub theme_scope: Option<Arc<str>>,
    pub nonce: Option<Arc<str>>,
    pub layer: Option<Arc<str>>,
    pub hash_class: Option<Arc<str>>,
    pub hash_priority: HashPriority,
    pub context: Option<StyleContext>,
}

impl Default for UseStyleRegisterOptions {
    fn default() -> Self {
        Self {
            style_id: Arc::<str>::from(""),
            css: Arc::<str>::from(""),
            identity_scope: None,
            order: 0,
            token_hash: None,
            hashed: None,
            css_var_key: None,
            algorithm: None,
            theme_scope: None,
            nonce: None,
            layer: None,
            hash_class: None,
            hash_priority: HashPriority::Low,
            context: None,
        }
    }
}

#[must_use]
pub fn use_style_register(options: UseStyleRegisterOptions) -> Option<StyleRegisterResult> {
    let mut input = CssInJsStyleInput {
        style_id: options.style_id,
        css: options.css,
        identity_scope: options.identity_scope,
        order: options.order,
        token_hash: options.token_hash,
        hashed: options.hashed,
        css_var_key: options.css_var_key,
        algorithm: options.algorithm,
        theme_scope: options.theme_scope,
        nonce: options.nonce,
        layer: options.layer,
        hash_class: options.hash_class,
        hash_priority: options.hash_priority,
        rewrite: None,
    };

    if let Some(context) = options.context {
        if input.layer.is_none() {
            input.layer = context.layer;
        }
        if input.nonce.is_none() {
            input.nonce = context.nonce;
        }
        input.hash_priority = context.hash_priority;
    }

    let registration = style_backend::CssInJs::register(input)?;
    Some(StyleRegisterResult {
        cache_key: registration.cache_key,
        hash_class: registration.hash_class,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CacheTokenOptions {
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
    pub context: Option<StyleContext>,
}

impl Default for CacheTokenOptions {
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
            context: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CacheTokenResult {
    pub themed_token: CssVarTokenMap,
    pub output: CssVarRegisterOutput,
}

#[must_use]
pub fn use_cache_token(
    theme: &Theme,
    token: &CssVarTokenMap,
    options: CacheTokenOptions,
) -> CacheTokenResult {
    let themed_token = theme.apply(token);

    let mut cfg = CssVarRegisterInput {
        path: options.path,
        key: options.key,
        style_id: options.style_id,
        prefix: options.prefix,
        unitless: options.unitless,
        ignore: options.ignore,
        preserve: options.preserve,
        scope: options.scope,
        token_hash: options.token_hash,
        hash_class: options.hash_class,
        hash_priority: options.hash_priority,
        layer: options.layer,
        nonce: options.nonce,
    };

    if let Some(context) = options.context {
        if cfg.layer.is_none() {
            cfg.layer = context.layer;
        }
        if cfg.nonce.is_none() {
            cfg.nonce = context.nonce;
        }
        cfg.hash_priority = context.hash_priority;
    }

    let output = style_backend::CssInJs::register_css_vars(&cfg, &themed_token);
    CacheTokenResult {
        themed_token,
        output,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UseCssVarRegisterOptions {
    pub path: Vec<Arc<str>>,
    pub key: Arc<str>,
    pub style_id: Option<Arc<str>>,
    pub prefix: Option<Arc<str>>,
    pub unitless: BTreeSet<String>,
    pub ignore: BTreeSet<String>,
    pub preserve: BTreeSet<String>,
    pub scope: Vec<Arc<str>>,
    pub token_hash: Option<Arc<str>>,
    pub hash_id: Option<Arc<str>>,
    pub hash_priority: HashPriority,
    pub layer: Option<Arc<str>>,
    pub nonce: Option<Arc<str>>,
    pub context: Option<StyleContext>,
}

impl Default for UseCssVarRegisterOptions {
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
            hash_id: None,
            hash_priority: HashPriority::Low,
            layer: None,
            nonce: None,
            context: None,
        }
    }
}

#[must_use]
pub fn use_css_var_register(
    token: &CssVarTokenMap,
    options: UseCssVarRegisterOptions,
) -> CssVarRegisterOutput {
    let mut cfg = CssVarRegisterInput {
        path: options.path,
        key: options.key,
        style_id: options.style_id,
        prefix: options.prefix,
        unitless: options.unitless,
        ignore: options.ignore,
        preserve: options.preserve,
        scope: options.scope,
        token_hash: options.token_hash,
        hash_class: options.hash_id,
        hash_priority: options.hash_priority,
        layer: options.layer,
        nonce: options.nonce,
    };

    if let Some(context) = options.context {
        if cfg.layer.is_none() {
            cfg.layer = context.layer;
        }
        if cfg.nonce.is_none() {
            cfg.nonce = context.nonce;
        }
        cfg.hash_priority = context.hash_priority;
    }

    style_backend::CssInJs::register_css_vars(&cfg, token)
}

#[must_use]
pub fn token_to_css_var(token: &str, prefix: Option<&str>) -> String {
    CssVarEngine::token_to_css_var(token, prefix)
}

#[must_use]
pub fn merge_token<T: Clone>(
    base: &BTreeMap<String, T>,
    overlay: &BTreeMap<String, T>,
) -> BTreeMap<String, T> {
    let mut merged = base.clone();
    merged.extend(overlay.clone());
    merged
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalcMode {
    Css,
    Js,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CalcOperand {
    Number(f64),
    Text(Arc<str>),
    Calc(AbstractCalculator),
}

impl From<AbstractCalculator> for CalcOperand {
    fn from(value: AbstractCalculator) -> Self {
        Self::Calc(value)
    }
}

impl From<f64> for CalcOperand {
    fn from(value: f64) -> Self {
        Self::Number(value)
    }
}

impl From<f32> for CalcOperand {
    fn from(value: f32) -> Self {
        Self::Number(f64::from(value))
    }
}

impl From<i32> for CalcOperand {
    fn from(value: i32) -> Self {
        Self::Number(f64::from(value))
    }
}

impl From<u32> for CalcOperand {
    fn from(value: u32) -> Self {
        Self::Number(f64::from(value))
    }
}

impl From<String> for CalcOperand {
    fn from(value: String) -> Self {
        Self::Text(Arc::<str>::from(value))
    }
}

impl From<&str> for CalcOperand {
    fn from(value: &str) -> Self {
        Self::Text(Arc::<str>::from(value))
    }
}

impl From<Arc<str>> for CalcOperand {
    fn from(value: Arc<str>) -> Self {
        Self::Text(value)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AbstractCalculator {
    mode: CalcMode,
    css_expr: String,
    numeric_value: Option<f64>,
    unitless_css_var: Arc<BTreeSet<String>>,
}

impl AbstractCalculator {
    #[must_use]
    pub fn new(
        mode: CalcMode,
        value: CalcOperand,
        unitless_css_var: Arc<BTreeSet<String>>,
    ) -> Self {
        let (css_expr, numeric_value) = operand_parts(&value);
        Self {
            mode,
            css_expr,
            numeric_value,
            unitless_css_var,
        }
    }

    #[allow(clippy::should_implement_trait)]
    #[must_use]
    pub fn add(self, rhs: impl Into<CalcOperand>) -> Self {
        self.combine("+", rhs.into(), |lhs, rhs| lhs + rhs)
    }

    #[allow(clippy::should_implement_trait)]
    #[must_use]
    pub fn sub(self, rhs: impl Into<CalcOperand>) -> Self {
        self.combine("-", rhs.into(), |lhs, rhs| lhs - rhs)
    }

    #[allow(clippy::should_implement_trait)]
    #[must_use]
    pub fn mul(self, rhs: impl Into<CalcOperand>) -> Self {
        self.combine("*", rhs.into(), |lhs, rhs| lhs * rhs)
    }

    #[allow(clippy::should_implement_trait)]
    #[must_use]
    pub fn div(self, rhs: impl Into<CalcOperand>) -> Self {
        self.combine("/", rhs.into(), |lhs, rhs| lhs / rhs)
    }

    #[must_use]
    pub fn max(self, rhs: impl Into<CalcOperand>) -> Self {
        self.combine_function("max", rhs.into(), f64::max)
    }

    #[must_use]
    pub fn min(self, rhs: impl Into<CalcOperand>) -> Self {
        self.combine_function("min", rhs.into(), f64::min)
    }

    #[must_use]
    pub fn css(&self) -> &str {
        self.css_expr.as_str()
    }

    #[must_use]
    pub fn numeric(&self) -> Option<f64> {
        self.numeric_value
    }

    #[must_use]
    pub fn unitless_css_var(&self) -> &BTreeSet<String> {
        self.unitless_css_var.as_ref()
    }

    fn combine(self, operator: &str, rhs: CalcOperand, numeric: impl Fn(f64, f64) -> f64) -> Self {
        let (rhs_css, rhs_numeric) = operand_parts(&rhs);
        let numeric_value = match (self.mode, self.numeric_value, rhs_numeric) {
            (CalcMode::Js, Some(lhs), Some(rhs)) => Some(numeric(lhs, rhs)),
            _ => None,
        };

        Self {
            mode: self.mode,
            css_expr: format!("calc({} {} {})", self.css_expr, operator, rhs_css),
            numeric_value,
            unitless_css_var: self.unitless_css_var,
        }
    }

    fn combine_function(
        self,
        function_name: &str,
        rhs: CalcOperand,
        numeric: impl Fn(f64, f64) -> f64,
    ) -> Self {
        let (rhs_css, rhs_numeric) = operand_parts(&rhs);
        let numeric_value = match (self.mode, self.numeric_value, rhs_numeric) {
            (CalcMode::Js, Some(lhs), Some(rhs)) => Some(numeric(lhs, rhs)),
            _ => None,
        };

        Self {
            mode: self.mode,
            css_expr: format!("{function_name}({}, {})", self.css_expr, rhs_css),
            numeric_value,
            unitless_css_var: self.unitless_css_var,
        }
    }
}

impl std::fmt::Display for AbstractCalculator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.css())
    }
}

pub fn gen_calc(
    mode: CalcMode,
    unitless_css_var: BTreeSet<String>,
) -> impl Fn(CalcOperand) -> AbstractCalculator {
    let unitless_css_var = Arc::new(unitless_css_var);
    move |value| AbstractCalculator::new(mode, value, Arc::clone(&unitless_css_var))
}

#[must_use]
pub fn auto_prefix_transformer() -> Transformer {
    Arc::new(|css_object| css_object)
}

#[must_use]
pub fn legacy_logical_properties_transformer() -> Transformer {
    Arc::new(|css_object| {
        let mut out = CSSObject::new();

        for (key, value) in css_object {
            let Some(targets) = legacy_logical_targets(key.as_str()) else {
                out.insert(key, value);
                continue;
            };

            let Some((scalar, important)) = scalar_value_parts(&value) else {
                out.insert(key, value);
                continue;
            };

            let values = if targets.no_split {
                vec![scalar.to_css_string()]
            } else {
                split_values(scalar.to_css_string().as_str())
                    .0
                    .into_iter()
                    .map(|value| value.to_string())
                    .collect::<Vec<_>>()
            };

            match targets.names.len() {
                1 => {
                    out.insert(
                        targets.names[0].to_string(),
                        scalar_to_interpolation(
                            scalar.value_at(values.first().map(String::as_str), important),
                        ),
                    );
                }
                2 => {
                    for (index, name) in targets.names.iter().enumerate() {
                        out.insert(
                            (*name).to_string(),
                            scalar_to_interpolation(
                                scalar.value_at(values.get(index).map(String::as_str), important),
                            ),
                        );
                    }
                }
                4 => {
                    for (index, name) in targets.names.iter().enumerate() {
                        let next = values
                            .get(index)
                            .or_else(|| index.checked_sub(2).and_then(|idx| values.get(idx)))
                            .or_else(|| values.first())
                            .map(String::as_str);
                        out.insert(
                            (*name).to_string(),
                            scalar_to_interpolation(scalar.value_at(next, important)),
                        );
                    }
                }
                _ => {
                    out.insert(key, value);
                }
            }
        }

        out
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Px2RemOptions {
    pub root_value: f64,
    pub precision: u8,
    pub media_query: bool,
}

impl Default for Px2RemOptions {
    fn default() -> Self {
        Self {
            root_value: 16.0,
            precision: 5,
            media_query: false,
        }
    }
}

#[must_use]
pub fn px2rem_transformer(options: Px2RemOptions) -> Transformer {
    Arc::new(move |css_object| {
        let mut out = CSSObject::new();

        for (key, value) in css_object {
            let next_key = if options.media_query && key.trim_start().starts_with('@') {
                replace_px_values(key.as_str(), options.root_value, options.precision)
            } else {
                key.clone()
            };

            let next_value = match value {
                CSSInterpolation::Str(text) if text.contains("px") => CSSInterpolation::from(
                    replace_px_values(text.as_ref(), options.root_value, options.precision),
                ),
                CSSInterpolation::Number(number)
                    if number != 0.0 && !is_unitless_property(key.as_str()) =>
                {
                    let px_value = format!("{number}px");
                    CSSInterpolation::from(replace_px_values(
                        px_value.as_str(),
                        options.root_value,
                        options.precision,
                    ))
                }
                other => other,
            };

            out.insert(next_key, next_value);
        }

        out
    })
}

#[must_use]
pub fn logical_properties_linter() -> Linter {
    Arc::new(|key, value, info| {
        let warning = match key {
            "marginLeft"
            | "marginRight"
            | "paddingLeft"
            | "paddingRight"
            | "left"
            | "right"
            | "borderLeft"
            | "borderLeftWidth"
            | "borderLeftStyle"
            | "borderLeftColor"
            | "borderRight"
            | "borderRightWidth"
            | "borderRightStyle"
            | "borderRightColor"
            | "borderTopLeftRadius"
            | "borderTopRightRadius"
            | "borderBottomLeftRadius"
            | "borderBottomRightRadius" => Some(format!(
                "You seem to be using non-logical property '{key}' which is not compatible with RTL mode."
            )),
            "margin" | "padding" | "borderWidth" | "borderStyle" => {
                let parts = value
                    .split_whitespace()
                    .map(str::trim)
                    .filter(|part| !part.is_empty())
                    .collect::<Vec<_>>();
                if parts.len() == 4 && parts[1] != parts[3] {
                    Some(format!(
                        "You seem to be using '{key}' with different left and right values, which is not compatible with RTL mode."
                    ))
                } else {
                    None
                }
            }
            "clear" | "textAlign" if value == "left" || value == "right" => Some(format!(
                "You seem to be using non-logical value '{value}' for {key}, which is not compatible with RTL mode."
            )),
            "borderRadius" => {
                if has_non_logical_border_radius(value) {
                    Some(format!(
                        "You seem to be using non-logical value '{value}' for {key}, which is not compatible with RTL mode."
                    ))
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(warning) = warning {
            push_lint_warning(warning.as_str(), info);
        }
    })
}

#[must_use]
pub fn legacy_not_selector_linter() -> Linter {
    Arc::new(|_key, _value, info| {
        let selector_path = parse_parent_selector_path(info);
        for content in extract_not_selector_contents(selector_path.as_str()) {
            if is_concat_selector(content.as_str()) {
                push_lint_warning(
                    "Concat ':not' selector not supported in legacy browsers.",
                    info,
                );
                break;
            }
        }
    })
}

#[must_use]
pub fn parent_selector_linter() -> Linter {
    Arc::new(|_key, _value, info| {
        if info.parent_selectors.iter().any(|selector| {
            selector
                .split(',')
                .any(|item| item.matches('&').count() > 1)
        }) {
            push_lint_warning("Should not use more than one `&` in a selector.", info);
        }
    })
}

#[must_use]
pub fn nan_linter() -> Linter {
    Arc::new(|key, value, info| {
        if value.contains("NaN") {
            push_lint_warning(
                format!("Unexpected 'NaN' in property '{key}: {value}'.").as_str(),
                info,
            );
        }
    })
}

#[doc(hidden)]
pub fn debug_take_lint_warnings_for_tests() -> Vec<String> {
    match lint_warnings_store().write() {
        Ok(mut warnings) => std::mem::take(&mut *warnings),
        Err(poisoned) => {
            let mut warnings = poisoned.into_inner();
            std::mem::take(&mut *warnings)
        }
    }
}

fn operand_parts(value: &CalcOperand) -> (String, Option<f64>) {
    match value {
        CalcOperand::Number(number) => (number_to_string(*number), Some(*number)),
        CalcOperand::Text(text) => (text.to_string(), text.parse::<f64>().ok()),
        CalcOperand::Calc(calc) => (calc.css().to_string(), calc.numeric()),
    }
}

fn number_to_string(value: f64) -> String {
    if value.fract().abs() <= f64::EPSILON {
        (value as i64).to_string()
    } else {
        value.to_string()
    }
}

fn lint_warnings_store() -> &'static RwLock<Vec<String>> {
    static STORE: OnceLock<RwLock<Vec<String>>> = OnceLock::new();
    STORE.get_or_init(|| RwLock::new(Vec::new()))
}

fn push_lint_warning(message: &str, info: &CssLintContext) {
    let mut rendered = String::from("[Ant Design CSS-in-JS] ");
    if let Some(path) = info
        .path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
    {
        rendered.push_str("Error in ");
        rendered.push_str(path);
        rendered.push_str(": ");
    }
    rendered.push_str(message);

    if !info.parent_selectors.is_empty() {
        rendered.push_str(" Selector: ");
        rendered.push_str(
            &info
                .parent_selectors
                .iter()
                .map(|selector| selector.as_ref())
                .collect::<Vec<_>>()
                .join(" | "),
        );
    }

    match lint_warnings_store().write() {
        Ok(mut warnings) => warnings.push(rendered),
        Err(poisoned) => {
            let mut warnings = poisoned.into_inner();
            warnings.push(rendered);
        }
    }
}

#[derive(Clone, Copy)]
struct LogicalTargets {
    names: &'static [&'static str],
    no_split: bool,
}

fn legacy_logical_targets(key: &str) -> Option<LogicalTargets> {
    let targets = match key {
        "inset" => LogicalTargets {
            names: &["top", "right", "bottom", "left"],
            no_split: false,
        },
        "insetBlock" => LogicalTargets {
            names: &["top", "bottom"],
            no_split: false,
        },
        "insetBlockStart" => LogicalTargets {
            names: &["top"],
            no_split: false,
        },
        "insetBlockEnd" => LogicalTargets {
            names: &["bottom"],
            no_split: false,
        },
        "insetInline" => LogicalTargets {
            names: &["left", "right"],
            no_split: false,
        },
        "insetInlineStart" => LogicalTargets {
            names: &["left"],
            no_split: false,
        },
        "insetInlineEnd" => LogicalTargets {
            names: &["right"],
            no_split: false,
        },
        "marginBlock" => LogicalTargets {
            names: &["marginTop", "marginBottom"],
            no_split: false,
        },
        "marginBlockStart" => LogicalTargets {
            names: &["marginTop"],
            no_split: false,
        },
        "marginBlockEnd" => LogicalTargets {
            names: &["marginBottom"],
            no_split: false,
        },
        "marginInline" => LogicalTargets {
            names: &["marginLeft", "marginRight"],
            no_split: false,
        },
        "marginInlineStart" => LogicalTargets {
            names: &["marginLeft"],
            no_split: false,
        },
        "marginInlineEnd" => LogicalTargets {
            names: &["marginRight"],
            no_split: false,
        },
        "paddingBlock" => LogicalTargets {
            names: &["paddingTop", "paddingBottom"],
            no_split: false,
        },
        "paddingBlockStart" => LogicalTargets {
            names: &["paddingTop"],
            no_split: false,
        },
        "paddingBlockEnd" => LogicalTargets {
            names: &["paddingBottom"],
            no_split: false,
        },
        "paddingInline" => LogicalTargets {
            names: &["paddingLeft", "paddingRight"],
            no_split: false,
        },
        "paddingInlineStart" => LogicalTargets {
            names: &["paddingLeft"],
            no_split: false,
        },
        "paddingInlineEnd" => LogicalTargets {
            names: &["paddingRight"],
            no_split: false,
        },
        "borderBlock" => LogicalTargets {
            names: &["borderTop", "borderBottom"],
            no_split: true,
        },
        "borderBlockStart" => LogicalTargets {
            names: &["borderTop"],
            no_split: true,
        },
        "borderBlockEnd" => LogicalTargets {
            names: &["borderBottom"],
            no_split: true,
        },
        "borderInline" => LogicalTargets {
            names: &["borderLeft", "borderRight"],
            no_split: true,
        },
        "borderInlineStart" => LogicalTargets {
            names: &["borderLeft"],
            no_split: true,
        },
        "borderInlineEnd" => LogicalTargets {
            names: &["borderRight"],
            no_split: true,
        },
        "borderBlockWidth" => LogicalTargets {
            names: &["borderTopWidth", "borderBottomWidth"],
            no_split: false,
        },
        "borderBlockStartWidth" => LogicalTargets {
            names: &["borderTopWidth"],
            no_split: false,
        },
        "borderBlockEndWidth" => LogicalTargets {
            names: &["borderBottomWidth"],
            no_split: false,
        },
        "borderInlineWidth" => LogicalTargets {
            names: &["borderLeftWidth", "borderRightWidth"],
            no_split: false,
        },
        "borderInlineStartWidth" => LogicalTargets {
            names: &["borderLeftWidth"],
            no_split: false,
        },
        "borderInlineEndWidth" => LogicalTargets {
            names: &["borderRightWidth"],
            no_split: false,
        },
        "borderBlockStyle" => LogicalTargets {
            names: &["borderTopStyle", "borderBottomStyle"],
            no_split: false,
        },
        "borderBlockStartStyle" => LogicalTargets {
            names: &["borderTopStyle"],
            no_split: false,
        },
        "borderBlockEndStyle" => LogicalTargets {
            names: &["borderBottomStyle"],
            no_split: false,
        },
        "borderInlineStyle" => LogicalTargets {
            names: &["borderLeftStyle", "borderRightStyle"],
            no_split: false,
        },
        "borderInlineStartStyle" => LogicalTargets {
            names: &["borderLeftStyle"],
            no_split: false,
        },
        "borderInlineEndStyle" => LogicalTargets {
            names: &["borderRightStyle"],
            no_split: false,
        },
        "borderBlockColor" => LogicalTargets {
            names: &["borderTopColor", "borderBottomColor"],
            no_split: false,
        },
        "borderBlockStartColor" => LogicalTargets {
            names: &["borderTopColor"],
            no_split: false,
        },
        "borderBlockEndColor" => LogicalTargets {
            names: &["borderBottomColor"],
            no_split: false,
        },
        "borderInlineColor" => LogicalTargets {
            names: &["borderLeftColor", "borderRightColor"],
            no_split: false,
        },
        "borderInlineStartColor" => LogicalTargets {
            names: &["borderLeftColor"],
            no_split: false,
        },
        "borderInlineEndColor" => LogicalTargets {
            names: &["borderRightColor"],
            no_split: false,
        },
        "borderStartStartRadius" => LogicalTargets {
            names: &["borderTopLeftRadius"],
            no_split: false,
        },
        "borderStartEndRadius" => LogicalTargets {
            names: &["borderTopRightRadius"],
            no_split: false,
        },
        "borderEndStartRadius" => LogicalTargets {
            names: &["borderBottomLeftRadius"],
            no_split: false,
        },
        "borderEndEndRadius" => LogicalTargets {
            names: &["borderBottomRightRadius"],
            no_split: false,
        },
        _ => return None,
    };

    Some(targets)
}

#[derive(Clone)]
enum ScalarValue {
    Number(f64),
    Text(String),
}

impl ScalarValue {
    fn to_css_string(&self) -> String {
        match self {
            Self::Number(number) => number_to_string(*number),
            Self::Text(text) => text.clone(),
        }
    }

    fn value_at(&self, next: Option<&str>, important: bool) -> Self {
        match self {
            Self::Number(number) if !important => Self::Number(*number),
            Self::Number(number) => {
                let mut rendered = number_to_string(*number);
                rendered.push_str(" !important");
                Self::Text(rendered)
            }
            Self::Text(text) => {
                let mut rendered = next.unwrap_or(text.as_str()).to_string();
                if important && !rendered.ends_with("!important") {
                    rendered.push_str(" !important");
                }
                Self::Text(rendered)
            }
        }
    }
}

fn scalar_value_parts(value: &CSSInterpolation) -> Option<(ScalarValue, bool)> {
    match value {
        CSSInterpolation::Number(number) => Some((ScalarValue::Number(*number), false)),
        CSSInterpolation::Str(text) => {
            let (values, important) = split_values(text.as_ref());
            let rendered = values.join(" ");
            Some((ScalarValue::Text(rendered), important))
        }
        _ => None,
    }
}

fn scalar_to_interpolation(value: ScalarValue) -> CSSInterpolation {
    match value {
        ScalarValue::Number(number) => CSSInterpolation::Number(number),
        ScalarValue::Text(text) => CSSInterpolation::from(text),
    }
}

fn split_values(value: &str) -> (Vec<String>, bool) {
    let raw = value.trim();
    let (raw, important) = if let Some(stripped) = raw.strip_suffix("!important") {
        (stripped.trim_end(), true)
    } else {
        (raw, false)
    };

    if raw.is_empty() {
        return (Vec::new(), important);
    }

    let mut out = Vec::<String>::new();
    let mut current = String::new();
    let mut depth = 0i32;

    for ch in raw.chars() {
        match ch {
            '(' => {
                depth += 1;
                current.push(ch);
            }
            ')' => {
                depth -= 1;
                current.push(ch);
            }
            ' ' | '\t' | '\n' if depth == 0 => {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    out.push(trimmed.to_string());
                    current.clear();
                }
            }
            _ => current.push(ch),
        }
    }

    let trimmed = current.trim();
    if !trimmed.is_empty() {
        out.push(trimmed.to_string());
    }

    (out, important)
}

fn replace_px_values(input: &str, root_value: f64, precision: u8) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len() + 16);
    let mut i = 0usize;

    while i < bytes.len() {
        if input[i..].starts_with("url(") || input[i..].starts_with("var(") {
            let start = i;
            i += 4;
            let mut depth = 1i32;
            while i < bytes.len() {
                match bytes[i] {
                    b'(' => depth += 1,
                    b')' => {
                        depth -= 1;
                        if depth == 0 {
                            i += 1;
                            break;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            out.push_str(&input[start..i.min(input.len())]);
            continue;
        }

        let is_numeric_start = bytes[i].is_ascii_digit()
            || bytes[i] == b'.'
            || ((bytes[i] == b'+' || bytes[i] == b'-')
                && bytes.get(i + 1).map(u8::is_ascii_digit).unwrap_or(false));

        if !is_numeric_start {
            out.push(char::from(bytes[i]));
            i += 1;
            continue;
        }

        let start = i;
        if bytes[i] == b'+' || bytes[i] == b'-' {
            i += 1;
        }
        while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
            i += 1;
        }

        if i + 1 < bytes.len() && &input[i..i + 2] == "px" {
            let number = &input[start..i];
            if let Ok(pixels) = number.parse::<f64>()
                && pixels > 1.0
            {
                let rem = to_fixed(pixels / root_value, precision);
                out.push_str(number_to_string(rem).as_str());
                out.push_str("rem");
                i += 2;
                continue;
            }
        }

        out.push_str(&input[start..i]);
    }

    out
}

fn to_fixed(value: f64, precision: u8) -> f64 {
    let multiplier = 10f64.powi(i32::from(precision) + 1);
    let whole = (value * multiplier).floor();
    ((whole / 10.0).round() * 10.0) / multiplier
}

fn is_unitless_property(key: &str) -> bool {
    matches!(
        key,
        "lineHeight"
            | "fontWeight"
            | "fontWeightStrong"
            | "opacity"
            | "zIndex"
            | "zIndexPopup"
            | "zIndexPopupBase"
            | "zoom"
            | "flex"
            | "flexGrow"
            | "flexShrink"
            | "order"
    )
}

fn has_non_logical_border_radius(value: &str) -> bool {
    let groups = value.split('/').map(str::trim);
    for group in groups {
        let radius = group
            .split_whitespace()
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();
        let invalid = match radius.len() {
            0 | 1 => false,
            2 => radius[0] != radius[1],
            3 => radius[1] != radius[2],
            4 => radius[2] != radius[3],
            _ => true,
        };
        if invalid {
            return true;
        }
    }
    false
}

fn parse_parent_selector_path(info: &CssLintContext) -> String {
    let mut out = String::new();
    for selector in &info.parent_selectors {
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

fn extract_not_selector_contents(selector_path: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut remaining = selector_path;

    while let Some(start) = remaining.find(":not(") {
        let after = &remaining[start + 5..];
        let mut depth = 1i32;
        let mut end = None;

        for (index, ch) in after.char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(index);
                        break;
                    }
                }
                _ => {}
            }
        }

        let Some(end) = end else {
            break;
        };
        out.push(after[..end].to_string());
        remaining = &after[end + 1..];
    }

    out
}

fn is_concat_selector(value: &str) -> bool {
    let bytes = value.as_bytes();
    let mut i = 0usize;
    let mut parts = 0usize;

    while i < bytes.len() {
        while i < bytes.len()
            && matches!(bytes[i], b' ' | b'\t' | b'\n' | b'>' | b'+' | b'~' | b',')
        {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }

        parts += 1;
        if parts > 1 {
            return true;
        }

        match bytes[i] {
            b'[' => {
                i += 1;
                while i < bytes.len() && bytes[i] != b']' {
                    i += 1;
                }
                if i < bytes.len() {
                    i += 1;
                }
            }
            b'.' | b'#' => {
                i += 1;
                while i < bytes.len() && is_selector_ident(bytes[i]) {
                    i += 1;
                }
            }
            _ => {
                while i < bytes.len() && is_selector_ident(bytes[i]) {
                    i += 1;
                }
            }
        }
    }

    false
}

fn is_selector_ident(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':')
}
