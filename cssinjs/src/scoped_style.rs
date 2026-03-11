use crate::engine::ScopedStyle;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScopedClassEntry {
    pub key: &'static str,
    pub class_name: &'static str,
}

impl ScopedClassEntry {
    #[inline]
    pub const fn new(key: &'static str, class_name: &'static str) -> Self {
        Self { key, class_name }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScopedStyleSpec {
    style_id: &'static str,
    raw_css: &'static str,
    scope: &'static str,
    classes_joined: &'static str,
    classes: &'static [ScopedClassEntry],
}

impl ScopedStyleSpec {
    #[inline]
    pub const fn new(
        style_id: &'static str,
        raw_css: &'static str,
        scope: &'static str,
        classes_joined: &'static str,
        classes: &'static [ScopedClassEntry],
    ) -> Self {
        Self {
            style_id,
            raw_css,
            scope,
            classes_joined,
            classes,
        }
    }

    #[inline]
    pub fn ensure_registered(&self) {
        if self.style_id.is_empty() || self.raw_css.is_empty() {
            return;
        }
        let _ = ScopedStyle::new(self.style_id, self.raw_css);
    }

    #[inline]
    pub fn scope(&self) -> &'static str {
        self.ensure_registered();
        self.scope
    }

    #[inline]
    pub const fn style_id(&self) -> &'static str {
        self.style_id
    }

    #[inline]
    pub const fn raw_css(&self) -> &'static str {
        self.raw_css
    }

    #[inline]
    pub const fn classes_joined(&self) -> &'static str {
        self.classes_joined
    }

    #[inline]
    pub const fn classes(&self) -> &'static [ScopedClassEntry] {
        self.classes
    }

    #[inline]
    pub fn class(&self, key: &str) -> Option<&'static str> {
        self.classes
            .iter()
            .find(|entry| entry.key == key)
            .map(|entry| entry.class_name)
    }
}

impl std::fmt::Display for ScopedStyleSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.scope())
    }
}

impl From<ScopedStyle> for ScopedStyleSpec {
    fn from(value: ScopedStyle) -> Self {
        let scope = value.scope();
        Self::new(scope, value.raw_css(), scope, "", &[])
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScopedClassMap {
    scope: &'static str,
    classes_joined: &'static str,
    classes: &'static [ScopedClassEntry],
}

impl ScopedClassMap {
    #[inline]
    pub const fn from_spec(spec: ScopedStyleSpec) -> Self {
        Self {
            scope: spec.scope,
            classes_joined: spec.classes_joined,
            classes: spec.classes,
        }
    }

    #[inline]
    pub const fn scope(&self) -> &'static str {
        self.scope
    }

    #[inline]
    pub const fn classes_joined(&self) -> &'static str {
        self.classes_joined
    }

    #[inline]
    pub const fn entries(&self) -> &'static [ScopedClassEntry] {
        self.classes
    }

    #[inline]
    pub fn class(&self, key: &str) -> Option<&'static str> {
        self.classes
            .iter()
            .find(|entry| entry.key == key)
            .map(|entry| entry.class_name)
    }
}

pub trait IntoScopedStyleSpec {
    fn into_scoped_style_spec(self) -> ScopedStyleSpec;
}

impl IntoScopedStyleSpec for ScopedStyleSpec {
    #[inline]
    fn into_scoped_style_spec(self) -> ScopedStyleSpec {
        self
    }
}

impl IntoScopedStyleSpec for ScopedStyle {
    #[inline]
    fn into_scoped_style_spec(self) -> ScopedStyleSpec {
        self.into()
    }
}
