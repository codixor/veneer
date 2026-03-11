# dx-html

`dx-html` is a small framework-oriented crate for HTML safety primitives that are
commonly needed in SSR, LiveView, head/document rendering, template helpers,
inline boot payloads, and CMS / user-content rendering.

## Modules

- `escape.rs`
  - context-aware escaping for text and quoted attributes
  - `fmt::Display` wrappers for ergonomic rendering
  - optional / boolean / `data-*` attribute helpers
- `sanitize.rs`
  - untrusted HTML sanitization using `ammonia`
  - strict and rich-text presets
  - `SanitizedHtml` wrapper for explicit trust boundaries
- `script.rs`
  - HTML-safe JSON / JavaScript literal helpers for inline `<script>` usage
  - assignment helpers
  - `<script type="application/json">` helpers

## Intended use

Use `escape` for plain trusted strings that need HTML context encoding.

Use `sanitize` when the input itself is untrusted rich HTML and must be cleaned
before rendering.

Use `script` when embedding JSON / boot payloads into inline scripts or JSON
script tags.
