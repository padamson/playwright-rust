//! Compile-time-validated selector macros for [playwright-rs].
//!
//! Most users get this crate transitively through `playwright-rs`
//! (the `macros` feature is on by default) and never need to depend on
//! it directly:
//!
//! ```rust,ignore
//! use playwright_rs::locator;
//!
//! let l = page.locator(locator!("#submit")).await;
//! ```
//!
//! See the `playwright-rs` crate root for the broader Observability /
//! macros story.

use proc_macro::TokenStream;
use quote::quote;
use syn::{LitStr, parse_macro_input};

/// Compile-time-validated Playwright selector. Expands to a `&'static
/// str` containing the same selector verbatim, with the validation a
/// one-time compile-time check.
///
/// Catches:
/// - empty / whitespace-only selectors
/// - unbalanced `[]`, `()`, `{}` brackets
/// - unknown engine prefixes (e.g. `foo=...` instead of one of
///   `css=`, `xpath=`, `text=`, `role=`, `id=`, `data-testid=`, `nth=`,
///   `internal:*=...`)
///
/// Anything else passes through. Selector engines have rich grammars
/// of their own (CSS, XPath, role-name semantics, `>>` chaining); this
/// macro punts on those for the v0.1 surface and lets the Playwright
/// server give the runtime error if the selector turns out invalid.
/// Future versions will tighten this up — see issue #81.
///
/// # Example
///
/// ```ignore
/// use playwright_rs::locator;
///
/// let _ok = locator!("#submit");
/// let _ok = locator!("text=Hello");
/// let _ok = locator!("xpath=//button[@id='submit']");
/// // let _bad = locator!("");                 // compile error: empty selector
/// // let _bad = locator!("button[disabled");  // compile error: unbalanced [
/// // let _bad = locator!("foo=bar");          // compile error: unknown engine
/// ```
#[proc_macro]
pub fn locator(input: TokenStream) -> TokenStream {
    let lit = parse_macro_input!(input as LitStr);
    let value = lit.value();

    if let Err(msg) = validate_selector(&value) {
        return syn::Error::new(lit.span(), msg).to_compile_error().into();
    }

    quote! { #lit }.into()
}

/// Validate a Playwright selector string. Returns `Err(message)` when
/// the selector is rejectable at compile time; the message is the
/// diagnostic shown to the user.
fn validate_selector(s: &str) -> Result<(), String> {
    if s.trim().is_empty() {
        return Err("selector is empty or whitespace-only".to_string());
    }

    check_balanced_brackets(s)?;

    if let Some((engine, _rest)) = split_engine_prefix(s)
        && !is_known_engine(engine)
    {
        return Err(format!(
            "unknown selector engine `{engine}=...`; expected one of \
             css, xpath, text, role, id, data-testid, nth, or an \
             `internal:*=` prefix"
        ));
    }

    Ok(())
}

/// Track depth of `()`, `[]`, `{}`. Returns `Err` on the first
/// imbalance — either an unmatched closer or any unclosed opener at
/// end of input. Bracket characters inside string literals (`"..."`,
/// `'...'`) are skipped because Playwright selectors can carry quoted
/// values like `text="Hello"` or `[aria-label='go [back]']`.
fn check_balanced_brackets(s: &str) -> Result<(), String> {
    let mut stack: Vec<char> = Vec::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                // skip the escaped char
                let _ = chars.next();
            }
            '"' | '\'' => {
                // skip until matching closing quote (or end of input);
                // honour `\<x>` so an escaped quote inside the value
                // doesn't terminate the run
                let quote = c;
                loop {
                    match chars.next() {
                        None => break,
                        Some('\\') => {
                            let _ = chars.next();
                        }
                        Some(q) if q == quote => break,
                        Some(_) => {}
                    }
                }
            }
            '(' | '[' | '{' => stack.push(c),
            ')' | ']' | '}' => match (stack.pop(), c) {
                (Some('('), ')') | (Some('['), ']') | (Some('{'), '}') => {}
                (Some(open), close) => {
                    return Err(format!(
                        "mismatched bracket: `{open}` opened, `{close}` closed"
                    ));
                }
                (None, close) => {
                    return Err(format!("unmatched closing `{close}`"));
                }
            },
            _ => {}
        }
    }

    if let Some(open) = stack.pop() {
        return Err(format!("unclosed `{open}`"));
    }

    Ok(())
}

/// If the selector starts with `<word>=`, return `(word, rest)`.
/// Returns `None` for selectors that don't carry an explicit engine
/// prefix (CSS is the default).
///
/// `internal:visible=` and similar internal-prefixed engines also have
/// an `=`, but they all start with `internal:` so we treat the
/// `internal:` namespace specially as known.
fn split_engine_prefix(s: &str) -> Option<(&str, &str)> {
    let s = s.trim_start();
    let eq_pos = s.find('=')?;
    let prefix = &s[..eq_pos];
    let rest = &s[eq_pos + 1..];

    // The prefix must look like an identifier (letters, digits, `-`,
    // `:`). Bail otherwise — `[attr=val]` and similar shouldn't be
    // misread as an engine.
    if prefix.is_empty()
        || !prefix
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == ':')
    {
        return None;
    }

    Some((prefix, rest))
}

fn is_known_engine(engine: &str) -> bool {
    matches!(
        engine,
        "css" | "xpath" | "text" | "role" | "id" | "data-testid" | "nth"
    ) || engine.starts_with("internal:")
}
