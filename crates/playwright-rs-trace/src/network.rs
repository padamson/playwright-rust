//! `trace.network` — HAR-like resource snapshots.

use serde::Deserialize;
use serde_json::Value;

/// One entry from `trace.network` — a HAR-like resource snapshot
/// recording a single HTTP request/response pair (or a single redirect
/// step in a chain).
#[derive(Debug, Clone)]
pub struct NetworkEntry {
    /// `_frameref` — frame GUID. `None` when the trace was recorded
    /// with `includeTraceInfo: false`.
    pub frame_ref: Option<String>,
    /// `pageref` — page GUID.
    pub page_ref: Option<String>,
    /// `_monotonicTime` (ms). `None` when `includeTraceInfo` was
    /// disabled at record time.
    pub monotonic_time: Option<f64>,
    /// `startedDateTime` — ISO-8601 wall-clock timestamp of the
    /// request start.
    pub started_date_time: String,
    /// Total request+response time in ms. `None` when timings weren't
    /// captured (HAR-spec `-1` sentinel mapped to `None` at parse time).
    pub time: Option<f64>,
    pub request: RequestSnapshot,
    pub response: ResponseSnapshot,
    /// HAR fields we don't model individually (`cookies`, `timings`,
    /// `cache`, `queryString`, `_transferSize`, …). Preserved verbatim
    /// for forward-compat and for callers that need them.
    pub raw_snapshot: Value,
}

#[derive(Debug, Clone)]
pub struct RequestSnapshot {
    pub method: String,
    pub url: String,
    pub http_version: String,
    pub headers: Vec<HeaderEntry>,
    pub headers_size: Option<u64>,
    pub body_size: Option<u64>,
    pub post_data: Option<RequestPostData>,
}

#[derive(Debug, Clone)]
pub struct ResponseSnapshot {
    pub status: Option<u16>,
    pub status_text: String,
    pub http_version: String,
    pub headers: Vec<HeaderEntry>,
    pub headers_size: Option<u64>,
    pub body_size: Option<u64>,
    /// `None` when not a redirect (empty string in the HAR wire).
    pub redirect_url: Option<String>,
    pub content: ResponseContent,
}

#[derive(Debug, Clone)]
pub struct HeaderEntry {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct RequestPostData {
    /// Points to `resources/<sha1>` in the zip.
    pub sha1: String,
}

#[derive(Debug, Clone)]
pub struct ResponseContent {
    pub size: Option<u64>,
    pub mime_type: String,
    /// Points to `resources/<sha1>`. `None` when the response has no
    /// body (`204`, `304`, …).
    pub sha1: Option<String>,
}

// ---------------------------------------------------------------------------
// Wire-format helpers (crate-private)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotWire {
    #[serde(default, rename = "_frameref")]
    frame_ref: Option<String>,
    #[serde(default)]
    pageref: Option<String>,
    #[serde(default, rename = "_monotonicTime")]
    monotonic_time: Option<f64>,
    #[serde(default)]
    started_date_time: String,
    #[serde(default = "default_time")]
    time: f64,
    request: RequestWire,
    response: ResponseWire,
}

fn default_time() -> f64 {
    -1.0
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequestWire {
    method: String,
    url: String,
    #[serde(default)]
    http_version: String,
    #[serde(default)]
    headers: Vec<HeaderEntryWire>,
    #[serde(default = "default_neg_one")]
    headers_size: i64,
    #[serde(default = "default_neg_one")]
    body_size: i64,
    #[serde(default)]
    post_data: Option<PostDataWire>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResponseWire {
    #[serde(default = "default_neg_one_i32")]
    status: i32,
    #[serde(default)]
    status_text: String,
    #[serde(default)]
    http_version: String,
    #[serde(default)]
    headers: Vec<HeaderEntryWire>,
    #[serde(default = "default_neg_one")]
    headers_size: i64,
    #[serde(default = "default_neg_one")]
    body_size: i64,
    #[serde(default)]
    redirect_url: String,
    content: ContentWire,
}

#[derive(Deserialize)]
struct HeaderEntryWire {
    name: String,
    value: String,
}

#[derive(Deserialize)]
struct PostDataWire {
    #[serde(rename = "_sha1")]
    sha1: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContentWire {
    #[serde(default = "default_neg_one")]
    size: i64,
    #[serde(default)]
    mime_type: String,
    #[serde(default, rename = "_sha1")]
    sha1: Option<String>,
}

fn default_neg_one() -> i64 {
    -1
}
fn default_neg_one_i32() -> i32 {
    -1
}

// HAR encodes "unknown" as `-1` for sizes / status / time and as the
// empty string for `redirectURL`. Public types map both to `None`.
fn unknown_neg_one_u64(n: i64) -> Option<u64> {
    if n == -1 { None } else { Some(n as u64) }
}

fn unknown_neg_one_f64(n: f64) -> Option<f64> {
    if n == -1.0 { None } else { Some(n) }
}

fn empty_string_to_none(s: String) -> Option<String> {
    if s.is_empty() { None } else { Some(s) }
}

impl NetworkEntry {
    pub(crate) fn from_snapshot(snapshot: Value) -> Result<Self, serde_json::Error> {
        let wire: SnapshotWire = serde_json::from_value(snapshot.clone())?;
        Ok(NetworkEntry {
            frame_ref: wire.frame_ref,
            page_ref: wire.pageref,
            monotonic_time: wire.monotonic_time,
            started_date_time: wire.started_date_time,
            time: unknown_neg_one_f64(wire.time),
            request: RequestSnapshot {
                method: wire.request.method,
                url: wire.request.url,
                http_version: wire.request.http_version,
                headers: wire
                    .request
                    .headers
                    .into_iter()
                    .map(|h| HeaderEntry {
                        name: h.name,
                        value: h.value,
                    })
                    .collect(),
                headers_size: unknown_neg_one_u64(wire.request.headers_size),
                body_size: unknown_neg_one_u64(wire.request.body_size),
                post_data: wire
                    .request
                    .post_data
                    .map(|p| RequestPostData { sha1: p.sha1 }),
            },
            response: ResponseSnapshot {
                status: if wire.response.status == -1 {
                    None
                } else {
                    Some(wire.response.status as u16)
                },
                status_text: wire.response.status_text,
                http_version: wire.response.http_version,
                headers: wire
                    .response
                    .headers
                    .into_iter()
                    .map(|h| HeaderEntry {
                        name: h.name,
                        value: h.value,
                    })
                    .collect(),
                headers_size: unknown_neg_one_u64(wire.response.headers_size),
                body_size: unknown_neg_one_u64(wire.response.body_size),
                redirect_url: empty_string_to_none(wire.response.redirect_url),
                content: ResponseContent {
                    size: unknown_neg_one_u64(wire.response.content.size),
                    mime_type: wire.response.content.mime_type,
                    sha1: wire.response.content.sha1,
                },
            },
            raw_snapshot: snapshot,
        })
    }
}
