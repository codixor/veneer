# CSSinJS Unification Plan

Goal: align css-in-js with our official primitive Dioxus architecture, with one canonical runtime crate + one macro crate, dev-mode live styling, and production bundle generation.

## Phase 1. Canonical crate unification
- Promote `/home/barracuda/comp/cssinjs` as canonical runtime crate.
- Keep one macro crate for proc macros (`cssinjs-macro` target shape).
- Keep temporary compatibility layer from old crate names during migration.
- Remove duplicate runtime ownership across crates after migration completes.

## Phase 2. Primitive runtime architecture (provider/hooks/plugin/platform/runtime)
- Add primitive modules in root `cssinjs` crate:
  - `provider.rs`
  - `hooks.rs`
  - `plugin.rs`
  - `platform/{mod.rs,web.rs,native.rs,ssr.rs}`
  - `runtime.rs`
- Keep API cross-platform stable and isolate platform behavior by target.
- Wire provider/context in official primitive style (no nested wrappers needed).

## Phase 3. Header integration
- Route runtime style nodes through `/home/barracuda/comp/header` instead of direct ad-hoc DOM ownership.
- Use deterministic style keys and staged deltas via header runtime manager.
- Preserve SSR/native no-op patch safety.

## Phase 4. Dev-mode live style authoring
- Add runtime APIs for per-component style editing by key.
- Support token updates and scoped live CSS updates.
- Keep updates batched and deterministic.

## Phase 5. Production bundle generation
- Add bundle command to emit final `assets/style.css`.
- Merge macro-precompiled styles + runtime records + token/css-var outputs deterministically.
- Keep bundle output CSS-only (no metadata JSON artifact).

## Phase 6. Ant CSS-in-JS parity matrix (V6)
- Add parity matrix against `ant-design/cssinjs` behavior surface.
- Cover `StyleProvider`, style register flow, css-var register flow, extract/export behavior.
- Track parity gaps explicitly.

### Phase 6 Locked Reference Model (Ant Design)
- Layer 1: Root runtime injection
  - Wrap app root with `StyleProvider` (and `ConfigProvider` where required by antd integration flow).
  - Source refs:
    - `/home/barracuda/comp/react-components/ant-design/docs/react/compatible-style.en-US.md` (line 30)
    - `/home/barracuda/comp/react-components/ant-design/docs/react/compatible-style.en-US.md` (line 113)
- Layer 2: Token + component style registration
  - Token path: `useToken()` -> `useCacheToken(...)` (theme/token/hash pipeline).
  - Component style path: `useStyleRegister(...)` with `path/hashId/layer/order`.
  - Source refs:
    - `/home/barracuda/comp/react-components/ant-design/components/theme/useToken.ts` (line 132)
    - `/home/barracuda/comp/react-components/ant-design/components/theme/internal.ts` (line 1)
    - `/home/barracuda/comp/react-components/ant-design/components/theme/util/useResetIconStyle.ts` (line 11)
- Layer 3: SSR / build extraction
  - SSR inline path: `createCache()` -> `renderToString(...)` -> `extractStyle(cache)`.
  - Static build path: render full component surface and extract plain CSS (`types: "style"`).
  - Source refs:
    - `/home/barracuda/comp/react-components/ant-design/docs/react/server-side-rendering.en-US.md` (line 19)
    - `/home/barracuda/comp/react-components/ant-design/scripts/build-style.tsx` (line 130)
- Internal extract behavior parity:
  - `extractStyle(..., { once })` skips previously extracted keys while still appending cache-path marker mapping.
  - Source ref:
    - `/home/barracuda/comp/react-components/cssinjs-react/cssinjs/src/extractStyle.ts` (line 32)
- Parity note:
  - metadata JSON artifact path was removed from the bundle CLI in the cleanup pass; Ant parity is validated on emitted CSS/runtime behavior.

## Phase 7. Performance/min-computation
- Incremental cache invalidation by identity/rewrite/token signatures.
- No heavy compile work on render path.
- Batch platform updates (frame-coalesced).

## Phase 8. Migration rollout and cleanup
- Stage A: scaffold + compatibility re-exports.
- Stage B: move consumers to canonical runtime API.
- Stage C: remove deprecated paths.

## Phase 9. Test and verification gates
- Native + wasm `cargo check`/`cargo clippy` for cssinjs crates.
- Parity tests and deterministic bundle snapshot tests.
- Stress tests for rapid live-edit loops.

## Current Status
- Phase 1: Completed (workspace v1).
  - `/home/barracuda/comp/cssinjs` is the canonical runtime API crate.
  - Workspace alias wiring is in place.
  - Legacy wildcard compatibility export (`pub use dioxus_style::*`) was removed in migration cleanup.
  - Runtime ownership is consolidated at root `cssinjs` API surface.
- Phase 2: Completed (v1).
  - Primitive modules are present (`provider/hooks/plugin/platform/runtime` + platform split).
  - `CssInJsPlugin` is wired into `app-config` provider stack.
- Phase 3: Completed (v1).
  - `CssInJsProvider` bridges runtime CSS through `/home/barracuda/comp/header` with deterministic style keying.
  - Bridge mode disables direct runtime DOM injection (`runtime_dom_injection=false`) and commits via header runtime manager.
  - SSR/native safety is preserved via platform capability split and no-op-safe bridge behavior.
- Phase 4: Completed (v1).
  - Added keyed dev style runtime API:
    - `upsert_dev_style(component_key, style_id, css)`
    - `remove_dev_style(component_key, style_id)`
    - `clear_dev_styles()`
    - `list_dev_styles()`
  - Added scoped dev token runtime API:
    - `upsert_dev_tokens(component_key, token_id, token_map)`
    - `remove_dev_tokens(component_key, token_id)`
    - `clear_dev_tokens()`
    - `list_dev_tokens()`
  - Dev token updates now emit deterministic scope + prefix wiring and replace previous token payload by key.
  - Added regression coverage:
    - `/home/barracuda/comp/cssinjs/tests/dev_token_runtime.rs`
    - bundle once delta test in `/home/barracuda/comp/cssinjs/tests/bundle_runtime.rs`
- Phase 5: Completed (v1).
  - Added `cssinjs::bundle` runtime API:
    - `build_bundle(BundleBuildOptions)`
    - `write_bundle_files(css_path, BundleBuildOptions)`
  - Added CLI command:
    - `cargo run -p cssinjs -- bundle --css assets/style.css`
  - Added deterministic bundle summary payload:
    - css hash (`xxh3_64`)
    - css length counters
    - merged precompile/runtime record snapshot
  - JSON artifact output path was removed; parity is validated on emitted CSS + runtime behavior.
  - Added crate-root tests for deterministic bundle snapshot and file emission.
- Phase 6: Completed (v1).
  - Added React-source parity matrix document:
    - `/home/barracuda/comp/cssinjs/PARITY_ANT_CSSINJS.md`
  - Added parity tests mapped to Ant source surfaces:
    - style register flow
    - css-var register flow
    - extract/export bundle parity checks
    - hash priority + layer parity checks (V6 surface)
  - Added V6 parity closures (v1):
    - `build_bundle_once` + `BundleExtractState` (`once`-style extraction)
    - optional cache-path marker css emission (`emit_cache_path_marker`)
    - provider/plugin `ssr_inline` runtime toggle
    - provider-level extract state hook (`use_cssinjs_extract_state`) for cache-entity-like persistence
    - preview route `/cssinjs` for live parity validation
- Phase 7: Completed (v1).
  - Added monotonic style revision tracking in canonical `cssinjs` runtime (`CssInJs::revision()`).
  - Bridge path now uses revision-gated updates + Arc pointer checks to avoid expensive css string churn on each tick.
  - Default bridge polling aligned to frame cadence (`16ms`) for coalesced platform commits.
  - Added performance regression test for revision behavior under noop/replace/unregister/clear flows.
- Phase 8: Completed (workspace v1).
  - Added migration guide:
    - `/home/barracuda/comp/cssinjs/MIGRATION.md`
  - Stage A documented as complete.
  - Stage B documented and app-config path confirmed on `CssInJsPlugin`.
  - Stage C applied for workspace:
    - removed `pub use dioxus_style::*` compatibility wildcard from `cssinjs::lib`.
- Phase 9: Completed (v1).
  - Added rapid live-edit stress test:
    - `/home/barracuda/comp/cssinjs/tests/dev_style_stress.rs`
  - Added executable verification gate script:
    - `/home/barracuda/comp/.parity/cssinjs/scripts/verify.sh`
  - Gate run complete:
    - native/wasm check + clippy (`cssinjs`)
    - `cssinjs` tests (parity, bundle, stress, revision)
    - canonical runtime parity/perf/concurrency suites
- Phase 10: Completed (v2 initial).
  - Added release pipeline script:
    - `/home/barracuda/comp/cssinjs/scripts/release_web_bundle.sh`
  - Script now wires automatic release flow:
    - `dx build --platform web -p <package> --release`
    - `cargo run -p cssinjs -- bundle ...`
    - mirror bundle into release public assets directory.
  - Deterministic output location contract:
    - canonical: `/home/barracuda/comp/assets/style.css`
    - release mirror: `/home/barracuda/comp/target/dx/<package>/release/web/public/assets/style.css`
  - Added build task wiring in `/home/barracuda/comp/mise.toml`:
    - `mise run cssbundle`
    - `mise run dxweb-release`
- Phase 11: Completed (v2).
  - Bridge scheduling now uses revision-event callbacks as the primary trigger path.
  - Added runtime revision listener subscription APIs:
    - `subscribe_revision_listener`
    - `unsubscribe_revision_listener`
  - `CssInJsProvider` now subscribes to revision events via Dioxus `schedule_update()` callback.
  - Timer polling path remains only as fallback when runtime revision events are unavailable.
  - Added regression coverage:
    - `/home/barracuda/comp/cssinjs/tests/revision_listener.rs`
- Phase 12: Completed (v2).
  - Added CI profile split to `verify.sh`:
    - `--profile fast`: rust checks/tests only
    - `--profile parity`: rust checks/tests + browser parity + bundler trace flows
  - Added workspace tasks in `/home/barracuda/comp/mise.toml`:
    - `mise run cssverify-fast`
    - `mise run cssverify-parity`
  - Parity scripts remain artifact-preserving by default under `/home/barracuda/comp/output/playwright/...`.
- Phase 13: Completed (v2).
  - Extended HMR churn stress is now implemented in:
    - `/home/barracuda/comp/.parity/cssinjs/scripts/parity_hmr_order_live.sh`
  - Added randomized burst support:
    - `CSSINJS_HMR_ORDER_EXTRA_RANDOM_BURSTS=<N>`
    - parity profile default: `CSSINJS_HMR_ORDER_EXTRA_RANDOM_BURSTS=1`
  - Added burst summary output and invariants:
    - ndjson records: `burst-records.ndjson`
    - summary json: `hmr-order-summary.json`
    - invariant gate: all burst build deltas must stay positive.
- Phase 14: Completed (v2).
  - Ant-only scope hardening is applied to live parity controls/selectors:
    - preview heading: `Ant Full Engine Live Parity`
    - buttons: `Style Register Mount|Update|Unmount`
    - probe selector: `.ant-style-register-probe`
    - theme probe var: `--ant-parity-color-primary`
  - Updated browser parity scripts to target the Ant naming surface:
    - `/home/barracuda/comp/.parity/cssinjs/scripts/parity_full_engine_live.sh`
    - `/home/barracuda/comp/.parity/cssinjs/scripts/parity_hmr_live.sh`
    - `/home/barracuda/comp/.parity/cssinjs/scripts/parity_hmr_order_live.sh`
  - Validation gates passed after retarget:
    - `cargo check -p preview_app --target wasm32-unknown-unknown`
    - `bash /home/barracuda/comp/.parity/cssinjs/scripts/verify.sh --profile fast`

## Additional Notes
- New: locked local npm package-surface manifest for audit-before-migration:
  - `/home/barracuda/comp/.parity/cssinjs/manifest/ant_surface_manifest.json`
  - `/home/barracuda/comp/cssinjs/tests/ant_surface_manifest_lock.rs`
- This lock distinguishes:
  - behavioral/runtime parity already validated by browser/native tests
  - top-level package-surface parity against local `@ant-design/cssinjs` and `@ant-design/cssinjs-utils`
- Current locked surface gaps:
  - implemented in workspace but not at `cssinjs` top-level:
    - `genStyleUtils`
  - missing from workspace:
    - none in the locked Ant runtime export list
  - completed in this pass:
    - `StyleProvider`
    - `useCSSVarRegister`
    - `getComputedToken`
    - `token2CSSVar`
    - `genCalc`
    - `mergeToken`
    - `autoPrefixTransformer`
    - `legacyLogicalPropertiesTransformer`
    - `px2remTransformer`
    - `logicalPropertiesLinter`
    - `legacyNotSelectorLinter`
    - `parentSelectorLinter`
    - `NaNLinter`
  - new focused verification:
    - `cargo test -p cssinjs --test ant_api_parity`
    - `cargo test -p cssinjs --test ant_surface_manifest_lock`
- Completed: class hash layout was normalized to strict alpha bodies:
  - dev runtime classes: `_R_[A-Za-z]{6}_`
  - core cssinjs class hashes: `[A-Za-z]{N}` (default `N=6`)
- Completed: Ant hash layout parity confirmation added (`tests/ant_hash_layout_parity.rs`) against local source:
  - `hashPrefix` + `@emotion/hash(mergedSalt)` -> ``${hashPrefix}-${hashId}``.
- Completed: `build_bundle_once` moved from cache-key delta to fragment-fingerprint delta:
  - detects same-key style updates (runtime injector/cssinjs) via record+css fingerprint
  - regression test added in `tests/bundle_runtime.rs` (`bundle_once_emits_runtime_style_changes_for_same_cache_key`)
- Completed: user cache-entity lifecycle for once extraction:
  - `BundleExtractCache` + `build_bundle_once_with_cache(cache, cache_id, ...)`
  - isolated per-cache-id once state with clear/remove helpers
  - regression test added in `tests/bundle_runtime.rs` (`bundle_once_cache_entities_are_isolated`)
- Completed: one-command live browser parity script:
  - `.parity/cssinjs/scripts/parity_live.sh`
  - runs dxweb + Playwright flow end-to-end and asserts hash format, once delta behavior, marker emission, and clean console
- Completed: one-command browser HMR module-cycle parity script:
  - `.parity/cssinjs/scripts/parity_hmr_live.sh`
  - validates source-patch rebuild cycles (`hmr-a -> hmr-b -> hmr-a`) and asserts no duplicate/stale style nodes after each cycle.
- Completed: one-command browser HMR event-order burst parity script:
  - `.parity/cssinjs/scripts/parity_hmr_order_live.sh`
  - validates rapid invalidation order (`hmr-a -> hmr-b -> hmr-c`, then `hmr-c -> hmr-d -> hmr-a`) and asserts last-write-wins + no duplicate/stale style nodes.
- Completed: parity page CSS scope cleanup for Ant focus:
  - removed non-Ant demo-only card style churn from `/home/barracuda/comp/crates/preview-app/src/pages/cssinjs.rs`
  - kept SCSS/ACSS compiler lanes untouched for post-parity integration.
- Completed: toolchain-level HMR ordering trace script:
  - `.parity/cssinjs/scripts/parity_hmr_toolchain_trace.sh`
  - runs `dx serve` with default logs (workspace-compatible) and emits `toolchain-trace-report.json` with:
    - source write sequence (`writes.json`)
    - server-side trace events (`server-events.json`)
    - build-count progression per burst
    - last-write-wins invariants
  - note: `dx serve --trace --json-output` is not used here because it does not reliably expose HTTP readiness in this workspace.
- Completed: bundler-internal HMR trace script (no browser dependency):
  - `.parity/cssinjs/scripts/parity_hmr_bundler_trace.sh`
  - runs `dx --trace --json-output --log-to-file` and emits `bundler-trace-report.json` with:
    - source write sequence (`writes.json`)
    - burst-local queue/build event subsets from trace logs
    - canonical bundler callback identifiers resolved from direct runtime target logs + source anchors:
      - `dioxus::serve::runner::handle_file_change.queue_pending`
      - `dioxus::serve::serve_all.build_ready_pending_drain`
      - `dioxus::build::request::build_command.cargo_rustc_start`
      - `dioxus::build::request::build.build_and_bundle_complete`
    - build-count progression invariants per burst
    - last-write persistence invariants in source (`hmr-c`, `hmr-a`)
    - callback parity mapping to React/Vite hook names (`handleHotUpdate`, `buildStart`, `buildEnd`)

## Parity Closure (Full-Engine)
- Full-engine parity now includes runtime/browser parity for:
  - runtime injector lifecycle in real DOM
  - theme switching reconciliation in real DOM
  - module-level source hot-update cycle smoke checks (sentinel patch loop)
- Native/runtime full-engine parity test suite now added:
  - `cssinjs/tests/full_engine_parity.rs`
  - covers runtime injector lifecycle, HMR-style replacement cleanup, and theme-switch delta semantics.
- Implemented browser parity coverage:
  - `.parity/cssinjs/scripts/parity_full_engine_live.sh` (mount/update/unmount + theme light/dark reconciliation)
  - `.parity/cssinjs/scripts/parity_hmr_live.sh` (module hot-update cycle parity via source sentinel patching)
  - `.parity/cssinjs/scripts/parity_hmr_order_live.sh` (burst hot-update order parity via rapid sentinel patches)
  - `.parity/cssinjs/scripts/parity_hmr_toolchain_trace.sh` (toolchain trace-level ordering report + invariants)
  - `.parity/cssinjs/scripts/parity_hmr_bundler_trace.sh` (pure bundler trace ordering invariants, no runtime DOM assertions)
- Bundler hook callback parity closure:
  - direct callback instrumentation now enabled via DX target logs (`--log-to-file`) and source-anchored callback IDs.
  - latest pass artifact:
    - `/home/barracuda/comp/output/playwright/cssinjs-hmr-bundler-trace/20260307-060924`
  - report now includes:
    - `hook_callback_sources` (dioxus source file:line anchors)
    - `hook_callbacks` per burst with callback IDs
    - callback parity mapping to React/Vite hook surface
  - parity matrix promoted from `v1` to `full-engine` in `PARITY_ANT_CSSINJS.md`.
- `.parity/cssinjs/scripts/verify.sh` wiring updated:
  - always runs `cargo test -p cssinjs --test full_engine_parity`
  - optional browser gates via:
    - `CSSINJS_VERIFY_ANTD_TRACE=1`
    - `CSSINJS_VERIFY_BROWSER=1`
    - `CSSINJS_VERIFY_FULL_ENGINE_BROWSER=1` (runs `.parity/cssinjs/scripts/parity_full_engine_live.sh`)
    - `CSSINJS_VERIFY_HMR_BROWSER=1` (runs `.parity/cssinjs/scripts/parity_hmr_live.sh`)
    - `CSSINJS_VERIFY_HMR_ORDER_BROWSER=1` (runs `.parity/cssinjs/scripts/parity_hmr_order_live.sh`)
    - `CSSINJS_VERIFY_HMR_TOOLCHAIN_BROWSER=1` (runs `.parity/cssinjs/scripts/parity_hmr_toolchain_trace.sh`)
    - `CSSINJS_VERIFY_HMR_BUNDLER_TRACE=1` (runs `.parity/cssinjs/scripts/parity_hmr_bundler_trace.sh`)
- Exit criteria for full-engine parity:
  - deterministic style output across repeated theme toggles/HMR cycles: met
  - no duplicate/stale style nodes after hot updates: met
  - parity matrix status promoted from `v1` to `full-engine` in `PARITY_ANT_CSSINJS.md`: met

## Phase 15. Crate consolidation into canonical runtime
- Status: Completed.
- End-state contract:
  - one canonical runtime crate (`cssinjs`)
  - one macro crate (`dioxus-style-macro`)
  - no duplicated engine ownership in active workspace runtime wiring.

#### Phase 15 Progress (v3)
- Completed slice:
  - Added an internal backend boundary in `/home/barracuda/comp/cssinjs/src/backend.rs` and routed `cssinjs/src/*` backend usage through it.
  - Added canonical compiler facade module:
    - `/home/barracuda/comp/cssinjs/src/compiler/mod.rs`
    - exposes `cssinjs::compiler::*` and `cssinjs::compiler::hash::*` as migration-safe fronts.
  - Promoted canonical runtime surface from `cssinjs::lib` (runtime APIs now resolve from in-crate canonical modules).
  - Migrated cssinjs integration tests away from direct `dioxus_style` imports to canonical `cssinjs` imports:
    - `/home/barracuda/comp/cssinjs/tests/perf_revision.rs`
    - `/home/barracuda/comp/cssinjs/tests/full_engine_parity.rs`
    - `/home/barracuda/comp/cssinjs/tests/bundle_runtime.rs`
    - `/home/barracuda/comp/cssinjs/tests/ant_parity_matrix.rs`
- Completed architecture migration (requested 1/2/3):
  - Runtime ownership moved into canonical crate:
    - copied runtime engine modules from `dioxus-style/src/*` into
      `/home/barracuda/comp/cssinjs/src/engine/*`
    - `cssinjs` now compiles/runtime-serves from in-crate engine modules (no runtime dependency on `dioxus-style`).
  - Compiler ownership moved into canonical crate:
    - copied compiler modules from `dioxus-style-compiler-core/src/*` into
      `/home/barracuda/comp/cssinjs/src/compiler/*`
    - compiler imports now resolve to in-crate compiler modules.
  - Macro internals retargeted to canonical paths:
    - `/home/barracuda/comp/cssinjs/dioxus-style-macro` now imports compile helpers from `cssinjs::compiler::*`
    - macro expansion runtime paths switched from `::dioxus_style::...` to `::cssinjs::...`
    - `dioxus-style-macro` added to workspace members for direct validation.
    - macro crate is now intentionally decoupled from runtime re-export loops.
- Completed final cleanup/deprecation pass:
  - removed stale workspace dependency wiring for `dioxus-style-compiler-core` from root manifest.
  - removed legacy verify gate step that executed `-p dioxus-style` tests.
  - docs/parity/migration guides now reference canonical `cssinjs` runtime paths.
  - removed legacy sibling directories:
    - `/home/barracuda/comp/cssinjs/dioxus-style`
    - `/home/barracuda/comp/cssinjs/dioxus-style-compiler-core`
- Verification gates passed for this slice:
  - `cargo check -p cssinjs`
  - `cargo test -p cssinjs --tests`
  - `cargo clippy -p cssinjs`
  - `cargo clippy -p cssinjs --target wasm32-unknown-unknown`
  - `cargo check -p dioxus-style-macro`
  - `cargo check -p dioxus-style-macro --features scss`

## Phase 16. SCSS/ACSS Primitive Alignment + Macro Compliance
- Status: Planned.
- Goal:
  - align SCSS/ACSS pipelines to the same primitive runtime contract (`provider/hooks/plugin/platform`).
  - make macro output fully compliant with runtime-owned style emission (no direct ad-hoc head/style ownership in macro expansions).
- Locked invariant:
  - `scss` / `acss` / `css` differ only in compile front-end.
  - after compile, all flow through one shared canonical runtime contract:
    - same scope/hash flow
    - same registration/emission path (`CssInJsProvider` ownership)
    - same SSR/wasm/native behavior
    - same provider/plugin runtime ownership (`CssInJsProvider` + `CssInJsPlugin`, no direct ad-hoc path)

### 16.1 SCSS/ACSS primitive contract alignment (first)
- Define one canonical compile contract in `cssinjs::compiler` for all style kinds:
  - input: `content`, optional source path, minify/options.
  - output: deterministic css payload + deterministic scope/hash metadata needed by runtime.
- Ensure SCSS and ACSS both flow through the same normalization stage (LightningCSS pass + hash/scoping contract).
- Keep format behavior deterministic across wasm/native/ssr (no target-specific output drift).
- Add explicit crate-root tests:
  - scss/acss deterministic hash+scope output parity.
  - same input -> same output across repeated runs.
  - same logical source path semantics for hash stability.

### 16.2 Macro runtime compliance (second)
- Refactor macro expansion contract to runtime-first:
  - generated code should register/resolve style through canonical `cssinjs` runtime APIs.
  - macro-generated `<Style>` nodes should be optional compatibility path, not primary ownership.
- Introduce strict mode switch (default for workspace):
  - strict mode: runtime/provider owns style emission lifecycle.
  - compatibility mode: keep legacy inline-injection behavior only where explicitly enabled.
- Remove or deprecate legacy alias surfaces after migration window:
  - `with_css`
  - `with_scss`
  - `with_acss`
  - `component_with_css`
- Keep `scoped_style!`, `css!`, `acss!` as canonical macro surface.

### 16.3 Primitive architecture rules to enforce
- No macro path should bypass `CssInJsProvider` ownership in strict mode.
- No direct DOM/head calls in macro-generated runtime paths.
- Keep SSR-safe behavior (no runtime DOM assumptions).
- Preserve wasm/native parity and deterministic bundle output.

### 16.4 Migration and compatibility
- Stage A:
  - add strict-mode plumbing + runtime registration path.
  - keep compatibility path on by explicit flag only.
- Stage B:
  - migrate preview and workspace consumers to strict mode.
  - remove remaining callsites relying on legacy alias macros.
- Stage C:
  - deprecate compatibility mode in docs.
  - follow-up removal once no consumers remain.

### 16.5 Validation gates
- Rust checks:
  - `cargo check -p cssinjs`
  - `cargo check -p cssinjs --target wasm32-unknown-unknown`
  - `cargo check -p dioxus-style-macro --features scss`
- Clippy:
  - `cargo clippy -p cssinjs`
  - `cargo clippy -p cssinjs --target wasm32-unknown-unknown`
  - `cargo clippy -p dioxus-style-macro --features scss`
- Tests:
  - `cargo test -p cssinjs --tests`
  - add macro compliance tests for strict-mode runtime ownership.
- Parity:
  - rerun cssinjs full-engine parity scripts and confirm no regression.

#### Phase 16 Progress (v1, started)
- Completed slice:
  - Canonical compile contract was moved into runtime crate API:
    - `cssinjs::compiler::StyleCompiler`
    - `cssinjs::compiler::StyleKind`
    - macro compile bridge now delegates to canonical runtime compiler contract.
  - `dioxus-style-macro` now defaults to strict primitive mode:
    - `#[scoped]` registers scoped styles and returns user element without direct `<Style>` injection.
    - legacy inline fallback remains available only via macro feature:
      - `legacy-inline-style-fallback`
  - ACSS macro path now passes generated CSS through the same shared normalization stage used by CSS/SCSS (`StyleCompiler::normalize_css`), tightening post-compile parity.
  - Legacy alias macro surfaces were marked deprecated:
    - `scoped_asset_scope`
    - `with_css`
    - `with_scss`
    - `with_acss`
    - `component_with_css`
  - Added compliance tests:
    - `cssinjs/tests/compiler_contract.rs` validates shared post-compile normalization contract.
    - `dioxus-style-macro` unit test validates strict mode does not inject `dioxus::document::Style`.
  - Added concrete scoped hook API for RSX class-map usage:
    - new runtime types in `cssinjs`:
      - `ScopedStyleSpec`
      - `ScopedClassEntry`
      - `ScopedClassMap`
    - new hook:
      - `use_scoped_style(...)`
    - macro outputs aligned to hook input:
      - `scoped_style!(...)` now returns `ScopedStyleSpec`
      - `acss!(...)` now returns `ScopedStyleSpec`
    - class map access supports RSX usage:
      - `map.scope()`
      - `map.classes_joined()`
      - `map.class("key")`
  - Added API coverage tests:
    - `/home/barracuda/comp/cssinjs/tests/scoped_style_api.rs`
    - `/home/barracuda/comp/cssinjs/dioxus-style-macro/tests/scoped_hook_api.rs`
- Verification run for this slice:
  - `cargo check -p dioxus-style-macro --features scss`
  - `cargo test -p dioxus-style-macro --features scss`
  - `cargo clippy -p dioxus-style-macro --features scss`
  - `cargo check -p cssinjs`
  - `cargo clippy -p cssinjs`
  - `cargo test -p cssinjs --tests`
  - `cargo test -p dioxus-style-macro --features scss`
  - `cargo check -p preview_app --target wasm32-unknown-unknown`

## Phase 17. Ant Theme + Style Full API Parity (Planned)
- Status: Planned.
- Scope owner crates:
  - `/home/barracuda/comp/theme`
  - `/home/barracuda/comp/style`
  - `/home/barracuda/comp/cssinjs` (runtime/parity harness)

### 17.1 Required Ant API parity tests
- Ant API parity tests for:
  - `Keyframes`
  - `useStyleRegister`
  - `createCache`
  - `extractStyle`
  - `createTheme`
  - `Theme`
  - `useCacheToken`
  - `StyleContext`

### 17.2 End-to-end parity tests
- End-to-end component parity tests proving those APIs behave like Ant in:
  - runtime behavior (browser/live updates/HMR-safe)
  - SSR extraction behavior (cache + once semantics)

### 17.3 Contract tests for ordering/layering
- Contract tests for style ordering/layering semantics of `useStyleRegister` path:
  - stable order across rerender and rebuild
  - layer precedence invariants
  - no duplicate/stale registrations after burst updates

### 17.4 Playwright parity preparation
- Prepare Playwright parity flows that operate on dedicated preview probes for:
  - API-level registration and extraction checks
  - component-level style/token integration checks
  - ordering/layering burst checks
- Persist artifacts and reports under `/home/barracuda/comp/output/playwright/`.

### 17.5 Imported Wiring Process (Theme + Style -> CssInJs)
- Source plans imported into this phase:
  - `/home/barracuda/comp/theme/TODO.md`
  - `/home/barracuda/comp/style/TODO.md`
- Integration order (no phase inversion allowed):
  1. Lock API manifests and bridge contracts (`theme/internal` exports + style register metadata shape).
  2. Accept theme runtime ABI in cssinjs with no adapters (`useToken` tuple + cssVar metadata).
  3. Wire cache identity forwarding exactly (`salt`, `override`, `getComputedToken`, `cssVar`, `nonce`).
  4. Wire style register path so `hashId` and theme-generated css var metadata are forwarded unchanged.
  5. Keep style generator output as payload-only handoff (`CSSObject`/`CSSInterpolation`/`Keyframes` + `path/layer/order`) into cssinjs.
  6. Run extraction parity checks proving registration and extraction consume the same token/hash identity.
  7. Run preview wasm/browser parity only after steps 1-6 pass.
- Execution rule: stop on first failed gate; do not advance until fixed.

### 17.6 Imported Wiring Contracts (Blocking)
- Contract W-T1 (Theme ABI):
  - consume Ant-order tuple contract exactly: `[theme, token, hashId, realToken, cssVar, zeroRuntime]`.
- Contract W-T2 (Theme cache identity):
  - cssinjs must consume `useCacheToken` identity inputs without lossy remapping.
- Contract W-T3 (Theme->Style boundary):
  - style consumes theme outputs only (`token/hashId/cssVar/zeroRuntime`); no derivation logic in style.
- Contract W-S1 (Style generation boundary):
  - style exports pure generation functions only; no DOM ownership and no css string emission.
- Contract W-S2 (Style->CssInJs register payload):
  - payload includes deterministic `path`, `layer`, `order`, style/keyframe key, and unchanged theme identity fields.
- Contract W-S3 (Shared register lifecycle):
  - keyframes and static styles must use one register lifecycle; no split pipeline.
- Contract W-C1 (Css var naming ownership):
  - theme `genCssVar` output is source of truth; manual var-name reconstruction is disallowed.
- Contract W-C2 (Hash forwarding boundary):
  - style forwards `hashId` unchanged; cssinjs must not re-hash theme token payloads downstream.

### 17.7 Stop-Ship Gates (Imported)
- Stop immediately if style/theme code introduces direct DOM/document/window ownership in non-runtime crates.
- Stop immediately if style re-implements theme derivation (`seed/map/alias`) instead of consuming theme outputs.
- Stop immediately if keyframes register through a different pipeline than static styles.
- Stop immediately if register ordering depends on runtime timing or non-deterministic map iteration.
- Stop immediately if any adapter mutates forwarded `hashId`/css-var identity fields.
- Stop immediately if any required gate command fails (`-D warnings` gates are hard blockers).

### 17.8 Command Matrix (Imported, Run From `/home/barracuda/comp`)
- M0: Theme/Style/CssInJs core ABI and wiring gates (`C0-C4` + Style Gate A-D)
  - `cargo check --locked -p theme -p cssinjs -p style --all-targets`
  - `cargo check --locked -p theme -p cssinjs -p style --target wasm32-unknown-unknown`
  - `cargo clippy --locked -p theme -p cssinjs -p style --all-targets -- -D warnings`
  - `cargo clippy --locked -p theme -p cssinjs -p style --target wasm32-unknown-unknown -- -D warnings`
  - `cargo test --locked -p theme -p cssinjs -p style`
- M1: Style register contract gate (Style Gate A/C)
  - `cargo test -p cssinjs --test register_order`
  - `cargo test -p style --features cssinjs-bridge --test cssinjs_bridge_contract`
  - `cargo check -p style -p cssinjs`
  - `cargo clippy -p style -p cssinjs --all-targets -- -D warnings`
- M2: Cross-crate boundary gate (Style Gate D + Theme Phase 6/7 alignment)
  - `cargo check -p theme -p style -p cssinjs`
  - `cargo clippy -p theme -p style -p cssinjs --all-targets -- -D warnings`
- M3: Preview integration gate (Style Gate E + Theme preview gate)
  - `cargo check --locked -p theme -p style -p cssinjs -p preview_app --all-targets`
  - `cargo check --locked -p theme -p style -p cssinjs -p preview_app --target wasm32-unknown-unknown`
  - `cargo clippy --locked -p theme -p style -p cssinjs -p preview_app --all-targets -- -D warnings`
  - `cargo clippy --locked -p theme -p style -p cssinjs -p preview_app --target wasm32-unknown-unknown -- -D warnings`
- M4: CssInJs extraction/runtime parity gate (must stay green after M0-M3)
  - `cargo test -p cssinjs --test ant_parity_matrix`
  - `cargo test -p cssinjs --test ant_api_parity`
  - `cargo test -p cssinjs --test full_engine_parity`
  - `bash /home/barracuda/comp/.parity/cssinjs/scripts/verify.sh --profile parity`

#### Phase 17 Progress (v1, in progress)
- Added `cssinjs::ant_api` compatibility surface for Ant API parity work:
  - `Keyframes`
  - `create_theme` + `Theme`
  - `create_cache`
  - `extract_style` / `extract_style_output`
  - `use_style_register`
  - `use_cache_token`
  - `StyleContext`
- Added initial API-level parity coverage:
  - `/home/barracuda/comp/cssinjs/tests/ant_api_parity.rs`
- Added ordering/layering contract coverage for `use_style_register` path:
  - style replacement now preserves original relative order slot for the same identity scope.
  - runtime registry patch:
    - `/home/barracuda/comp/cssinjs/src/engine/cssinjs/registry.rs`
  - parity test:
    - `use_style_register_replacement_keeps_relative_order`
    - `/home/barracuda/comp/cssinjs/tests/ant_api_parity.rs`
- Added explicit `order` parity coverage across the runtime register path:
  - `CssInJsStyleInput.order`
  - `UseStyleRegisterOptions.order`
  - `StyleRegisterInput.order`
  - CSS output and record export now sort by explicit `order` first, then preserved registration slot
  - order updates reflow without duplicate or stale records
  - parity tests:
    - `/home/barracuda/comp/cssinjs/tests/register_order.rs`
    - `/home/barracuda/comp/style/tests/cssinjs_bridge_contract.rs`
- Added `StyleContext` parity coverage across both register paths:
  - verifies context-provided `layer`, `nonce`, and `hash_priority` are applied by:
    - `use_style_register`
    - `use_cache_token`
  - parity test:
    - `style_context_applies_layer_nonce_and_priority`
    - `/home/barracuda/comp/cssinjs/tests/ant_api_parity.rs`
- Verification slice executed (green):
  - `cargo check -p cssinjs --all-targets`
  - `cargo check -p cssinjs --target wasm32-unknown-unknown`
  - `cargo clippy -p cssinjs --all-targets -- -D warnings`
  - `cargo clippy -p cssinjs --target wasm32-unknown-unknown -- -D warnings`
  - `cargo test -p cssinjs --test ant_parity_matrix`
  - `cargo test -p cssinjs --test ant_api_parity`
  - `cargo test -p cssinjs --test full_engine_parity`
