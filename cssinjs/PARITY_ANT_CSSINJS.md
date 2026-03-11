# CSSinJS React Parity Matrix (Ant Design V6)

Status: `full-engine-behavior`

Source of truth used for this parity pass (V6-oriented runtime surfaces):
- `/home/barracuda/comp/react-components/cssinjs-react/cssinjs/src/StyleContext.tsx`
- `/home/barracuda/comp/react-components/cssinjs-react/cssinjs/src/hooks/useStyleRegister.tsx`
- `/home/barracuda/comp/react-components/cssinjs-react/cssinjs/src/hooks/useCSSVarRegister.ts`
- `/home/barracuda/comp/react-components/cssinjs-react/cssinjs/src/extractStyle.ts`

## Matrix

| Surface | Ant V6 behavior target | Dioxus status | Notes |
|---|---|---|---|
| `StyleProvider` context merge | Nested provider merges parent + overrides | Implemented | `cssinjs` provider config merge path is active. |
| Hash priority | `low` uses `:where`, `high` direct selector | Implemented | Verified by `parity_v6_hash_priority_low_high_selector_behavior`. |
| Layer support | Optional cascade layer wrapping (`layer`) | Implemented | Verified by `parity_v6_layer_wrapping_behavior`. |
| Style register dedup | Cache identity dedup and replacement | Implemented | `identity_key` + registry replacement in canonical `cssinjs` runtime. |
| Explicit register order | Deterministic `order` controls output and record ordering | Implemented | Verified by `tests/register_order.rs` and style bridge order tests. |
| Rewrite flow | Prefix/class/css-var rewrite support | Implemented | `CssRewriteCfg` + rewrite fingerprint in cache identity. |
| CSS var registration | Token map -> vars css + optional style register | Implemented | `register_css_vars` supports scoped output + registration. |
| Extract/export styles | Extract style/token/cssVar sets from cache | Implemented (full-engine) | `build_bundle` + metadata snapshot gives deterministic export payload. |
| `once` extract semantics | Skip already extracted styles by cache instance | Implemented | `BundleExtractState` + `build_bundle_once(...)`; `BundleExtractCache` + `build_bundle_once_with_cache(...)` align user cache-entity lifecycle semantics. |
| DOM cache path marker | `data-ant-cssinjs-cache-path` serialization | Implemented (v1) | `emit_cache_path_marker` appends hydration marker css. |
| SSR inline fallback flag | Optional inline `<style/>` fallback | Implemented (v1) | `CssInJsProviderProps.ssr_inline` and plugin `ssr_inline`. |
| Hash class layout (dev/runtime) | Ant uses `hashCls = ${hashPrefix}-${hashId}` from `@emotion/hash` | Confirmed | We verify source parity in `tests/ant_hash_layout_parity.rs`; runtime formats are intentionally normalized to alpha-only short classes (`_R_[A-Za-z]{6}_` for dev styles, `[A-Za-z]{N}` for core cssinjs hash classes). |

## Explicit Gaps (Tracked)

Behavioral parity and package-surface parity are now split explicitly.

Behavioral/runtime parity status:
- Full-engine runtime flows are covered and passing for the current scope.

Top-level package-surface parity status against local `@ant-design/cssinjs` `2.1.2` and `@ant-design/cssinjs-utils` `2.1.2`:
- Implemented at `cssinjs` top level:
  - `Theme`
  - `createTheme`
  - `useStyleRegister`
  - `useCSSVarRegister`
  - `useCacheToken`
  - `createCache`
  - `StyleProvider`
  - `StyleContext`
  - `Keyframes`
  - `extractStyle`
  - `getComputedToken`
  - `autoPrefixTransformer`
  - `legacyLogicalPropertiesTransformer`
  - `px2remTransformer`
  - `logicalPropertiesLinter`
  - `legacyNotSelectorLinter`
  - `parentSelectorLinter`
  - `NaNLinter`
  - `token2CSSVar`
  - `genCalc`
  - `mergeToken`
  - `unit`
- Implemented in workspace but not matched at `cssinjs` top-level API:
  - `genStyleUtils`
- Missing from workspace today:
  - none in the locked runtime export list

Locked source manifest and tests:
- [ant_surface_manifest.json](/home/barracuda/comp/.parity/cssinjs/manifest/ant_surface_manifest.json)
- [ant_surface_manifest_lock.rs](/home/barracuda/comp/cssinjs/tests/ant_surface_manifest_lock.rs)
- [ant_api_parity.rs](/home/barracuda/comp/cssinjs/tests/ant_api_parity.rs)

Latest strict surface pass:
- Added top-level `cssinjs` exports for:
  - `StyleProvider`
  - `useCSSVarRegister`
  - `getComputedToken`
  - `token2CSSVar`
  - `genCalc`
  - `mergeToken`
  - `statisticToken`
  - `statistic`
  - Ant transformer exports
  - Ant linter exports
- Added behavior coverage for:
  - css-var register flow without theme derivative stage
  - logical property transform mapping
  - `px -> rem` transform behavior
  - lint warning emission

Bundler callback-name parity closure:
- `.parity/cssinjs/scripts/parity_hmr_bundler_trace.sh` now runs DX with target log capture:
  - `dx --trace --json-output --log-to-file <artifact>/dx-target.log serve ...`
- report includes source-anchored callback IDs:
  - `dioxus::serve::runner::handle_file_change.queue_pending`
  - `dioxus::serve::serve_all.build_ready_pending_drain`
  - `dioxus::build::request::build_command.cargo_rustc_start`
  - `dioxus::build::request::build.build_and_bundle_complete`
- callback IDs are mapped to React/Vite hook surface (`handleHotUpdate`, `buildStart`, `buildEnd`) in report metadata.

## Full-Engine Parity Coverage (Current)

Implemented dedicated full-engine parity suite:
- `/home/barracuda/comp/cssinjs/tests/full_engine_parity.rs`

Covered flows:
- Runtime injector lifecycle parity:
  - mount -> once extract emits
  - no-op second pass emits nothing
  - update replaces previous css
  - unmount removes style from full bundle
- HMR-style replacement parity:
  - runtime injector + cssinjs runtime replacement does not keep stale fragments
  - counts remain stable (no duplicate active records after replace)
- Theme switching parity:
  - no-op register with same token payload emits nothing in once mode
  - token mutation emits delta only
  - switching back emits delta only
  - single active cssinjs record retained for same identity path

Verification wiring:
- `.parity/cssinjs/scripts/verify.sh` now runs:
  - `cargo test -p cssinjs --test full_engine_parity`
  - optional `CSSINJS_VERIFY_ANTD_TRACE=1` for AntD trace command
  - optional `CSSINJS_VERIFY_BROWSER=1` for Playwright live browser parity flow
  - optional `CSSINJS_VERIFY_FULL_ENGINE_BROWSER=1` for full-engine DOM parity flow:
    - `.parity/cssinjs/scripts/parity_full_engine_live.sh`
  - optional `CSSINJS_VERIFY_RUNTIME_ORDER_BROWSER=1` for runtime DOM `<style>` order parity:
    - `.parity/cssinjs/scripts/parity_runtime_order_live.sh`
  - optional `CSSINJS_VERIFY_HMR_BROWSER=1` for module-level HMR parity flow:
    - `.parity/cssinjs/scripts/parity_hmr_live.sh`
  - optional `CSSINJS_VERIFY_HMR_ORDER_BROWSER=1` for burst event-order HMR parity flow:
    - `.parity/cssinjs/scripts/parity_hmr_order_live.sh`
  - optional `CSSINJS_VERIFY_HMR_TOOLCHAIN_BROWSER=1` for toolchain trace ordering parity flow:
    - `.parity/cssinjs/scripts/parity_hmr_toolchain_trace.sh`
  - optional `CSSINJS_VERIFY_HMR_BUNDLER_TRACE=1` for pure bundler trace ordering parity flow:
    - `.parity/cssinjs/scripts/parity_hmr_bundler_trace.sh`

Live integration page status:
- Implemented in preview app:
  - `/home/barracuda/comp/crates/preview-app/src/pages/cssinjs.rs`
  - includes runtime lifecycle controls, theme light/dark/clear controls, and DOM probe-friendly config (`style_node_id_prefix=dxcss-`).
  - includes dedicated runtime DOM order controls:
    - `Order Probe Mount`
    - `Order Probe Burst`
    - browser-readable markers:
      - `#cssinjs-order-probe-expected`
      - `#cssinjs-order-probe-status`

Latest browser parity run:
- Runtime DOM `<style>` order parity passed:
  - `bash .parity/cssinjs/scripts/parity_runtime_order_live.sh`
  - artifacts: `/home/barracuda/comp/output/playwright/cssinjs-runtime-order-parity/20260308-152705`
- HMR module-cycle parity script passed:
  - `DX_PORT=8091 .parity/cssinjs/scripts/parity_hmr_live.sh`
  - artifacts: `/home/barracuda/comp/output/playwright/cssinjs-hmr-parity/20260307-031115`
- HMR burst event-order parity script passed:
  - `DX_PORT=8092 .parity/cssinjs/scripts/parity_hmr_order_live.sh`
  - artifacts: `/home/barracuda/comp/output/playwright/cssinjs-hmr-order-parity/20260307-040823`
- HMR toolchain-order trace script passed:
  - `DX_PORT=8095 .parity/cssinjs/scripts/parity_hmr_toolchain_trace.sh`
  - artifacts: `/home/barracuda/comp/output/playwright/cssinjs-hmr-toolchain-trace/20260307-052150`
- HMR bundler-internal trace script passed:
  - `DX_PORT=8096 .parity/cssinjs/scripts/parity_hmr_bundler_trace.sh`
  - artifacts: `/home/barracuda/comp/output/playwright/cssinjs-hmr-bundler-trace/20260307-060924`
  - includes canonical callback report fields: `hook_callback_sources`, burst-local `hook_callbacks`, and callback parity invariants.
