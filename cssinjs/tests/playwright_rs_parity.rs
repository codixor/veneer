use playwright_rs::{Playwright, expect};

const DEFAULT_PARITY_URL: &str = "http://127.0.0.1:8086/cssinjs";

fn should_run_playwright_rs() -> bool {
    std::env::var("CSSINJS_PLAYWRIGHT_RS_RUN")
        .ok()
        .is_some_and(|value| value == "1")
}

fn parity_url() -> String {
    std::env::var("CSSINJS_PARITY_URL").unwrap_or_else(|_| DEFAULT_PARITY_URL.to_string())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires dx serve and playwright browsers; set CSSINJS_PLAYWRIGHT_RS_RUN=1"]
async fn ant_parity_page_smoke_playwright_rs() -> Result<(), Box<dyn std::error::Error>> {
    if !should_run_playwright_rs() {
        eprintln!("skipping: set CSSINJS_PLAYWRIGHT_RS_RUN=1 to run playwright-rs parity test");
        return Ok(());
    }

    let url = parity_url();
    let playwright = Playwright::launch().await?;
    let browser = playwright.chromium().launch().await?;
    let page = browser.new_page().await?;

    page.goto(&url, None).await?;

    expect(page.get_by_text("CSSinJS V6 Parity Lab", false).await)
        .to_be_visible()
        .await?;

    page.get_by_text("Clear runtime", false)
        .await
        .click(None)
        .await?;
    page.get_by_text("Theme Clear", false)
        .await
        .click(None)
        .await?;
    page.get_by_text("Style Register Unmount", false)
        .await
        .click(None)
        .await?;

    page.get_by_text("Style Register Mount", false)
        .await
        .click(None)
        .await?;
    let mounted_ok: bool = page
        .evaluate(
            r#"() => {
                const styles = Array.from(document.querySelectorAll('style[id^="dxcss-"]'));
                const boxStyles = styles.filter(s => (s.textContent || '').includes('.ant-style-register-probe'));
                return boxStyles.length === 1 && boxStyles.some(s => (s.textContent || '').includes('solid #1677ff'));
            }"#,
            None::<&()>,
        )
        .await?;
    assert!(mounted_ok, "mount parity check failed");

    page.get_by_text("Theme Light", false)
        .await
        .click(None)
        .await?;
    let theme_light_ok: bool = page
        .evaluate(
            r#"() => {
                const styles = Array.from(document.querySelectorAll('style[id^="dxcss-"]'));
                const themeStyles = styles.filter(s => (s.textContent || '').includes('--ant-parity-color-primary'));
                return themeStyles.length === 1 && themeStyles.some(s => (s.textContent || '').includes('#1677ff'));
            }"#,
            None::<&()>,
        )
        .await?;
    assert!(theme_light_ok, "theme light parity check failed");

    browser.close().await?;
    Ok(())
}
