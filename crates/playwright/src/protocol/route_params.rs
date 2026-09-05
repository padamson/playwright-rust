//! Wire-format builders for `Route` commands, kept pure so they can be unit
//! tested and mutation tested without a driver.
//!
//! The parameter shapes come from `packages/protocol/spec/network.yml` at the
//! bundled driver's tag. Both commands take their fields as top-level command
//! parameters, and `postData` is declared `binary`, which the driver reads as
//! base64.

use super::route::{ContinueOptions, FulfillOptions};
use base64::Engine as _;
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap};

/// Headers in the driver's `NameValue[]` shape. Assign it into a params
/// object as `Value::Array(..)`, which moves it, rather than through `json!`,
/// which serializes a copy.
pub(crate) fn header_array(headers: impl IntoIterator<Item = (String, String)>) -> Vec<Value> {
    headers
        .into_iter()
        .map(|(name, value)| {
            Value::Object(serde_json::Map::from_iter([
                ("name".to_string(), Value::String(name)),
                ("value".to_string(), Value::String(value)),
            ]))
        })
        .collect()
}

/// The `(name, value)` pairs of a driver `NameValue[]`, absent or present,
/// skipping malformed entries.
pub(crate) fn name_value_pairs(raw: Option<&[Value]>) -> impl Iterator<Item = (&str, &str)> {
    raw.into_iter()
        .flatten()
        .filter_map(|entry| Some((entry.get("name")?.as_str()?, entry.get("value")?.as_str()?)))
}

fn base64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// A base64 field from the driver as bytes, or `None` if it does not decode.
pub(crate) fn base64_decode(encoded: &str) -> Option<Vec<u8>> {
    base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .ok()
}

/// The `postData` parameter: binary on the wire, so both forms are base64.
/// The string form wins when both are given.
pub(crate) fn post_data_param(post_data: Option<String>, bytes: Option<Vec<u8>>) -> Option<String> {
    post_data
        .map(String::into_bytes)
        .or(bytes)
        .map(|bytes| base64(&bytes))
}

/// Header `name`/`value` pairs merged into a single-valued map with
/// lowercase keys.
///
/// Repeated `set-cookie` headers are joined with a newline, which is how
/// upstream carries several cookies through a single-valued map; the browser
/// splits them again. Other repeated names are joined with `join_others`, the
/// `, ` that RFC 9110 gives such repeats, or the last value wins when it is
/// `None`, which is what the upstream HAR router does with a recorded
/// response.
pub(crate) fn merge_headers<K: AsRef<str>, V: AsRef<str>>(
    pairs: impl IntoIterator<Item = (K, V)>,
    join_others: Option<&str>,
) -> HashMap<String, String> {
    let mut headers: HashMap<String, String> = HashMap::new();
    for (name, value) in pairs {
        let name = name.as_ref().to_ascii_lowercase();
        let value = value.as_ref();
        let separator = if name == "set-cookie" {
            Some("\n")
        } else {
            join_others
        };
        match (headers.get_mut(&name), separator) {
            (Some(existing), Some(separator)) => {
                existing.push_str(separator);
                existing.push_str(value);
            }
            _ => {
                headers.insert(name, value.to_string());
            }
        }
    }
    headers
}

/// Parameters for the driver's `Route.fulfill` command.
///
/// A UTF-8 body is sent as text; anything else is base64 with `isBase64` set.
/// `content-type` and `content-length` are folded into the headers, whose
/// names are lowercased, the same normalization playwright-python applies.
/// A caller's own `content-length` stands, and none is added for an empty
/// body.
pub(crate) fn fulfill_params(opts: FulfillOptions) -> Value {
    let mut headers: BTreeMap<String, String> = opts
        .headers
        .into_iter()
        .flatten()
        .map(|(name, value)| (name.to_ascii_lowercase(), value))
        .collect();
    if let Some(content_type) = opts.content_type {
        headers.insert("content-type".to_string(), content_type);
    }
    if let Some(body) = &opts.body
        && !body.is_empty()
    {
        headers
            .entry("content-length".to_string())
            .or_insert_with(|| body.len().to_string());
    }

    let mut params = json!({ "status": opts.status.unwrap_or(200) });
    params["headers"] = Value::Array(header_array(headers));

    if let Some(body) = opts.body {
        let (encoded, is_base64) = match String::from_utf8(body) {
            Ok(text) => (text, false),
            Err(not_utf8) => (base64(not_utf8.as_bytes()), true),
        };
        params["body"] = Value::String(encoded);
        params["isBase64"] = Value::Bool(is_base64);
    }

    params
}

/// Parameters for the driver's `Route.continue` command, shared by
/// `continue_` and `fallback`.
///
/// `postData` is binary on the wire, so the string form is base64-encoded
/// like the bytes form. Header names are sent as given; upstream does not
/// lowercase them here.
pub(crate) fn continue_params(overrides: Option<ContinueOptions>, is_fallback: bool) -> Value {
    let mut params = json!({ "isFallback": is_fallback });
    let Some(opts) = overrides else {
        return params;
    };

    if let Some(headers) = opts.headers {
        params["headers"] = Value::Array(header_array(headers));
    }
    if let Some(method) = opts.method {
        params["method"] = json!(method);
    }
    if let Some(encoded) = post_data_param(opts.post_data, opts.post_data_bytes) {
        params["postData"] = Value::String(encoded);
    }
    if let Some(url) = opts.url {
        params["url"] = json!(url);
    }

    params
}

/// Response headers from a HAR lookup as the map `FulfillOptions` takes:
/// repeated `set-cookie` joined with a newline, other repeats last-wins.
pub(crate) fn har_response_headers(raw: Option<&[Value]>) -> HashMap<String, String> {
    merge_headers(name_value_pairs(raw), None)
}

/// The `FulfillOptions` that replay a HAR lookup result: the recorded status,
/// its headers, and the base64 body the driver hands back.
pub(crate) fn har_fulfill_options(
    status: Option<u16>,
    body_base64: Option<&str>,
    headers: Option<&[Value]>,
) -> FulfillOptions {
    let mut builder = FulfillOptions::builder()
        .status(status.unwrap_or(200))
        .headers(har_response_headers(headers));
    if let Some(body) = body_base64 {
        builder = builder.body(base64_decode(body).unwrap_or_default());
    }
    builder.build()
}

/// A header map from literals, for this module's tests and its siblings'.
#[cfg(test)]
pub(crate) fn headers_of(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header_values(params: &Value) -> Vec<(String, String)> {
        params["headers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|h| {
                (
                    h["name"].as_str().unwrap().to_string(),
                    h["value"].as_str().unwrap().to_string(),
                )
            })
            .collect()
    }

    #[test]
    fn fulfill_params_are_top_level_command_parameters_not_nested() {
        let params = fulfill_params(FulfillOptions::default());

        assert!(params.get("response").is_none());
        assert_eq!(params["status"], 200);
        assert_eq!(params["headers"], json!([]));
        assert!(params.get("body").is_none());
        assert!(params.get("isBase64").is_none());
    }

    #[test]
    fn fulfill_status_is_passed_through() {
        let opts = FulfillOptions::builder().status(404).build();

        assert_eq!(fulfill_params(opts)["status"], 404);
    }

    #[test]
    fn fulfill_utf8_body_is_sent_as_text_with_its_length() {
        let opts = FulfillOptions::builder().body_string("héllo").build();

        let params = fulfill_params(opts);

        assert_eq!(params["body"], "héllo");
        assert_eq!(params["isBase64"], false);
        assert_eq!(
            header_values(&params),
            vec![("content-length".to_string(), "6".to_string())]
        );
    }

    #[test]
    fn fulfill_binary_body_is_sent_as_base64() {
        let opts = FulfillOptions::builder()
            .body(vec![0xff, 0x00, 0xfe])
            .build();

        let params = fulfill_params(opts);

        assert_eq!(params["body"], "/wD+");
        assert_eq!(params["isBase64"], true);
        assert_eq!(
            header_values(&params),
            vec![("content-length".to_string(), "3".to_string())]
        );
    }

    #[test]
    fn fulfill_empty_body_gets_no_content_length() {
        let opts = FulfillOptions::builder().body(vec![]).build();

        let params = fulfill_params(opts);

        assert_eq!(params["body"], "");
        assert_eq!(params["headers"], json!([]));
    }

    #[test]
    fn fulfill_content_type_and_explicit_headers_are_lowercased_and_merged() {
        let opts = FulfillOptions::builder()
            .headers(headers_of(&[("X-Custom", "yes")]))
            .content_type("text/html")
            .build();

        assert_eq!(
            header_values(&fulfill_params(opts)),
            vec![
                ("content-type".to_string(), "text/html".to_string()),
                ("x-custom".to_string(), "yes".to_string()),
            ]
        );
    }

    #[test]
    fn fulfill_explicit_content_length_is_not_overwritten() {
        let opts = FulfillOptions::builder()
            .headers(headers_of(&[("Content-Length", "99")]))
            .body_string("abc")
            .build();

        assert_eq!(
            header_values(&fulfill_params(opts)),
            vec![("content-length".to_string(), "99".to_string())]
        );
    }

    #[test]
    fn continue_without_overrides_only_carries_the_fallback_flag() {
        assert_eq!(continue_params(None, false), json!({ "isFallback": false }));
        assert_eq!(continue_params(None, true), json!({ "isFallback": true }));
    }

    #[test]
    fn continue_overrides_pass_through_with_post_data_encoded() {
        let opts = ContinueOptions::builder()
            .headers(headers_of(&[("X-Custom", "v")]))
            .method("POST".to_string())
            .post_data("x".to_string())
            .url("https://example.test/x".to_string())
            .build();

        let params = continue_params(Some(opts), true);

        assert_eq!(params["isFallback"], true);
        assert_eq!(params["method"], "POST");
        assert_eq!(params["url"], "https://example.test/x");
        assert_eq!(
            header_values(&params),
            vec![("X-Custom".to_string(), "v".to_string())]
        );
        assert_eq!(params["postData"], "eA==");
    }

    #[test]
    fn post_data_prefers_the_string_form_and_encodes_both() {
        assert_eq!(post_data_param(None, None), None);
        assert_eq!(
            post_data_param(Some("key=value".to_string()), None).as_deref(),
            Some("a2V5PXZhbHVl")
        );
        assert_eq!(
            post_data_param(None, Some(vec![0x01, 0x02, 0xff])).as_deref(),
            Some("AQL/")
        );
        assert_eq!(
            post_data_param(Some("text".to_string()), Some(vec![0x01])).as_deref(),
            Some("dGV4dA==")
        );
    }

    #[test]
    fn merge_headers_joins_others_with_the_given_separator_or_keeps_the_last() {
        let pairs = [
            ("vary", "accept"),
            ("Vary", "origin"),
            ("Set-Cookie", "a=1"),
            ("set-cookie", "b=2"),
        ];

        let joined = merge_headers(pairs, Some(", "));
        assert_eq!(joined["vary"], "accept, origin");
        assert_eq!(joined["set-cookie"], "a=1\nb=2");
        assert_eq!(joined.len(), 2);

        let last_wins = merge_headers(pairs, None);
        assert_eq!(last_wins["vary"], "origin");
        assert_eq!(last_wins["set-cookie"], "a=1\nb=2");
    }

    #[test]
    fn har_fulfill_decodes_the_body_and_defaults_the_status() {
        let raw = vec![json!({ "name": "content-type", "value": "text/plain" })];

        let opts = har_fulfill_options(None, Some("aGVsbG8="), Some(&raw));

        assert_eq!(opts.status, Some(200));
        assert_eq!(opts.body.as_deref(), Some(&b"hello"[..]));
        assert_eq!(opts.headers.unwrap()["content-type"], "text/plain");

        let bare = har_fulfill_options(Some(304), None, None);
        assert_eq!(bare.status, Some(304));
        assert_eq!(bare.body, None);
        assert!(bare.headers.unwrap().is_empty());
    }

    #[test]
    fn har_headers_join_repeated_set_cookie_and_keep_the_last_of_others() {
        let raw = vec![
            json!({ "name": "set-cookie", "value": "a=1" }),
            json!({ "name": "set-cookie", "value": "b=2" }),
            json!({ "name": "x-dup", "value": "first" }),
            json!({ "name": "x-dup", "value": "second" }),
            json!({ "name": "content-type", "value": "text/html" }),
            json!({ "name": "broken" }),
        ];

        let map = har_response_headers(Some(&raw));

        assert_eq!(map["set-cookie"], "a=1\nb=2");
        assert_eq!(map["x-dup"], "second");
        assert_eq!(map["content-type"], "text/html");
        assert_eq!(map.len(), 3);
    }
}
