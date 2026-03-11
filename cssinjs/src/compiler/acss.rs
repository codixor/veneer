//! ACSS (atomic CSS) compiler.
//!
//! Input format:
//! - `property:value` (base)
//! - `variant:property:value` where variant is: hover|focus|focus-visible|focus-within|active|disabled
//! - optional named class seed: `seed = property:value` or `seed = variant:property:value`
//! - optional inline marker to avoid false positives: leading `@acss`

use std::collections::HashSet;

use super::hash::{ScopeHashTarget, ScopeHasher, scope_hash_params_for};

pub struct AcssCompiler;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AcssCompileOutput {
    pub css: String,
    pub classes: Vec<String>,
}

impl AcssCompiler {
    #[inline]
    pub fn is_file(path: &str) -> bool {
        path.trim().to_ascii_lowercase().ends_with(".acss")
    }

    #[inline]
    pub fn is_inline_marker(content: &str) -> bool {
        content.trim_start().starts_with("@acss")
    }

    pub fn compile(content: &str, minify: bool) -> Result<String, String> {
        Ok(Self::compile_with_meta(content, minify)?.css)
    }

    pub fn compile_with_meta(content: &str, minify: bool) -> Result<AcssCompileOutput, String> {
        let source = Self::strip_inline_marker(content);

        let mut rules: Vec<String> = Vec::new();
        let mut classes: Vec<String> = Vec::new();
        let mut seen = HashSet::<String>::new();

        for (line_idx, line) in source.lines().enumerate() {
            let line_no = line_idx + 1;
            for segment in line.split(';') {
                let raw = segment.trim();
                if raw.is_empty() || raw.starts_with("//") || raw.starts_with('#') {
                    continue;
                }
                if raw.contains('{') || raw.contains('}') {
                    return Err(format!(
                        "line {line_no}: braces are not supported in ACSS atoms (`{raw}`)"
                    ));
                }

                let (class_seed, atom_src) = if let Some((left, right)) = raw.split_once('=') {
                    let seed = Self::normalize_class_seed(left).ok_or_else(|| {
                        format!("line {line_no}: invalid ACSS class seed `{left}`")
                    })?;
                    (Some(seed), right.trim())
                } else {
                    (None, raw)
                };

                let atom = Self::parse_atom(atom_src, line_no, class_seed)?;
                let key = atom.canonical_key();
                if !seen.insert(key) {
                    continue;
                }
                classes.push(atom.class_name());
                rules.push(atom.css_rule());
            }
        }

        if rules.is_empty() {
            return Ok(AcssCompileOutput::default());
        }

        Ok(AcssCompileOutput {
            css: if minify {
                rules.join("")
            } else {
                rules.join("\n")
            },
            classes,
        })
    }

    #[inline]
    fn strip_inline_marker(content: &str) -> &str {
        let trimmed = content.trim_start();
        if trimmed.starts_with("@acss") {
            trimmed.trim_start_matches("@acss").trim_start()
        } else {
            content
        }
    }

    fn normalize_class_seed(raw: &str) -> Option<String> {
        let mut out = String::with_capacity(raw.len() + 4);
        let mut prev_dash = false;

        for ch in raw.trim().chars() {
            match ch {
                'a'..='z' | '0'..='9' => {
                    out.push(ch);
                    prev_dash = false;
                }
                'A'..='Z' => {
                    out.push(ch.to_ascii_lowercase());
                    prev_dash = false;
                }
                '_' | '-' | ' ' => {
                    if !out.is_empty() && !prev_dash {
                        out.push('-');
                        prev_dash = true;
                    }
                }
                _ => {
                    if !out.is_empty() && !prev_dash {
                        out.push('-');
                        prev_dash = true;
                    }
                }
            }
        }

        let out = out.trim_matches('-').to_string();
        if out.is_empty() {
            return None;
        }
        if out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            return Some(format!("x-{out}"));
        }
        Some(out)
    }

    fn parse_atom(
        raw: &str,
        line_no: usize,
        class_seed: Option<String>,
    ) -> Result<ParsedAtom, String> {
        let atom = raw.trim();
        if atom.is_empty() {
            return Err(format!("line {line_no}: empty ACSS atom"));
        }

        let mut parts = atom.splitn(3, ':');
        let first = parts.next().unwrap_or_default().trim();
        let second = parts.next().unwrap_or_default().trim();
        let third = parts.next().map(str::trim);

        if first.is_empty() || second.is_empty() {
            return Err(format!("line {line_no}: expected `property:value`"));
        }

        let (variant, property, value) = match third {
            Some(rest) => {
                if let Some(v) = AtomVariant::parse(first) {
                    (v, second.to_ascii_lowercase(), rest.to_string())
                } else {
                    // Allow values to contain ':' by folding into base.
                    (
                        AtomVariant::Base,
                        first.to_ascii_lowercase(),
                        format!("{second}:{rest}"),
                    )
                }
            }
            None => (
                AtomVariant::Base,
                first.to_ascii_lowercase(),
                second.to_string(),
            ),
        };

        if property.trim().is_empty() || value.trim().is_empty() {
            return Err(format!("line {line_no}: invalid ACSS atom `{atom}`"));
        }

        Ok(ParsedAtom {
            class_seed,
            variant,
            property,
            value,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum AtomVariant {
    Base,
    Hover,
    Focus,
    FocusVisible,
    FocusWithin,
    Active,
    Disabled,
}

impl AtomVariant {
    #[inline]
    fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "hover" => Some(Self::Hover),
            "focus" => Some(Self::Focus),
            "focus-visible" | "focusvisible" => Some(Self::FocusVisible),
            "focus-within" | "focuswithin" => Some(Self::FocusWithin),
            "active" => Some(Self::Active),
            "disabled" => Some(Self::Disabled),
            "base" => Some(Self::Base),
            _ => None,
        }
    }

    #[inline]
    fn as_key(self) -> &'static str {
        match self {
            Self::Base => "base",
            Self::Hover => "hover",
            Self::Focus => "focus",
            Self::FocusVisible => "focus-visible",
            Self::FocusWithin => "focus-within",
            Self::Active => "active",
            Self::Disabled => "disabled",
        }
    }

    #[inline]
    fn selector_suffix(self) -> &'static str {
        match self {
            Self::Base => "",
            Self::Hover => ":hover",
            Self::Focus => ":focus",
            Self::FocusVisible => ":focus-visible",
            Self::FocusWithin => ":focus-within",
            Self::Active => ":active",
            Self::Disabled => ":disabled",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct ParsedAtom {
    class_seed: Option<String>,
    variant: AtomVariant,
    property: String,
    value: String,
}

impl ParsedAtom {
    #[inline]
    fn canonical_key(&self) -> String {
        format!(
            "{}|{}|{}|{}",
            self.class_seed.as_deref().unwrap_or_default(),
            self.variant.as_key(),
            self.property.as_str(),
            self.value.as_str()
        )
    }

    #[inline]
    fn hash_key(&self) -> String {
        format!(
            "{}|{}|{}",
            self.variant.as_key(),
            self.property.as_str(),
            self.value.as_str()
        )
    }

    #[inline]
    fn class_name(&self) -> String {
        if let Some(seed) = self.class_seed.as_deref() {
            seed.to_string()
        } else {
            let cfg = scope_hash_params_for(ScopeHashTarget::Acss);
            ScopeHasher::generate_with_cfg(self.hash_key().as_str(), None, cfg.as_cfg())
        }
    }

    #[inline]
    fn css_rule(&self) -> String {
        format!(
            ".{}{}{{{}:{};}}",
            self.class_name(),
            self.variant.selector_suffix(),
            self.property,
            self.value
        )
    }
}
