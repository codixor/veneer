use std::collections::BTreeSet;

use serde_json::Value;

const MANIFEST: &str = include_str!("../../.parity/cssinjs/manifest/ant_surface_manifest.json");
const ANT_CSSINJS_PACKAGE_JSON: &str =
    include_str!("../../react-components/ant-design/node_modules/@ant-design/cssinjs/package.json");
const ANT_CSSINJS_UTILS_PACKAGE_JSON: &str = include_str!(
    "../../react-components/ant-design/node_modules/@ant-design/cssinjs-utils/package.json"
);
const ANT_CSSINJS_INDEX_DTS: &str = include_str!(
    "../../react-components/ant-design/node_modules/@ant-design/cssinjs/es/index.d.ts"
);
const ANT_CSSINJS_UTILS_INDEX_DTS: &str = include_str!(
    "../../react-components/ant-design/node_modules/@ant-design/cssinjs-utils/es/index.d.ts"
);

const LOCAL_LIB_RS: &str = include_str!("../src/lib.rs");
const LOCAL_ENGINE_MOD_RS: &str = include_str!("../src/engine/mod.rs");
const LOCAL_ENGINE_STYLE_PROVIDER_RS: &str = include_str!("../src/engine/style_provider.rs");
const LOCAL_ENGINE_VAR_RS: &str = include_str!("../src/engine/cssinjs/var.rs");
const THEME_RUNTIME_COMPUTED_TOKEN_RS: &str =
    include_str!("../../theme/src/runtime/computed_token.rs");
const THEME_UTIL_MOD_RS: &str = include_str!("../../theme/src/util/mod.rs");
const THEME_UTIL_GEN_STYLE_UTILS_RS: &str = include_str!("../../theme/src/util/gen_style_utils.rs");

fn parse_manifest() -> Value {
    serde_json::from_str(MANIFEST).expect("cssinjs surface manifest must be valid json")
}

fn read_string(map: &Value, key: &str) -> String {
    map.get(key)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("manifest missing string key '{key}'"))
        .to_string()
}

fn read_list(map: &Value, key: &str) -> Vec<String> {
    map.get(key)
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("manifest missing array key '{key}'"))
        .iter()
        .map(|value| value.as_str().unwrap_or_default().to_string())
        .collect()
}

fn read_marker_pairs(map: &Value, key: &str) -> Vec<(String, String)> {
    map.get(key)
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("manifest missing array key '{key}'"))
        .iter()
        .map(|value| {
            let ant = read_string(value, "ant");
            let marker = read_string(value, "rust_marker");
            (ant, marker)
        })
        .collect()
}

fn package_version(package_json: &str) -> String {
    let value: Value =
        serde_json::from_str(package_json).expect("package.json payload must be valid json");
    value
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn combined_workspace_sources() -> String {
    [
        LOCAL_ENGINE_MOD_RS,
        LOCAL_ENGINE_STYLE_PROVIDER_RS,
        LOCAL_ENGINE_VAR_RS,
        THEME_RUNTIME_COMPUTED_TOKEN_RS,
        THEME_UTIL_MOD_RS,
        THEME_UTIL_GEN_STYLE_UTILS_RS,
    ]
    .join("\n")
}

#[test]
fn locked_package_versions_match_local_node_modules() {
    let manifest = parse_manifest();
    let packages = manifest
        .get("source_packages")
        .expect("manifest missing source_packages");

    assert_eq!(
        read_string(packages, "cssinjs"),
        package_version(ANT_CSSINJS_PACKAGE_JSON)
    );
    assert_eq!(
        read_string(packages, "cssinjs_utils"),
        package_version(ANT_CSSINJS_UTILS_PACKAGE_JSON)
    );
}

#[test]
fn locked_ant_entrypoints_match_local_runtime_and_type_surfaces() {
    let manifest = parse_manifest();

    for symbol in read_list(&manifest, "ant_cssinjs_runtime_exports") {
        assert!(
            ANT_CSSINJS_INDEX_DTS.contains(symbol.as_str()),
            "locked @ant-design/cssinjs runtime export '{symbol}' missing from local entrypoint"
        );
    }
    for symbol in read_list(&manifest, "ant_cssinjs_type_exports") {
        assert!(
            ANT_CSSINJS_INDEX_DTS.contains(symbol.as_str()),
            "locked @ant-design/cssinjs type export '{symbol}' missing from local entrypoint"
        );
    }
    for symbol in read_list(&manifest, "ant_cssinjs_utils_runtime_exports") {
        assert!(
            ANT_CSSINJS_UTILS_INDEX_DTS.contains(symbol.as_str()),
            "locked @ant-design/cssinjs-utils runtime export '{symbol}' missing from local entrypoint"
        );
    }
    for symbol in read_list(&manifest, "ant_cssinjs_utils_type_exports") {
        assert!(
            ANT_CSSINJS_UTILS_INDEX_DTS.contains(symbol.as_str()),
            "locked @ant-design/cssinjs-utils type export '{symbol}' missing from local entrypoint"
        );
    }
}

#[test]
fn implemented_top_level_cssinjs_symbols_exist_in_local_lib_surface() {
    let manifest = parse_manifest();

    for (ant, marker) in read_marker_pairs(&manifest, "implemented_at_cssinjs_top_level") {
        assert!(
            LOCAL_LIB_RS.contains(marker.as_str()),
            "manifest says Ant symbol '{ant}' is top-level in Rust cssinjs, but lib.rs is missing marker '{marker}'"
        );
    }
}

#[test]
fn workspace_but_not_top_level_symbols_exist_in_workspace_sources() {
    let manifest = parse_manifest();
    let workspace_sources = combined_workspace_sources();

    for (ant, marker) in read_marker_pairs(
        &manifest,
        "implemented_in_workspace_but_not_cssinjs_top_level",
    ) {
        assert!(
            workspace_sources.contains(marker.as_str()),
            "manifest says Ant symbol '{ant}' exists elsewhere in workspace, but sources are missing marker '{marker}'"
        );
    }
}

#[test]
fn runtime_classification_covers_locked_ant_runtime_surface() {
    let manifest = parse_manifest();

    let expected = read_list(&manifest, "ant_cssinjs_runtime_exports")
        .into_iter()
        .chain(read_list(&manifest, "ant_cssinjs_utils_runtime_exports"))
        .collect::<BTreeSet<_>>();

    let classified = read_marker_pairs(&manifest, "implemented_at_cssinjs_top_level")
        .into_iter()
        .map(|(ant, _)| ant)
        .chain(
            read_marker_pairs(
                &manifest,
                "implemented_in_workspace_but_not_cssinjs_top_level",
            )
            .into_iter()
            .map(|(ant, _)| ant),
        )
        .chain(read_list(&manifest, "missing_from_workspace"))
        .collect::<BTreeSet<_>>();

    assert_eq!(
        classified, expected,
        "runtime classification must cover every locked Ant runtime export exactly once"
    );
}
