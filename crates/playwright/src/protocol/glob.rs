// URL glob matching for client-side waits (e.g. `Frame::wait_for_url`).
//
// Playwright URL globs: `*` matches any run of characters except `/` (one
// path segment), `**` matches any run including `/` (across segments), and
// `.` is a literal dot. Everything else is matched literally. A pattern that
// fails to translate to a valid regex never matches.

/// Returns whether `text` matches the glob `pattern`.
///
/// `*` matches within a path segment, `**` crosses segments, `.` is literal.
/// Returns `false` if the translated pattern is not a valid regex.
pub(crate) fn glob_match(pattern: &str, text: &str) -> bool {
    let regex_str = pattern
        .replace('.', "\\.")
        .replace("**", "\x00") // placeholder so the next step skips `**`
        .replace('*', "[^/]*")
        .replace('\x00', ".*"); // restore `**` as cross-segment match
    let regex_str = format!("^{}$", regex_str);
    regex::Regex::new(&regex_str)
        .map(|re| re.is_match(text))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::glob_match;

    #[test]
    fn exact_match_is_anchored() {
        assert!(glob_match("https://example.com/", "https://example.com/"));
        // anchored at both ends: a trailing extra segment must not match
        assert!(!glob_match("https://example.com/", "https://example.com/x"));
        // ...nor a prefix
        assert!(!glob_match("example.com", "an.example.com"));
    }

    #[test]
    fn single_star_stays_within_one_segment() {
        assert!(glob_match(
            "https://example.com/*",
            "https://example.com/foo"
        ));
        // `*` does not cross a `/`
        assert!(!glob_match(
            "https://example.com/*",
            "https://example.com/foo/bar"
        ));
        // `*` matches the empty string
        assert!(glob_match("https://example.com/*", "https://example.com/"));
    }

    #[test]
    fn double_star_crosses_segments() {
        assert!(glob_match(
            "https://example.com/**",
            "https://example.com/a/b/c"
        ));
        assert!(glob_match(
            "**/*.png",
            "https://cdn.example.com/img/logo.png"
        ));
        // wrong extension fails
        assert!(!glob_match(
            "**/*.png",
            "https://cdn.example.com/img/logo.jpg"
        ));
    }

    #[test]
    fn dot_is_literal_not_wildcard() {
        assert!(glob_match("https://a.com/x", "https://a.com/x"));
        // the literal dot must not match an arbitrary character
        assert!(!glob_match("https://a.com/x", "https://axcom/x"));
    }

    #[test]
    fn extension_globs() {
        assert!(glob_match("*.png", "logo.png"));
        // single `*` stops at `/`
        assert!(!glob_match("*.png", "img/logo.png"));
        // `**` crosses `/`
        assert!(glob_match("**.png", "img/logo.png"));
    }

    #[test]
    fn invalid_translation_does_not_match() {
        // An unbalanced regex group from a stray metachar must yield false,
        // not panic.
        assert!(!glob_match("[unterminated", "anything"));
    }
}
