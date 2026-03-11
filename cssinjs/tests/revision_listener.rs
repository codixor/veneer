use cssinjs::CssInJsRuntime;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

#[test]
fn revision_listener_receives_and_unsubscribes_cleanly() {
    let runtime = CssInJsRuntime;

    runtime.clear_dev_styles();
    runtime.clear();

    let calls = Arc::new(AtomicUsize::new(0));
    let last_revision = Arc::new(AtomicU64::new(0));

    let calls_for_listener = Arc::clone(&calls);
    let last_revision_for_listener = Arc::clone(&last_revision);
    let listener_id = runtime.subscribe_revision_listener(Arc::new(move |revision| {
        calls_for_listener.fetch_add(1, Ordering::Relaxed);
        last_revision_for_listener.store(revision, Ordering::Relaxed);
    }));

    let reg = runtime.upsert_dev_style(
        "revision-listener",
        "probe",
        ".revision-listener-probe{color:#1677ff;}",
    );
    assert!(reg.is_some());
    assert!(calls.load(Ordering::Relaxed) >= 1);
    assert!(last_revision.load(Ordering::Relaxed) >= runtime.revision());

    let _ = runtime.remove_dev_style("revision-listener", "probe");
    let calls_after_remove = calls.load(Ordering::Relaxed);
    assert!(calls_after_remove >= 2);

    assert!(runtime.unsubscribe_revision_listener(listener_id));

    let before_unsub_mutation = calls.load(Ordering::Relaxed);
    let reg2 = runtime.upsert_dev_style(
        "revision-listener",
        "probe",
        ".revision-listener-probe{color:#0958d9;}",
    );
    assert!(reg2.is_some());
    assert_eq!(calls.load(Ordering::Relaxed), before_unsub_mutation);

    let _ = runtime.remove_dev_style("revision-listener", "probe");
    runtime.clear_dev_styles();
    runtime.clear();
}
