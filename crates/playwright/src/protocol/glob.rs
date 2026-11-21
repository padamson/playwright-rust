// URL glob matching, shared by client-side waits (`Frame::wait_for_url`) and
// client-side route matching (`Page::route`, `BrowserContext::route`). Both
// resolve each pattern against the context's base URL first
// (`GlobMatcher::for_url`), as the driver's `urlMatches` does.
//
// This is a port of the driver's `globToRegexPattern`
// (`packages/isomorphic/urlMatch.ts`), not an independent implementation.
// Both sides have to agree on whether a URL matches: the server decides
// which requests to report, and the client decides which handler to run, so
// a pattern the two read differently means a route event nobody answers and
// a request that hangs until it times out.

#[cfg(verus_only)]
use vstd::prelude::*;

/// Whether the driver escapes `c` when translating a glob to a regex.
///
/// Kept as the driver's exact set (`escapedChars`) rather than deferring to
/// `regex::escape`, because `{`, `}` and `,` are glob syntax here
/// (alternation) and must not be escaped, and because a wider set would
/// silently diverge.
#[cfg_attr(verus_only, verus_spec(r =>
    ensures r == spec_is_escaped(c)
))]
fn is_escaped(c: char) -> bool {
    matches!(
        c,
        '$' | '^' | '+' | '.' | '*' | '(' | ')' | '|' | '\\' | '?' | '{' | '}' | '[' | ']'
    )
}

#[cfg(verus_only)]
verus! {

spec fn spec_is_escaped(c: char) -> bool {
    c == '$' || c == '^' || c == '+' || c == '.' || c == '*' || c == '(' || c == ')'
        || c == '|' || c == '\\' || c == '?' || c == '{' || c == '}' || c == '['
        || c == ']'
}

/// `c` as a literal in the pattern: escaped exactly when the driver escapes it.
spec fn literal(c: char) -> Seq<char> {
    if spec_is_escaped(c) { seq!['\\', c] } else { seq![c] }
}

/// The index of the last `*` in the run of them that starts at `i`.
spec fn star_run_end(g: Seq<char>, i: int) -> int
    decreases g.len() - i,
{
    if 0 <= i && i + 1 < g.len() && g[i + 1] == '*' {
        star_run_end(g, i + 1)
    } else {
        i
    }
}

/// The pattern for `g[i..]` appended to `acc`, one glob token at a time,
/// reading from inside a group when `in_group`.
spec fn translate_from(g: Seq<char>, i: int, in_group: bool, acc: Seq<char>) -> Option<Seq<char>>
    decreases g.len() - i,
    via translate_from_decreases
{
    if i < 0 || i >= g.len() {
        if in_group { None } else { Some(acc.push('$')) }
    } else if g[i] == '\\' && i + 1 < g.len() {
        translate_from(g, i + 2, in_group, acc + literal(g[i + 1]))
    } else if g[i] == '*' {
        let j = star_run_end(g, i);
        if j == i {
            translate_from(g, i + 1, in_group, acc + seq!['(', '[', '^', '/', ']', '*', ')'])
        } else if j + 1 < g.len() && g[j + 1] == '/' {
            let piece = if i > 0 && g[i - 1] == '/' {
                seq!['(', '(', '.', '+', '/', ')', '|', ')']
            } else {
                seq!['(', '.', '*', '/', ')']
            };
            translate_from(g, j + 2, in_group, acc + piece)
        } else {
            translate_from(g, j + 1, in_group, acc + seq!['(', '.', '*', ')'])
        }
    } else if g[i] == '{' {
        if in_group { None } else { translate_from(g, i + 1, true, acc.push('(')) }
    } else if g[i] == '}' {
        if in_group { translate_from(g, i + 1, false, acc.push(')')) } else { None }
    } else if g[i] == ',' {
        if in_group {
            translate_from(g, i + 1, in_group, acc.push('|'))
        } else {
            translate_from(g, i + 1, in_group, acc + seq!['\\', ','])
        }
    } else {
        translate_from(g, i + 1, in_group, acc + literal(g[i]))
    }
}

/// The whole pattern for `g`.
spec fn translate(g: Seq<char>) -> Option<Seq<char>> {
    translate_from(g, 0, false, seq!['^'])
}

/// Whether the braces in `g[i..]` close without nesting, reading from inside
/// a group when `in_group` and skipping each escaped char.
spec fn braces_ok_from(g: Seq<char>, i: int, in_group: bool) -> bool
    decreases g.len() - i,
{
    if i < 0 || i >= g.len() {
        !in_group
    } else if g[i] == '\\' && i + 1 < g.len() {
        braces_ok_from(g, i + 2, in_group)
    } else if g[i] == '{' {
        !in_group && braces_ok_from(g, i + 1, true)
    } else if g[i] == '}' {
        in_group && braces_ok_from(g, i + 1, false)
    } else {
        braces_ok_from(g, i + 1, in_group)
    }
}

spec fn braces_ok(g: Seq<char>) -> bool {
    braces_ok_from(g, 0, false)
}

proof fn lemma_star_run_end(g: Seq<char>, i: int)
    requires
        0 <= i < g.len(),
    ensures
        i <= star_run_end(g, i) < g.len(),
        forall|m: int| i < m <= star_run_end(g, i) ==> g[m] == '*',
        star_run_end(g, i) + 1 < g.len() ==> g[star_run_end(g, i) + 1] != '*',
    decreases g.len() - i,
{
    if i + 1 < g.len() && g[i + 1] == '*' {
        lemma_star_run_end(g, i + 1);
    }
}

#[verifier::decreases_by]
proof fn translate_from_decreases(g: Seq<char>, i: int, in_group: bool, acc: Seq<char>) {
    if 0 <= i < g.len() {
        lemma_star_run_end(g, i);
    }
}

/// Chars that are not `\`, `{` or `}` leave the brace check unchanged.
proof fn lemma_braces_skip(g: Seq<char>, i: int, k: int, in_group: bool)
    requires
        0 <= i <= k <= g.len(),
        forall|m: int| i <= m < k ==> g[m] != '\\' && g[m] != '{' && g[m] != '}',
    ensures
        braces_ok_from(g, i, in_group) == braces_ok_from(g, k, in_group),
    decreases k - i,
{
    if i < k {
        lemma_braces_skip(g, i + 1, k, in_group);
    }
}

/// The translation of `g[i..]` fails exactly when its braces are malformed,
/// and when it succeeds it ends in `$`.
proof fn lemma_translate_from(g: Seq<char>, i: int, in_group: bool, acc: Seq<char>)
    requires
        0 <= i <= g.len(),
    ensures
        translate_from(g, i, in_group, acc) is None <==> !braces_ok_from(g, i, in_group),
        translate_from(g, i, in_group, acc) matches Some(t)
            ==> t.len() > acc.len() && t.last() == '$' && (acc.len() > 0 ==> t[0] == acc[0]),
    decreases g.len() - i,
{
    if i < g.len() {
        if g[i] == '\\' && i + 1 < g.len() {
            lemma_translate_from(g, i + 2, in_group, acc + literal(g[i + 1]));
        } else if g[i] == '*' {
            lemma_star_run_end(g, i);
            let j = star_run_end(g, i);
            if j == i {
                lemma_translate_from(g, i + 1, in_group, acc + seq!['(', '[', '^', '/', ']', '*', ')']);
            } else if j + 1 < g.len() && g[j + 1] == '/' {
                let piece = if i > 0 && g[i - 1] == '/' {
                    seq!['(', '(', '.', '+', '/', ')', '|', ')']
                } else {
                    seq!['(', '.', '*', '/', ')']
                };
                lemma_braces_skip(g, i, j + 2, in_group);
                lemma_translate_from(g, j + 2, in_group, acc + piece);
            } else {
                lemma_braces_skip(g, i, j + 1, in_group);
                lemma_translate_from(g, j + 1, in_group, acc + seq!['(', '.', '*', ')']);
            }
        } else if g[i] == '{' {
            if !in_group {
                lemma_translate_from(g, i + 1, true, acc.push('('));
            }
        } else if g[i] == '}' {
            if in_group {
                lemma_translate_from(g, i + 1, false, acc.push(')'));
            }
        } else if g[i] == ',' {
            lemma_translate_from(g, i + 1, in_group, if in_group { acc.push('|') } else { acc + seq!['\\', ','] });
        } else {
            lemma_translate_from(g, i + 1, in_group, acc + literal(g[i]));
        }
    }
}

/// [`literal`] of each char of `s[i..]`, in order.
spec fn literals_from(s: Seq<char>, i: int) -> Seq<char>
    decreases s.len() - i,
{
    if i < 0 || i >= s.len() {
        seq![]
    } else {
        literal(s[i]) + literals_from(s, i + 1)
    }
}

/// Whether `c` is glob syntax when it is not escaped.
spec fn is_glob_syntax(c: char) -> bool {
    c == '\\' || c == '*' || c == '{' || c == '}' || c == ','
}

/// `s` with a backslash before each char.
spec fn quote(s: Seq<char>) -> Seq<char> {
    Seq::new(2 * s.len(), |k: int| if k % 2 == 0 { '\\' } else { s[k / 2] })
}

proof fn lemma_plain_from(g: Seq<char>, i: int, acc: Seq<char>)
    requires
        0 <= i <= g.len(),
        forall|m: int| i <= m < g.len() ==> !is_glob_syntax(#[trigger] g[m]),
    ensures
        translate_from(g, i, false, acc) == Some(acc + literals_from(g, i) + seq!['$']),
    decreases g.len() - i,
{
    if i < g.len() {
        lemma_plain_from(g, i + 1, acc + literal(g[i]));
        assert(acc + literal(g[i]) + literals_from(g, i + 1) =~= acc + literals_from(g, i));
    } else {
        assert(acc + literals_from(g, i) + seq!['$'] =~= acc.push('$'));
    }
}

/// A glob without glob syntax comes out as its chars, each as [`literal`].
proof fn lemma_plain(g: Seq<char>)
    requires
        forall|m: int| 0 <= m < g.len() ==> !is_glob_syntax(#[trigger] g[m]),
    ensures
        translate(g) == Some(seq!['^'] + literals_from(g, 0) + seq!['$']),
{
    lemma_plain_from(g, 0, seq!['^']);
}

proof fn lemma_quoted_from(s: Seq<char>, m: int, acc: Seq<char>)
    requires
        0 <= m <= s.len(),
    ensures
        translate_from(quote(s), 2 * m, false, acc) == Some(acc + literals_from(s, m) + seq!['$']),
    decreases s.len() - m,
{
    let q = quote(s);
    if m < s.len() {
        assert(q[2 * m] == '\\');
        assert(q[2 * m + 1] == s[m]);
        lemma_quoted_from(s, m + 1, acc + literal(s[m]));
        assert(acc + literal(s[m]) + literals_from(s, m + 1) =~= acc + literals_from(s, m));
    } else {
        assert(acc + literals_from(s, m) + seq!['$'] =~= acc.push('$'));
    }
}

/// Escaping every char of any text, glob syntax included, gives the text
/// back, each char as [`literal`].
proof fn lemma_quoted(s: Seq<char>)
    ensures
        translate(quote(s)) == Some(seq!['^'] + literals_from(s, 0) + seq!['$']),
{
    lemma_quoted_from(s, 0, seq!['^']);
}

} // verus!

#[cfg_attr(verus_only, verus_spec(
    ensures
        final(out)@ == old(out)@ + literal(c)
))]
fn push_escaped(out: &mut Vec<char>, c: char) {
    if is_escaped(c) {
        out.push('\\');
    }
    out.push(c);
}

/// `extend_from_slice`, whose vstd spec is per element, stated as the
/// concatenation the proofs work in.
#[cfg_attr(verus_only, verus_spec(
    ensures
        final(out)@ == old(out)@ + s@
))]
fn push_all(out: &mut Vec<char>, s: &[char]) {
    out.extend_from_slice(s);
    #[cfg(verus_only)]
    proof! {
        assert(out@ =~= old(out)@ + s@);
    }
}

/// Translates a Playwright URL glob into an anchored regex pattern.
///
/// Ported from the driver so the two stay in step:
///
/// - `\x` is an escape; the next character is always literal.
/// - `*` matches within one path segment.
/// - `**` crosses segments. Between slashes (`/**/`) it also matches *zero*
///   directories, so `a/**/b` matches `a/b`.
/// - `{a,b}` is alternation. Nested or unbalanced braces are an error.
/// - Everything else is literal.
///
/// Returns `None` for a malformed pattern (nested `{`, unmatched `}` or `{`),
/// which the driver rejects by throwing.
pub(crate) fn glob_to_regex_pattern(glob: &str) -> Option<String> {
    let glob: Vec<char> = glob.chars().collect();
    glob_to_regex_chars(&glob).map(|pattern| pattern.into_iter().collect())
}

/// [`glob_to_regex_pattern`] over chars, which is the part Verus proves.
///
/// The loop follows the driver's index by index, including its reads of
/// `glob[i - 1]` and `glob[i + 1]`, so the two can be compared line for line.
/// Verus checks every index against the bounds, that each loop ends, that
/// the output is the spec `translate`, and that it fails exactly when
/// `braces_ok` does. `translate` restates this loop a token at a time, so
/// `lemma_plain` and `lemma_quoted` pin down what it does with literal text
/// without repeating it.
#[cfg_attr(verus_only, verus_spec(r =>
    ensures
        match translate(glob@) {
            Some(p) => r matches Some(v) && v@ == p,
            None => r is None,
        },
        r is None <==> !braces_ok(glob@),
        r matches Some(v) ==> v@[0] == '^' && v@.last() == '$',
))]
fn glob_to_regex_chars(glob: &[char]) -> Option<Vec<char>> {
    #[cfg(verus_only)]
    proof! {
        lemma_translate_from(glob@, 0, false, seq!['^']);
    }
    let mut out: Vec<char> = Vec::with_capacity(glob.len());
    out.push('^');
    let mut in_group = false;
    let mut i: usize = 0;

    #[cfg_attr(verus_only, verus_spec(
        invariant
            i <= glob.len(),
            translate_from(glob@, i as int, in_group, out@) == translate(glob@),
            translate(glob@) is None <==> !braces_ok(glob@),
        decreases glob.len() - i
    ))]
    while i < glob.len() {
        let c = glob[i];
        if c == '\\' && i + 1 < glob.len() {
            i += 1;
            push_escaped(&mut out, glob[i]);
        } else if c == '*' {
            let char_before_is_slash = i > 0 && glob[i - 1] == '/';
            // The rest of the run of stars, read ahead with `j` so `i` is
            // seen to move only forward.
            let mut j = i;
            #[cfg_attr(verus_only, verus_spec(
                invariant
                    i <= j < glob.len(),
                    star_run_end(glob@, i as int) == star_run_end(glob@, j as int),
                decreases glob.len() - j
            ))]
            while j + 1 < glob.len() && glob[j + 1] == '*' {
                j += 1;
            }
            let crosses_segments = j > i;
            i = j;
            if crosses_segments {
                if i + 1 < glob.len() && glob[i + 1] == '/' {
                    if char_before_is_slash {
                        // `/**/` also matches zero directories.
                        push_all(&mut out, &['(', '(', '.', '+', '/', ')', '|', ')']);
                    } else {
                        push_all(&mut out, &['(', '.', '*', '/', ')']);
                    }
                    i += 1;
                } else {
                    push_all(&mut out, &['(', '.', '*', ')']);
                }
            } else {
                push_all(&mut out, &['(', '[', '^', '/', ']', '*', ')']);
            }
        } else if c == '{' {
            if in_group {
                return None;
            }
            in_group = true;
            out.push('(');
        } else if c == '}' {
            if !in_group {
                return None;
            }
            in_group = false;
            out.push(')');
        } else if c == ',' {
            if in_group {
                out.push('|');
            } else {
                // Needs no escape, but the driver escapes it.
                push_all(&mut out, &['\\', ',']);
            }
        } else {
            // A trailing lone backslash lands here, as in the driver, and
            // comes out escaped.
            push_escaped(&mut out, c);
        }
        i += 1;
    }

    if in_group {
        return None;
    }

    out.push('$');
    Some(out)
}

/// Reads an `http(s)` base URL as the `ws(s)` URL a WebSocket pattern is
/// resolved against, the scheme matched without regard to case. Any other
/// base is left as it is.
pub(crate) fn to_web_socket_base_url(base_url: &str) -> String {
    match base_url.split_once("://") {
        Some((scheme, rest)) if scheme.eq_ignore_ascii_case("http") => format!("ws://{rest}"),
        Some((scheme, rest)) if scheme.eq_ignore_ascii_case("https") => format!("wss://{rest}"),
        _ => base_url.to_string(),
    }
}

/// Resolves a glob against `base_url` as the driver's `resolveGlobBase`
/// does, so the client and the driver agree on what a pattern names.
///
/// A glob starting with `*` is left alone. Otherwise each path segment that
/// holds glob syntax is swapped for a placeholder, the rest is resolved as a
/// URL (which also normalizes it: a trailing slash after a bare host, a
/// default port dropped, percent-encoding, a lowercased scheme and host),
/// and the segments are put back, lowercased where they fall in the origin.
fn resolve_glob_base(base_url: Option<&str>, glob: &str) -> String {
    if glob.starts_with('*') {
        return glob.to_string();
    }
    // An escaped `\\?` reads the same as `?`.
    let glob = glob.replace("\\\\?", "?");
    if ["about:", "data:", "chrome:", "edge:", "file:"]
        .iter()
        .any(|scheme| glob.starts_with(scheme))
    {
        return glob;
    }

    let mut tokens: Vec<(String, String)> = Vec::new();
    let mut map_token = |original: &str, replacement: String| -> String {
        if original.is_empty() {
            return String::new();
        }
        tokens.push((replacement.clone(), original.to_string()));
        replacement
    };
    let relative_path = glob
        .split('/')
        .enumerate()
        .map(|(index, token)| {
            if index == 0 && token.ends_with(':') {
                if token.contains(['*', '{']) {
                    return map_token(token, "http:".to_string());
                }
                return token.to_string();
            }
            if !token.contains(['*', '?', '{', '}', '\\']) {
                return token.to_string();
            }
            match token.find('?') {
                None => map_token(token, format!("$_{index}_$")),
                Some(question) => {
                    let (prefix, suffix) = token.split_at(question);
                    let prefix = map_token(prefix, format!("$_{index}_$"));
                    let suffix = map_token(suffix, format!("?$_{index}_$"));
                    prefix + &suffix
                }
            }
        })
        .collect::<Vec<_>>()
        .join("/");

    let (mut resolved, origin) = resolve_base_url(base_url, &relative_path);
    for (token, original) in tokens {
        let in_origin = origin
            .as_deref()
            .is_some_and(|origin| origin.contains(&token));
        let replacement = if in_origin {
            original.to_lowercase()
        } else {
            original
        };
        resolved = resolved.replacen(&token, &replacement, 1);
    }
    resolved
}

/// Resolves an expected URL against `base_url` as the driver's
/// `constructURLBasedOnBaseURL` does for `toHaveURL`: the URL comes back
/// resolved and normalized, or unchanged when it does not resolve. Unlike
/// [`GlobMatcher::for_url`] it reads no glob syntax.
pub(crate) fn construct_url_based_on_base_url(base_url: Option<&str>, url: &str) -> String {
    resolve_base_url(base_url, url).0
}

/// Resolves `given` against `base_url` as `new URL(given, baseURL)` does,
/// returning the serialized URL and its origin, or `given` unchanged (and no
/// origin) when either does not parse.
fn resolve_base_url(base_url: Option<&str>, given: &str) -> (String, Option<String>) {
    let parsed = match base_url {
        Some(base) => url::Url::parse(base).and_then(|base| base.join(given)),
        None => url::Url::parse(given),
    };
    match parsed {
        Ok(url) => {
            let origin = url.origin().ascii_serialization();
            (url.into(), Some(origin))
        }
        Err(_) => (given.to_string(), None),
    }
}

/// Returns whether `text` matches the glob `pattern`.
///
/// Returns `false` for a malformed pattern, matching the driver's behavior
/// of treating it as matching nothing rather than panicking. Callers compile
/// with [`GlobMatcher::new`] instead, once per pattern; this one-shot form is
/// what the port's tests check against the driver's cases.
#[cfg(test)]
pub(crate) fn glob_match(pattern: &str, text: &str) -> bool {
    GlobMatcher::new(pattern).is_some_and(|m| m.matches(text))
}

/// A glob compiled once, for callers that match the same pattern repeatedly.
///
/// `Frame::wait_for_url` polls every 50ms, so compiling per call meant
/// hundreds of identical regex compilations for a single wait.
#[derive(Clone)]
pub(crate) struct GlobMatcher {
    regex: regex::Regex,
}

impl GlobMatcher {
    /// Compiles `pattern`, returning `None` if it is malformed.
    pub(crate) fn new(pattern: &str) -> Option<Self> {
        let regex_pattern = glob_to_regex_pattern(pattern)?;
        regex::Regex::new(&regex_pattern)
            .ok()
            .map(|regex| Self { regex })
    }

    /// Compiles `pattern` the way the driver does before matching a URL
    /// against it: resolved against `base_url` first. For a WebSocket
    /// pattern the caller passes the base through [`to_web_socket_base_url`]
    /// first. Returns `None` if the pattern is malformed.
    pub(crate) fn for_url(base_url: Option<&str>, pattern: &str) -> Option<Self> {
        Self::new(&resolve_glob_base(base_url, pattern))
    }

    pub(crate) fn matches(&self, text: &str) -> bool {
        self.regex.is_match(text)
    }
}

#[cfg(test)]
mod properties {
    use super::{GlobMatcher, glob_match, glob_to_regex_pattern};
    use proptest::prelude::*;

    /// Glob-shaped text over the full syntax: wildcards, group punctuation,
    /// escapes, separators and regex metacharacters, in any order.
    ///
    /// Unlike `url_atom` this is allowed to be malformed. The point is to
    /// reach the states hand-written cases do not: an unbalanced brace, a
    /// backslash at the very end, a run of stars against a separator.
    fn glob_soup() -> impl Strategy<Value = String> {
        proptest::collection::vec(
            proptest::sample::select(vec![
                "a", "/", "*", "**", "{", "}", ",", "\\", ".", "?", "[", "]", "(", ")", "|", "+",
                "$", "^",
            ]),
            0..10,
        )
        .prop_map(|parts| parts.concat())
    }

    /// URL-ish text containing no glob metacharacter, over a deliberately
    /// narrow alphabet that nonetheless includes the regex metacharacters a
    /// real URL carries (`?`, `+`, `(`, `[`, ...).
    ///
    /// The alphabet is the point. Identifier-shaped input is already covered
    /// by the example tests below, so letting the generator roam there would
    /// rediscover those instead of exercising the escaping.
    fn url_atom() -> impl Strategy<Value = String> {
        proptest::collection::vec(
            proptest::sample::select(vec![
                "a", "b", "1", "-", "_", "=", "&", "?", "+", "(", ")", "[", "]", "|", "$", "^", ".",
            ]),
            1..6,
        )
        .prop_map(|parts| parts.concat())
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        /// A pattern with no wildcard matches itself. Translating a glob to a
        /// regex is exactly where "everything else is literal" stops holding.
        #[test]
        fn a_pattern_without_a_wildcard_is_literal(text in url_atom()) {
            prop_assert!(
                glob_match(&text, &text),
                "pattern `{}` should match itself literally", text
            );
        }

        /// A trailing `*` matches any one segment, whatever it contains.
        #[test]
        fn a_trailing_wildcard_matches_any_single_segment(
            prefix in url_atom(),
            segment in url_atom(),
        ) {
            let pattern = format!("https://example.com/{prefix}/*");
            let text = format!("https://example.com/{prefix}/{segment}");
            prop_assert!(
                glob_match(&pattern, &text),
                "`{}` should match `{}`", pattern, text
            );
        }

        /// Escaping a character makes it literal, whatever it is. This is the
        /// escape hatch for matching a URL that really does contain a `*`.
        #[test]
        fn a_backslash_escape_is_always_literal(text in url_atom()) {
            let pattern: String = text.chars().flat_map(|c| ['\\', c]).collect();
            prop_assert!(
                glob_match(&pattern, &text),
                "escaped `{}` should match `{}`", pattern, text
            );
        }

        /// Anything this accepts, the regex engine must accept too.
        ///
        /// The two failure modes are indistinguishable to a caller: a glob we
        /// reject and a glob we translate into an uncompilable regex both come
        /// back as "matches nothing". This is the invariant that keeps that
        /// silence honest, and it is the shape of the bug this module was
        /// rewritten to fix.
        #[test]
        fn an_accepted_pattern_always_compiles(pattern in glob_soup()) {
            if let Some(translated) = glob_to_regex_pattern(&pattern) {
                prop_assert!(
                    regex::Regex::new(&translated).is_ok(),
                    "accepted `{}` but produced uncompilable regex `{}`",
                    pattern, translated
                );
                prop_assert!(
                    GlobMatcher::new(&pattern).is_some(),
                    "`{}` translated but did not compile", pattern
                );
            }
        }

        /// Matching is total: no pattern, however malformed, panics.
        #[test]
        fn matching_never_panics(pattern in glob_soup(), text in glob_soup()) {
            let _ = glob_match(&pattern, &text);
        }
    }
}

#[cfg(test)]
mod url_matches {
    use super::{GlobMatcher, to_web_socket_base_url};

    fn url_matches(base_url: Option<&str>, url: &str, glob: &str) -> bool {
        GlobMatcher::for_url(base_url, glob).is_some_and(|m| m.matches(url))
    }

    // The `urlMatches` cases of the driver's `interception.spec.ts` "should
    // work with glob", in order, as (base URL, URL, glob, matches).
    const DRIVER_CASES: &[(Option<&str>, &str, &str, bool)] = &[
        (
            None,
            "http://playwright.dev/",
            "http://playwright.dev",
            true,
        ),
        (
            None,
            "http://playwright.dev/?a=b",
            "http://playwright.dev?a=b",
            true,
        ),
        (None, "http://playwright.dev/", "h*://playwright.dev", true),
        (
            None,
            "http://api.playwright.dev/?x=y",
            "http://*.playwright.dev?x=y",
            true,
        ),
        (None, "http://playwright.dev/foo/bar", "**/foo/**", true),
        (
            Some("http://playwright.dev"),
            "http://playwright.dev/?x=y",
            "?x=y",
            true,
        ),
        (
            Some("http://playwright.dev/foo/"),
            "http://playwright.dev/foo/bar?x=y",
            "./bar?x=y",
            true,
        ),
        (
            None,
            "http://playwright.dev/foo$$bar",
            "http://playwright.dev/foo$$bar",
            true,
        ),
        (
            None,
            "http://playwright.dev/a$&b",
            "http://playwright.dev/a$&b",
            true,
        ),
        (
            Some("http://playwright.dev"),
            "http://playwright.dev/p$$q",
            "./p$$q",
            true,
        ),
        (
            None,
            "https://playwright.dev/fooBAR",
            "HtTpS://pLaYwRiGhT.dEv/fooBAR",
            true,
        ),
        (
            Some("http://ignored"),
            "https://playwright.dev/fooBAR",
            "HtTpS://pLaYwRiGhT.dEv/fooBAR",
            true,
        ),
        (
            None,
            "https://playwright.dev/foobar",
            "https://playwright.dev/fooBAR",
            false,
        ),
        (
            None,
            "https://playwright.dev/foobar?a=b",
            "https://playwright.dev/foobar?A=B",
            false,
        ),
        (
            None,
            "http://example.com/path",
            "http://example.com:80/path",
            true,
        ),
        (
            None,
            "https://example.com/path",
            "https://example.com:443/path",
            true,
        ),
        (
            None,
            "http://example.com:8080/path",
            "http://example.com:8080/path",
            true,
        ),
        (None, "http://localhost/", "http://localhost:80/**", true),
        (
            None,
            "http://example.com/foo%20bar",
            "http://example.com/foo bar",
            true,
        ),
        (
            None,
            "http://xn--mnchen-3ya.de/",
            "http://münchen.de/",
            true,
        ),
        (None, "https://localhost:3000/?a=b", "**/?a=b", true),
        (None, "https://localhost:3000/?a=b", "**?a=b", true),
        (None, "https://localhost:3000/?a=b", "**=b", true),
        (
            None,
            "my.custom.protocol://foo",
            "my.custom.protocol://**",
            true,
        ),
        (None, "my.p://foo", "my.{p,y}://**", false),
        (None, "my.p://foo/", "my.{p,y}://**", true),
        (None, "file:///foo/", "f*e://**", true),
        (
            None,
            "http://playwright.dev/",
            "http://playwright.?ev",
            false,
        ),
        (
            None,
            "http://playwright./?ev",
            "http://playwright.?ev",
            true,
        ),
        (
            None,
            "http://playwright.dev/foo",
            "http://playwright.dev/f??",
            false,
        ),
        (
            None,
            "http://playwright.dev/f??",
            "http://playwright.dev/f??",
            true,
        ),
        (
            None,
            "http://playwright.dev/?x=y",
            "http://playwright.dev\\?x=y",
            true,
        ),
        (
            None,
            "http://playwright.dev/?x=y",
            "http://playwright.dev/\\?x=y",
            true,
        ),
        (
            Some("http://playwright.dev/foo"),
            "http://playwright.dev/foo?bar",
            "?bar",
            true,
        ),
        (
            Some("http://playwright.dev/foo"),
            "http://playwright.dev/foo?bar",
            "\\\\?bar",
            true,
        ),
        (
            Some("http://first.host/"),
            "http://second.host/foo",
            "**/foo",
            true,
        ),
        (
            Some("http://playwright.dev/"),
            "http://localhost/",
            "*//localhost/",
            true,
        ),
        (None, "https://foo/bar.js", "https://foo/**/bar.js", true),
        (None, "https://foo/bar.js", "https://foo/**/**/bar.js", true),
    ];

    #[test]
    fn matches_urls_as_the_driver_does() {
        for &(base_url, url, glob, expected) in DRIVER_CASES {
            assert_eq!(
                url_matches(base_url, url, glob),
                expected,
                "base {base_url:?}, url {url:?}, glob {glob:?}"
            );
        }
    }

    #[test]
    fn custom_scheme_prefixes_are_not_resolved() {
        for prefix in ["about", "data", "chrome", "edge", "file"] {
            let base = Some("http://playwright.dev/");
            let blank = format!("{prefix}:blank");
            assert!(url_matches(base, &blank, &blank), "{prefix}");
            assert!(
                !url_matches(base, &blank, "http://playwright.dev/"),
                "{prefix}"
            );
            assert!(url_matches(None, &blank, &blank), "{prefix}");
            assert!(
                url_matches(None, &blank, &format!("{prefix}:*")),
                "{prefix}"
            );
            assert!(
                !url_matches(None, &format!("not{prefix}:blank"), &format!("{prefix}:*")),
                "{prefix}"
            );
        }
    }

    fn web_socket_matches(base_url: Option<&str>, url: &str, glob: &str) -> bool {
        let base_url = base_url.map(to_web_socket_base_url);
        GlobMatcher::for_url(base_url.as_deref(), glob).is_some_and(|m| m.matches(url))
    }

    #[test]
    fn a_web_socket_pattern_resolves_against_the_ws_form_of_the_base() {
        assert!(web_socket_matches(
            Some("http://localhost:3000"),
            "ws://localhost:3000/ws",
            "/ws"
        ));
        assert!(web_socket_matches(
            Some("https://localhost:3000"),
            "wss://localhost:3000/ws",
            "/ws"
        ));
        assert!(!web_socket_matches(
            Some("http://localhost:3000"),
            "http://localhost:3000/ws",
            "/ws"
        ));
    }

    #[test]
    fn the_base_scheme_is_read_without_regard_to_case() {
        assert!(web_socket_matches(
            Some("HTTP://localhost:3000"),
            "ws://localhost:3000/ws",
            "/ws"
        ));
        assert!(web_socket_matches(
            Some("HtTpS://localhost:3000"),
            "wss://localhost:3000/ws",
            "/ws"
        ));
    }

    #[test]
    fn a_base_that_is_not_http_is_left_as_it_is() {
        for base in ["ws://localhost:3000/", "file:///tmp/", "localhost:3000"] {
            assert_eq!(to_web_socket_base_url(base), base);
        }
    }

    #[test]
    fn a_web_socket_url_without_a_path_matches_with_its_trailing_slash() {
        assert!(web_socket_matches(
            None,
            "ws://localhost:3000/",
            "ws://localhost:3000"
        ));
    }

    #[test]
    fn only_a_first_segment_ending_in_a_colon_is_read_as_a_scheme() {
        let base = Some("http://playwright.dev/dir/");
        assert!(url_matches(
            base,
            "http://playwright.dev/dir/foobar/baz",
            "foo*/baz"
        ));
        assert!(url_matches(base, "http://playwright.dev/dir/a/b:", "a/b*:"));
    }

    #[test]
    fn a_relative_pattern_without_a_base_stays_as_written() {
        assert!(url_matches(None, "/ws", "/ws"));
        assert!(!url_matches(None, "http://localhost/ws", "/ws"));
    }

    #[test]
    fn an_unusable_base_leaves_the_pattern_unnormalized() {
        assert!(url_matches(
            None,
            "http://playwright.dev/",
            "http://playwright.dev"
        ));
        for base in ["", "not a url"] {
            assert!(
                !url_matches(
                    Some(base),
                    "http://playwright.dev/",
                    "http://playwright.dev"
                ),
                "base {base:?}"
            );
        }
    }
}

#[cfg(test)]
mod construct_url {
    use super::construct_url_based_on_base_url;

    // (base URL, URL, resolved), each checked against the pinned driver's
    // own constructURLBasedOnBaseURL.
    const DRIVER_CASES: &[(Option<&str>, &str, &str)] = &[
        (
            Some("http://localhost:3000/foobar/"),
            "/hello",
            "http://localhost:3000/hello",
        ),
        (
            Some("http://localhost:3000/foobar/"),
            "./kek",
            "http://localhost:3000/foobar/kek",
        ),
        (None, "http://localhost:3000", "http://localhost:3000/"),
        (None, "/hello", "/hello"),
        (Some("not a url"), "/hello", "/hello"),
        (
            Some("http://ignored/"),
            "https://Example.COM:443/a",
            "https://example.com/a",
        ),
    ];

    #[test]
    fn resolves_an_expected_url_as_the_driver_does() {
        for &(base_url, url, resolved) in DRIVER_CASES {
            assert_eq!(
                construct_url_based_on_base_url(base_url, url),
                resolved,
                "base {base_url:?}, url {url:?}"
            );
        }
    }

    #[test]
    fn reads_no_glob_syntax() {
        assert_eq!(
            construct_url_based_on_base_url(Some("http://localhost:3000/"), "**/a"),
            "http://localhost:3000/**/a"
        );
    }
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

        // A single star immediately before a `/` is still a single star: only
        // a run of two or more takes the cross-segment branch.
        assert!(glob_match(
            "https://example.com/*/b",
            "https://example.com/a/b"
        ));
        assert!(!glob_match(
            "https://example.com/*/b",
            "https://example.com/a/x/b"
        ));
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
    }

    #[test]
    fn double_star_between_slashes_matches_zero_directories() {
        // The driver emits `((.+/)|)` here, so the directories are optional.
        assert!(glob_match(
            "https://example.com/**/edit",
            "https://example.com/edit"
        ));
        assert!(glob_match(
            "https://example.com/**/edit",
            "https://example.com/a/b/edit"
        ));
    }

    #[test]
    fn braces_are_alternation() {
        assert!(glob_match("**/*.{png,jpg}", "https://example.com/a.png"));
        assert!(glob_match("**/*.{png,jpg}", "https://example.com/a.jpg"));
        assert!(!glob_match("**/*.{png,jpg}", "https://example.com/a.gif"));
        // A comma outside a group is literal. The negative is the load-bearing
        // half: were the comma alternation, `.../a,b` would still match itself
        // through the `b$` branch while also matching `.../a`.
        assert!(glob_match(
            "https://example.com/a,b",
            "https://example.com/a,b"
        ));
        assert!(!glob_match(
            "https://example.com/a,b",
            "https://example.com/a"
        ));
    }

    #[test]
    fn malformed_braces_match_nothing() {
        assert!(!glob_match("{a{b}}", "ab"));
        assert!(!glob_match("a}", "a}"));
        assert!(!glob_match("{a", "a"));
    }

    #[test]
    fn regex_metacharacters_are_literal() {
        // These used to reach the regex engine: `[unterminated` failed to
        // compile (matching nothing), and `?`/`+` silently changed what the
        // pattern meant.
        assert!(!glob_match("[unterminated", "anything"));
        assert!(glob_match("[unterminated", "[unterminated"));

        // A query string is the common case, and the one that motivated this.
        assert!(glob_match(
            "https://example.com/search?q=*",
            "https://example.com/search?q=widget"
        ));
        // ...and it must not match the URL the unescaped `?` used to allow.
        assert!(!glob_match(
            "https://example.com/search?q=*",
            "https://example.com/searcq=widget"
        ));
    }

    #[test]
    fn backslash_escapes_the_next_character() {
        // the escape hatch for a URL that really contains a `*`
        assert!(glob_match(
            r"https://example.com/\*",
            "https://example.com/*"
        ));
        assert!(!glob_match(
            r"https://example.com/\*",
            "https://example.com/x"
        ));
    }

    #[test]
    fn dot_is_literal_not_wildcard() {
        assert!(glob_match(
            "https://example.com/a.png",
            "https://example.com/a.png"
        ));
        assert!(!glob_match(
            "https://example.com/a.png",
            "https://example.com/axpng"
        ));
    }

    #[test]
    fn extension_globs() {
        assert!(glob_match("**/*.png", "https://example.com/a/b/c.png"));
        assert!(!glob_match("**/*.png", "https://example.com/a/b/c.jpg"));

        // `**/` before a segment emits `(.*/)`, which requires the slash, so
        // a slashless subject does not match. Pins the star-run boundary:
        // miscounting the stars here turns this into `(.*)`, which does.
        assert!(!glob_match("**/*.png", "a.png"));
    }

    #[test]
    fn a_trailing_backslash_is_literal() {
        // There is nothing to escape, so the backslash is its own character.
        // Pins the escape lookahead bound: reading past it panics.
        assert!(glob_match(r"a\", r"a\"));
        assert!(!glob_match(r"a\", "a"));
    }

    // Expected strings are the driver's own output for each glob (its
    // `globToRegexPattern` at v1.65.1, run in Node). Matching tests cannot
    // tell `\,` from `,`; this can.
    #[test]
    fn translates_to_the_drivers_exact_pattern() {
        for (glob, driver) in [
            ("a,b", r"^a\,b$"),
            ("{a,b}", "^(a|b)$"),
            (r"a\,b", "^a,b$"),
            ("**/*.{png,jpg}", r"^(.*/)([^/]*)\.(png|jpg)$"),
            ("/**/x", "^/((.+/)|)x$"),
            ("a**/b", "^a(.*/)b$"),
            ("a**", "^a(.*)$"),
            ("*", "^([^/]*)$"),
            (r"a\", r"^a\\$"),
            (r"\**", r"^\*([^/]*)$"),
            ("$.(|)?[]^+", r"^\$\.\(\|\)\?\[\]\^\+$"),
            ("a{b}c,d", r"^a(b)c\,d$"),
            (r"\{a\}", r"^\{a\}$"),
            (r"{a\}b}", r"^(a\}b)$"),
        ] {
            assert_eq!(
                super::glob_to_regex_pattern(glob).as_deref(),
                Some(driver),
                "{glob}"
            );
        }
    }
}
