#[test]
fn ant_reference_hash_layout_is_prefix_plus_hash() {
    // Source snapshot parity check against local Ant cssinjs reference.
    let source =
        include_str!("../../react-components/cssinjs-react/cssinjs/src/hooks/useCacheToken.tsx");

    assert!(source.contains("const hashPrefix"));
    assert!(source.contains("css-dev-only-do-not-override"));
    assert!(source.contains("const hashId = hash(mergedSalt);"));
    assert!(source.contains("const hashCls = `${hashPrefix}-${hashId}`;"));
}
