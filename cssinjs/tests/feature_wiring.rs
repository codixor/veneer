#[test]
fn ssr_liveview_feature_alias_enables_both_lanes() {
    let has_alias = cfg!(feature = "ssr_liveview");
    let has_ssr = cfg!(feature = "ssr");
    let has_liveview = cfg!(feature = "liveview");

    if has_alias {
        assert!(has_ssr);
        assert!(has_liveview);
    }
}
