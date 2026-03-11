use std::sync::{Arc, Mutex};

use cssinjs::{CssInJs, CssInJsStyleInput};

static TEST_LOCK: Mutex<()> = Mutex::new(());

fn reset_runtime() {
    CssInJs::debug_reset_runtime_for_tests();
}

#[test]
fn style_revision_tracks_real_registry_mutations_only() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    reset_runtime();

    let rev0 = CssInJs::revision();

    let mut first = CssInJsStyleInput::new("rev-btn", ".rev-btn{opacity:0.8;}");
    first.identity_scope = Some(Arc::<str>::from("style|rev-btn"));
    first.hash_class = Some(Arc::<str>::from("rev-btn-hash"));
    let _ = CssInJs::register(first.clone()).expect("register");
    let rev1 = CssInJs::revision();
    assert!(rev1 > rev0);

    let _ = CssInJs::register(first).expect("duplicate register");
    let rev2 = CssInJs::revision();
    assert_eq!(rev2, rev1);

    let mut replaced = CssInJsStyleInput::new("rev-btn", ".rev-btn{opacity:1;}");
    replaced.identity_scope = Some(Arc::<str>::from("style|rev-btn"));
    replaced.hash_class = Some(Arc::<str>::from("rev-btn-hash"));
    let reg = CssInJs::register(replaced).expect("replacement register");
    let rev3 = CssInJs::revision();
    assert!(rev3 > rev2);

    assert!(!CssInJs::unregister("missing-cache-key"));
    let rev4 = CssInJs::revision();
    assert_eq!(rev4, rev3);

    assert!(CssInJs::unregister(reg.cache_key.as_str()));
    let rev5 = CssInJs::revision();
    assert!(rev5 > rev4);

    CssInJs::clear();
    let rev6 = CssInJs::revision();
    assert_eq!(rev6, rev5);
}
