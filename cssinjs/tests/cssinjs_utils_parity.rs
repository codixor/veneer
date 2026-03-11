use std::collections::BTreeMap;
use std::sync::Mutex;

use cssinjs::{
    StatisticValue, debug_reset_statistics_for_tests, statistic, statistic_build, statistic_token,
};

static TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn statistic_token_tracks_key_reads_and_flushes_component_payload() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    debug_reset_statistics_for_tests();

    let token = BTreeMap::from([
        ("colorPrimary".to_string(), "#1677ff".to_string()),
        ("padding".to_string(), "12".to_string()),
        ("radius".to_string(), "8".to_string()),
    ]);

    let tracked = statistic_token(&token);
    assert_eq!(
        tracked.token.get("colorPrimary").as_deref(),
        Some("#1677ff")
    );
    assert!(tracked.token.contains_key("padding"));

    tracked.flush(
        "Button",
        [
            ("colorPrimary", StatisticValue::from("#1677ff")),
            ("padding", StatisticValue::from(12_i64)),
        ],
    );

    let snapshot = statistic();
    let button = snapshot.get("Button").expect("button statistic");
    assert_eq!(
        button.global,
        vec!["colorPrimary".to_string(), "padding".to_string()]
    );
    assert_eq!(
        button.component.get("colorPrimary"),
        Some(&StatisticValue::String("#1677ff".to_string()))
    );
    assert_eq!(
        button.component.get("padding"),
        Some(&StatisticValue::Integer(12))
    );
}

#[test]
fn statistic_flush_merges_component_entries_and_updates_build_snapshot() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    debug_reset_statistics_for_tests();

    let token = BTreeMap::from([
        ("fontSize".to_string(), 14_i64),
        ("lineHeight".to_string(), 22_i64),
    ]);

    let tracked = statistic_token(&token);
    let _ = tracked.token.get("fontSize");
    tracked.flush("Typography", [("fontSize", 14_i64)]);

    let _ = tracked.token.get("lineHeight");
    tracked.flush("Typography", [("lineHeight", 22_i64)]);

    let snapshot = statistic();
    let typography = snapshot.get("Typography").expect("typography statistic");
    assert_eq!(
        typography.global,
        vec!["fontSize".to_string(), "lineHeight".to_string()]
    );
    assert_eq!(
        typography.component.get("fontSize"),
        Some(&StatisticValue::Integer(14))
    );
    assert_eq!(
        typography.component.get("lineHeight"),
        Some(&StatisticValue::Integer(22))
    );

    let build_snapshot = statistic_build();
    assert_eq!(build_snapshot, snapshot);
}
