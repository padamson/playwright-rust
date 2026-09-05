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
use std::collections::HashMap;

/// Headers in the driver's `NameValue[]` shape.
pub(crate) fn header_array(headers: impl IntoIterator<Item = (String, String)>) -> Vec<Value> {
    headers
        .into_iter()
        .map(|(name, value)| json!({ "name": name, "value": value }))
        .collect()
}

fn base64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// Parameters for the driver's `Route.fulfill` command.
///
/// A UTF-8 body is sent as text; anything else is base64 with `isBase64` set.
/// `content-type` and `content-length` are folded into the headers, whose
/// names are lowercased, the same normalization playwright-python applies.
/// A caller's own `content-length` stands, and none is added for an empty
/// body. Names that differ only by case collapse to one header; the winner is
/// the spelling that sorts last, so the result does not depend on `HashMap`
/// iteration order.
pub(crate) fn fulfill_params(opts: FulfillOptions) -> Value {
    let mut supplied: Vec<(String, String)> = opts.headers.into_iter().flatten().collect();
    supplied.sort();

    let mut headers: Vec<(String, String)> = Vec::with_capacity(supplied.len() + 2);
    let mut set = |name: String, value: String| match headers
        .iter_mut()
        .find(|(existing, _)| *existing == name)
    {
        Some(slot) => slot.1 = value,
        None => headers.push((name, value)),
    };
    for (name, value) in supplied {
        set(name.to_ascii_lowercase(), value);
    }
    if let Some(content_type) = opts.content_type {
        set("content-type".to_string(), content_type);
    }

    let body = opts.body;
    if let Some(body) = &body
        && !body.is_empty()
        && !headers.iter().any(|(name, _)| name == "content-length")
    {
        headers.push(("content-length".to_string(), body.len().to_string()));
    }

    let mut params = json!({
        "status": opts.status.unwrap_or(200),
        "headers": header_array(headers),
    });

    if let Some(body) = body {
        let (encoded, is_base64) = match String::from_utf8(body) {
            Ok(text) => (text, false),
            Err(not_utf8) => (base64(not_utf8.as_bytes()), true),
        };
        params["body"] = json!(encoded);
        params["isBase64"] = json!(is_base64);
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
        params["headers"] = json!(header_array(headers));
    }
    if let Some(method) = opts.method {
        params["method"] = json!(method);
    }
    let post_data = opts
        .post_data
        .map(String::into_bytes)
        .or(opts.post_data_bytes);
    if let Some(bytes) = post_data {
        params["postData"] = json!(base64(&bytes));
    }
    if let Some(url) = opts.url {
        params["url"] = json!(url);
    }

    params
}

/// Response headers from a HAR lookup as the map `FulfillOptions` takes.
///
/// Repeated `set-cookie` headers are joined with a newline, which is how the
/// upstream HAR router carries several cookies through the single-valued
/// header map; the browser side splits them again. Any other repeated name
/// keeps its last value.
pub(crate) fn har_response_headers(raw: &[Value]) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    for entry in raw {
        let (Some(name), Some(value)) = (
            entry.get("name").and_then(Value::as_str),
            entry.get("value").and_then(Value::as_str),
        ) else {
            continue;
        };
        if name.eq_ignore_ascii_case("set-cookie")
            && let Some(existing) = headers.get_mut(name)
        {
            let existing: &mut String = existing;
            existing.push('\n');
            existing.push_str(value);
            continue;
        }
        headers.insert(name.to_string(), value.to_string());
    }
    headers
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

    fn headers(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
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
            .headers(headers(&[("X-Custom", "yes")]))
            .content_type("text/html")
            .build();

        assert_eq!(
            header_values(&fulfill_params(opts)),
            vec![
                ("x-custom".to_string(), "yes".to_string()),
                ("content-type".to_string(), "text/html".to_string()),
            ]
        );
    }

    #[test]
    fn fulfill_explicit_content_length_is_not_overwritten() {
        let opts = FulfillOptions::builder()
            .headers(headers(&[("Content-Length", "99")]))
            .body_string("abc")
            .build();

        assert_eq!(
            header_values(&fulfill_params(opts)),
            vec![("content-length".to_string(), "99".to_string())]
        );
    }

    #[test]
    fn fulfill_case_variant_duplicates_resolve_deterministically() {
        let opts = FulfillOptions::builder()
            .headers(headers(&[("x-token", "lower"), ("X-Token", "upper")]))
            .build();

        // "x-token" sorts after "X-Token", so the lowercase spelling wins,
        // whatever order the HashMap yields them in.
        assert_eq!(
            header_values(&fulfill_params(opts)),
            vec![("x-token".to_string(), "lower".to_string())]
        );
    }

    #[test]
    fn continue_without_overrides_only_carries_the_fallback_flag() {
        assert_eq!(continue_params(None, false), json!({ "isFallback": false }));
        assert_eq!(continue_params(None, true), json!({ "isFallback": true }));
    }

    #[test]
    fn continue_string_post_data_is_base64_on_the_wire() {
        let opts = ContinueOptions::builder()
            .post_data("key=value".to_string())
            .build();

        assert_eq!(
            continue_params(Some(opts), false)["postData"],
            "a2V5PXZhbHVl"
        );
    }

    #[test]
    fn continue_bytes_post_data_is_base64_on_the_wire() {
        let opts = ContinueOptions::builder()
            .post_data_bytes(vec![0x01, 0x02, 0xff])
            .build();

        assert_eq!(continue_params(Some(opts), false)["postData"], "AQL/");
    }

    #[test]
    fn continue_string_post_data_takes_precedence_over_bytes() {
        // The builder keeps the two forms exclusive; the struct itself does not.
        let opts = ContinueOptions {
            post_data: Some("text".to_string()),
            post_data_bytes: Some(vec![0x01]),
            ..ContinueOptions::default()
        };

        assert_eq!(continue_params(Some(opts), false)["postData"], "dGV4dA==");
    }

    #[test]
    fn continue_headers_method_and_url_pass_through_unchanged() {
        let opts = ContinueOptions::builder()
            .headers(headers(&[("X-Custom", "v")]))
            .method("POST".to_string())
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
        assert!(params.get("postData").is_none());
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

        let map = har_response_headers(&raw);

        assert_eq!(map["set-cookie"], "a=1\nb=2");
        assert_eq!(map["x-dup"], "second");
        assert_eq!(map["content-type"], "text/html");
        assert_eq!(map.len(), 3);
    }
}
