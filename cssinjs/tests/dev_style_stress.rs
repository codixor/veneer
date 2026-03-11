use std::sync::Mutex;

use cssinjs::CssInJsRuntime;

static TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn dev_style_rapid_updates_remain_stable() {
    let _guard = TEST_LOCK.lock().expect("test lock");
    let runtime = CssInJsRuntime;
    runtime.clear_dev_styles();
    runtime.clear();

    const COMPONENTS: usize = 8;
    const STYLES: usize = 12;
    const ITERS: usize = 120;

    for iter in 0..ITERS {
        for c in 0..COMPONENTS {
            for s in 0..STYLES {
                let component_key = format!("cmp-{c}");
                let style_id = format!("slot-{s}");
                let css = format!(
                    ".c-{c}-s-{s}{{opacity:{};transform:translateX({}px);}}",
                    (iter + s) % 10,
                    (iter + c + s) % 21
                );
                let record = runtime.upsert_dev_style(
                    component_key.as_str(),
                    style_id.as_str(),
                    css.as_str(),
                );
                assert!(record.is_some());
            }
        }
    }

    let styles = runtime.list_dev_styles();
    assert_eq!(styles.len(), COMPONENTS * STYLES);

    for c in 0..COMPONENTS {
        for s in 0..STYLES {
            let component_key = format!("cmp-{c}");
            let style_id = format!("slot-{s}");
            assert!(runtime.remove_dev_style(component_key.as_str(), style_id.as_str()));
        }
    }

    assert!(runtime.list_dev_styles().is_empty());
    runtime.clear();
}
