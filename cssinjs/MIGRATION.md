# CSSinJS Migration Guide

Canonical architecture is now locked:

- Runtime crate: `cssinjs`
- Macro crate: `dioxus-style-macro`
- App/runtime integration: `cssinjs::CssInJsPlugin` + `cssinjs::CssInJsProvider`

## Consumer Rules

1. Use only `cssinjs` APIs in app/runtime code:
- `cssinjs::CssInJsPlugin`
- `cssinjs::CssInJsRuntime`
- `cssinjs::hooks::*`
- `cssinjs::bundle::*`

2. Do not wire legacy crate APIs directly in app code.

3. Production bundle output is CSS-only:
- `cssinjs::write_bundle_files("assets/style.css", options)`
- no `style.json` / `style.meta.json` artifact path

## Legacy Crate Status

- Legacy sibling directories `cssinjs/dioxus-style` and
  `cssinjs/dioxus-style-compiler-core` were removed from the active tree.
- They are not part of active workspace runtime wiring.
- New work must target `cssinjs/src/*` and `cssinjs/dioxus-style-macro/*` only.
