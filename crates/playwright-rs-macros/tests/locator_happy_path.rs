use playwright_rs_macros::locator;

#[test]
fn css_selector_passes_through_verbatim() {
    let s: &'static str = locator!("#submit-button");
    assert_eq!(s, "#submit-button");
}

#[test]
fn class_selector_with_pseudo_passes() {
    let s: &'static str = locator!("button.primary:hover");
    assert_eq!(s, "button.primary:hover");
}

#[test]
fn attribute_selector_with_brackets_passes() {
    let s: &'static str = locator!("input[type='email'][required]");
    assert_eq!(s, "input[type='email'][required]");
}

#[test]
fn engine_prefixes_pass() {
    assert_eq!(locator!("css=#x"), "css=#x");
    assert_eq!(locator!("text=Hello"), "text=Hello");
    assert_eq!(locator!("role=button"), "role=button");
    assert_eq!(
        locator!("xpath=//button[@id='submit']"),
        "xpath=//button[@id='submit']"
    );
    assert_eq!(locator!("id=username"), "id=username");
    assert_eq!(locator!("data-testid=login-form"), "data-testid=login-form");
    assert_eq!(locator!("nth=0"), "nth=0");
}

#[test]
fn internal_engine_prefixes_pass() {
    // Playwright's `internal:*=` namespace is intentionally allow-listed
    // wholesale — there are many internal engines and they're all
    // legitimate even though they're not part of the public API.
    assert_eq!(locator!("internal:visible=true"), "internal:visible=true");
    assert_eq!(locator!("internal:role=button"), "internal:role=button");
}

#[test]
fn chained_selectors_with_engine_arrow_pass() {
    // Playwright's `>>` selector chaining isn't validated structurally
    // by this macro yet, but valid chains pass through.
    let s = locator!("div.container >> text=Submit");
    assert_eq!(s, "div.container >> text=Submit");
}

#[test]
fn quoted_brackets_inside_attribute_value_pass() {
    // Brackets inside a quoted attribute value must not throw off the
    // balance check.
    let s = locator!("[aria-label='go [back]']");
    assert_eq!(s, "[aria-label='go [back]']");
}
