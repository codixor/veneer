//! Sanitization helpers built on top of `ammonia`.
//!
//! The API provides a conservative default plus a couple of framework-friendly
//! presets that are useful for common CMS / rich-text style use cases.

use ammonia::{Builder, Document, UrlRelative};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::fmt::{self, Display};

/// A sanitized HTML fragment that is safe to render as trusted HTML.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SanitizedHtml(Box<str>);

impl SanitizedHtml {
    /// Create a sanitized fragment using the strict default policy.
    #[must_use]
    pub fn new(input: &str) -> Self {
        Self(sanitize_html(input).into_boxed_str())
    }

    /// Access the sanitized HTML string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }

    /// Consume the wrapper and return the owned HTML string.
    #[must_use]
    pub fn into_string(self) -> String {
        self.0.into()
    }
}

impl AsRef<str> for SanitizedHtml {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Display for SanitizedHtml {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Common sanitizer presets for framework and CMS-style use cases.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SanitizerPreset {
    /// Conservative default policy.
    Strict,
    /// Rich-text content with pass-through relative URLs and common syntax-highlighting classes.
    RichText,
    /// Rich-text plus caller-supplied allowed classes for selected tags.
    RichTextWithClasses { allowed_classes: HashMap<&'static str, HashSet<&'static str>> },
}

impl Default for SanitizerPreset {
    fn default() -> Self {
        Self::Strict
    }
}

/// Sanitize untrusted HTML with the default Ammonia policy.
#[must_use]
pub fn sanitize_html(input: &str) -> String {
    ammonia::clean(input)
}

/// Sanitize untrusted HTML and return a borrowed value when no changes were needed.
#[must_use]
pub fn sanitize_html_cow(input: &str) -> Cow<'_, str> {
    Builder::default().clean(input)
}

/// Sanitize to an opaque Ammonia `Document`, useful when writing directly to a sink.
#[must_use]
pub fn sanitize_document(input: &str) -> Document {
    Builder::default().clean_from_str(input)
}

/// Build a sanitizer with a project-specific policy.
#[must_use]
pub fn sanitizer() -> Builder<'static> {
    Builder::default()
}

/// Build a sanitizer using one of the crate's built-in presets.
#[must_use]
pub fn sanitizer_with_preset(preset: SanitizerPreset) -> Builder<'static> {
    let mut builder = Builder::default();

    match preset {
        SanitizerPreset::Strict => {}
        SanitizerPreset::RichText => {
            builder.link_rel(None);
            builder.url_relative(UrlRelative::PassThrough);
            builder.add_tag_attributes("code", &["class"]);
            builder.add_tag_attributes("pre", &["class"]);
            builder.add_tag_attributes("span", &["class"]);
        }
        SanitizerPreset::RichTextWithClasses { allowed_classes } => {
            builder.link_rel(None);
            builder.url_relative(UrlRelative::PassThrough);

            let mapped = allowed_classes
                .into_iter()
                .map(|(tag, values)| (tag, values.into_iter().collect::<HashSet<_>>()))
                .collect::<HashMap<_, _>>();
            builder.allowed_classes(mapped);
        }
    }

    builder
}

/// Sanitize using a caller-supplied policy.
#[must_use]
pub fn sanitize_with(input: &str, builder: &Builder<'_>) -> String {
    builder.clean(input).to_string()
}

/// Sanitize using a built-in preset and return a trusted wrapper type.
#[must_use]
pub fn sanitize_with_preset(input: &str, preset: SanitizerPreset) -> SanitizedHtml {
    let builder = sanitizer_with_preset(preset);
    SanitizedHtml(sanitize_with(input, &builder).into_boxed_str())
}
