//! Serve a page's requests from an in-process [`tower::Service`], with no
//! socket.
//!
//! [`Page::route_service`](crate::protocol::Page::route_service) and
//! [`BrowserContext::route_service`](crate::protocol::BrowserContext::route_service)
//! register a route whose handler hands each matching request to a tower
//! `Service` and fulfills the request with whatever the service returns. An
//! axum `Router`, a tower-http `ServeDir`, a `tower::service_fn` closure: any
//! service that takes an `http::Request` and returns an `http::Response`
//! qualifies. The browser sees a normal HTTP response; no listener is bound,
//! no port is chosen, and no server task runs.
//!
//! It is the same mechanism as [`route_from_har`](crate::protocol::Page::route_from_har):
//! a source of responses wired into route interception. Everything it does is
//! expressible with [`route`](crate::protocol::Page::route) and
//! [`Route::fulfill`](crate::protocol::Route::fulfill); it removes the
//! conversion boilerplate and makes the pattern a one-liner.
//!
//! # Where it fits
//!
//! - **A Rust frontend with a Rust backend.** A Leptos, Dioxus, or Yew bundle
//!   plus the axum app that serves it can be tested end to end inside one
//!   test process: the router the app uses in production is the router the
//!   browser talks to.
//! - **Hermetic tests.** Nothing touches the network stack, so the test runs
//!   the same in a sandbox, a locked-down CI runner, or a container with no
//!   loopback access, and two tests never compete for a port.
//! - **Any origin.** The pattern decides which URLs the service owns, so the
//!   app can be served at `https://app.example/` without a certificate, and
//!   secure-context features (`isSecureContext`, WebAuthn, service workers'
//!   registration) behave as they would in production.
//! - **Per-page or per-context scope.** Register on a page to serve only that
//!   page, or on a context to serve every page it opens.
//!
//! # Serving an axum app
//!
//! ```no_run
//! use axum::{Router, routing::get};
//! use playwright_rs::Playwright;
//! use playwright_rs::expect;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let app = Router::new()
//!     .route("/", get(|| async { axum::response::Html("<h1>Hello</h1>") }))
//!     .route("/api/todos", get(|| async { axum::Json(vec!["write tests"]) }));
//!
//! let pw = Playwright::launch().await?;
//! let browser = pw.chromium().launch().await?;
//! let page = browser.new_page().await?;
//!
//! // Every request to this origin goes to `app`. No server was started.
//! page.route_service("https://app.example/**", app).await?;
//! page.goto("https://app.example/", None).await?;
//!
//! expect(page.locator("h1")).to_have_text("Hello").await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Serving a built wasm bundle
//!
//! A frontend compiled to WebAssembly is a directory of static files. Serve
//! it with tower-http's `ServeDir` and drive it like any other page:
//!
//! ```no_run
//! use playwright_rs::Playwright;
//! use playwright_rs::expect;
//! use tower_http::services::ServeDir;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let pw = Playwright::launch().await?;
//! let browser = pw.chromium().launch().await?;
//! let page = browser.new_page().await?;
//!
//! page.route_service("https://app.example/**", ServeDir::new("dist")).await?;
//! page.goto("https://app.example/", None).await?;
//!
//! // The wasm module booted once it has rendered something to wait on.
//! expect(page.locator("#app h1")).to_be_visible().await?;
//! # Ok(())
//! # }
//! ```
//!
//! An axum `Router` can combine the two: API routes first, then
//! `.fallback_service(ServeDir::new("dist"))` for the bundle.
//!
//! # What the service sees
//!
//! The request is rebuilt from what the browser sent: the method, the
//! absolute URL as the URI, the request headers in the order the driver
//! reports them (names lowercased, repeats kept), and the body for methods
//! that carry one. A router that matches on path works unchanged, since
//! `Uri::path` is the path of an absolute URI; one that reads `Host` finds
//! the origin from the pattern. Cookies the service set come back on later
//! requests like any other header, in all three engines; WebKit reports an
//! intercepted request before attaching its cookie header, so there the
//! header is filled from the context's cookie jar. The driver withholds
//! some request bodies, though: a multipart form with a file part, or a very
//! large body, reaches the handler without its body, and such a request is
//! aborted with an error saying so rather than handed to the service
//! bodiless. File uploads need a real listener.
//!
//! The response's status, headers, and collected body become the fulfilled
//! response. A header that appears more than once is joined with `, `, except
//! `set-cookie`, which is joined with a newline so the browser can split it
//! back into separate cookies. `content-length` and `transfer-encoding` are
//! dropped and recomputed from the body actually delivered.
//!
//! If the service returns an error, or its body fails to collect, the route
//! dispatcher logs the error at `warn` and aborts the request, as it now does
//! for any route handler that fails before handling its route. The browser
//! sees a failed request, not a hang. The one gap is a response the driver
//! itself refuses after the fulfill has begun, which can no longer be
//! aborted; that error is logged and the request waits out the browser's
//! timeout.
//!
//! # What it is not
//!
//! Route interception is not the network, and a test that passes here can
//! differ from one against a real listener in these ways:
//!
//! - **Bodies are whole.** The response is collected before it is delivered,
//!   so streaming responses and server-sent events complete before the
//!   browser sees the first byte. A service that never ends its body never
//!   fulfills.
//! - **WebSockets take a different path.** Use
//!   [`route_web_socket`](crate::protocol::Page::route_web_socket) for them;
//!   a `Service` sees only the HTTP upgrade request, and returning it is not
//!   a connection.
//! - **HTTP/1 semantics, no connection.** There is no TLS handshake,
//!   compression negotiation, HTTP/2, keep-alive, or `serverAddr`; timing
//!   data reflects the round trip through the driver, not a socket.
//! - **WebKit refuses redirects.** WebKit's interception cannot fulfill a
//!   3xx, so a service answer such as axum's `Redirect` or `ServeDir`'s
//!   trailing-slash redirect is aborted there with an error naming it,
//!   rather than followed. Chromium and Firefox follow it into a new
//!   routed request.
//! - **Cost per request.** Each request crosses the driver's JSON-RPC
//!   channel with its body, so very large assets are slower than a local
//!   listener would serve them. Fine for an app bundle; not a load test.
//!
//! For an app that depends on any of those, bind a listener on port `0` and
//! serve it with `axum::serve`; the browser and the service then talk over
//! real sockets.
//!
//! # Testing a wasm frontend
//!
//! The bundle boots asynchronously, so wait on something it renders rather
//! than on `goto` returning: a locator for the first element the app mounts
//! auto-waits, and [`wait_for_function`](crate::protocol::Page::wait_for_function)
//! covers a readiness flag the app sets on `window`. Canvas-rendered UIs give
//! DOM locators nothing to see; assert on pixels or on a test-facing hook the
//! app exports, and use [`Locator::drag_to`](crate::protocol::Locator::drag_to)
//! and pointer actions with positions for interaction. The crate's
//! `examples/canvas_pixels.rs` walks through canvas assertions.

use crate::error::{Error, Result};
use crate::protocol::browser_context::{BrowserContext, Cookie};
use crate::protocol::route::{FulfillOptions, Route};
use crate::protocol::route_params::merge_headers;
use bytes::Bytes;
use http::{HeaderMap, HeaderName, HeaderValue, Method, Request, Response, StatusCode, Uri};
use http_body_util::{BodyExt, Full};
use tower::{Service, ServiceExt};

/// Errors a service or its body may return; tower's own alias.
pub use tower::BoxError;

/// The request body type a `route_service` service receives.
pub type ServiceRequest = Request<Full<Bytes>>;

/// A tower `Service` that `route_service` can drive.
///
/// Implemented for every service that takes a [`ServiceRequest`], returns an
/// `http::Response` with any body, and can be cloned and shared across the
/// route handler's threads: axum's `Router`, tower-http's `ServeDir`, and a
/// `tower::service_fn` over a `Send + Sync` closure all qualify. The trait
/// exists so the bound is written once; there is nothing to implement.
pub trait RouteService:
    Service<ServiceRequest, Response = Response<Self::Body>, Future: Send, Error: Into<BoxError>>
    + Clone
    + Send
    + Sync
    + 'static
{
    /// The response body type the service produces.
    type Body: http_body::Body<Data: Send, Error: Into<BoxError>> + Send + 'static;
}

impl<S, B> RouteService for S
where
    S: Service<ServiceRequest, Response = Response<B>> + Clone + Send + Sync + 'static,
    S::Future: Send,
    S::Error: Into<BoxError>,
    B: http_body::Body + Send + 'static,
    B::Data: Send,
    B::Error: Into<BoxError>,
{
    type Body = B;
}

/// Hand the route's request to `service` and fulfill the route with its
/// response. An error before the fulfill starts is returned to the route
/// dispatcher, which aborts the still-unhandled request. A fulfill the driver
/// refuses after it has started cannot be aborted any more; that error is
/// logged and the request waits out the browser's timeout.
///
/// `context` is the one the caller registered on, captured when it was a hop
/// away; `None` falls back to walking this request's object chain.
pub(crate) async fn fulfill_from_service<S: RouteService>(
    route: Route,
    service: S,
    context: Option<BrowserContext>,
) -> Result<()> {
    let options = service_response(&route, service, context).await?;
    route.fulfill(Some(options)).await
}

/// The fulfill options for the service's answer to the route's request.
async fn service_response<S: RouteService>(
    route: &Route,
    service: S,
    context: Option<BrowserContext>,
) -> Result<FulfillOptions> {
    let request = route.request();
    let context = context.or_else(|| request_context(route));
    let is_webkit = context
        .as_ref()
        .and_then(BrowserContext::browser)
        .is_some_and(|browser| browser.name() == "webkit");

    let mut headers = request.header_pairs();
    if is_webkit
        && !headers.iter().any(|(name, _)| name == "cookie")
        && let Some(context) = &context
    {
        // WebKit reports an intercepted request before its cookie header is
        // attached, so the service would see every request as logged out.
        // The context's jar has what the browser would have sent.
        let cookies = context.cookies(Some(&[request.url()])).await?;
        if let Some(cookie) = cookie_header(&cookies) {
            headers.push(("cookie".to_string(), cookie));
        }
    }

    let http_request = request_from_parts(
        request.method(),
        request.url(),
        headers
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str())),
        request.post_data_buffer(),
    )?;

    let response = service.oneshot(http_request).await.map_err(|error| {
        Error::ServerError(format!(
            "route_service: the service failed for {}: {}",
            request.url(),
            error.into()
        ))
    })?;

    let (parts, body) = response.into_parts();
    if is_webkit && parts.status.is_redirection() {
        // WebKit's interception refuses to fulfill a 3xx. Refusing here, before
        // the fulfill starts, keeps the request abortable.
        return Err(Error::ServerError(format!(
            "route_service: WebKit cannot fulfill the {} redirect the service returned for {}",
            parts.status,
            request.url()
        )));
    }

    let body = body.collect().await.map_err(|error| {
        Error::ServerError(format!(
            "route_service: the response body for {} failed: {}",
            request.url(),
            error.into()
        ))
    })?;

    Ok(fulfill_options(
        parts.status,
        &parts.headers,
        body.to_bytes(),
    ))
}

/// The context serving this route, when the object chain from the request
/// to its page is intact.
fn request_context(route: &Route) -> Option<BrowserContext> {
    route.request().frame()?.page()?.context().ok()
}

/// The `cookie` request header carrying `cookies`, in the jar's order.
pub(crate) fn cookie_header(cookies: &[Cookie]) -> Option<String> {
    if cookies.is_empty() {
        return None;
    }
    Some(
        cookies
            .iter()
            .map(|cookie| format!("{}={}", cookie.name, cookie.value))
            .collect::<Vec<_>>()
            .join("; "),
    )
}

/// An `http::Request` rebuilt from the parts the driver reports for a route.
///
/// Headers are added in the order given, repeats included; a value with
/// bytes outside ASCII is carried as the browser sent it. Names that are not
/// valid HTTP field names (pseudo-headers such as `:authority`, which
/// Chromium reports for HTTP/2 origins) are skipped.
///
/// The driver withholds some request bodies (a multipart form with a file
/// part, very large bodies). A body-carrying request that arrives without
/// its body is an error, not a bodiless request the service would misread.
pub(crate) fn request_from_parts<'a>(
    method: &str,
    url: &str,
    headers: impl IntoIterator<Item = (&'a str, &'a str)>,
    body: Option<Vec<u8>>,
) -> Result<ServiceRequest> {
    let method = Method::from_bytes(method.as_bytes()).map_err(|error| {
        Error::ProtocolError(format!("route_service: invalid method {method:?}: {error}"))
    })?;
    let uri: Uri = url.parse().map_err(|error| {
        Error::ProtocolError(format!("route_service: invalid URL {url:?}: {error}"))
    })?;

    let mut declared_length: Option<u64> = None;
    let mut builder = Request::builder().method(method).uri(uri);
    for (name, value) in headers {
        if name == "content-length" {
            declared_length = value.trim().parse().ok();
        }
        let (Ok(name), Ok(value)) = (
            HeaderName::from_bytes(name.as_bytes()),
            HeaderValue::from_bytes(value.as_bytes()),
        ) else {
            continue;
        };
        builder = builder.header(name, value);
    }

    if body.is_none()
        && let Some(length) = declared_length
        && length > 0
    {
        return Err(Error::ProtocolError(format!(
            "route_service: the driver did not expose the {length}-byte request body for {url} \
             (multipart file uploads and very large bodies are withheld from route handlers); \
             serve this request from a real listener"
        )));
    }

    builder
        .body(Full::new(Bytes::from(body.unwrap_or_default())))
        .map_err(|error| Error::ProtocolError(format!("route_service: {error}")))
}

/// The `FulfillOptions` that deliver a service response.
///
/// Repeated headers are joined with `, `, except `set-cookie`, which is
/// joined with a newline for the browser to split. Framing headers are
/// dropped because the body is delivered whole and `fulfill` recomputes
/// `content-length` from it. A value with bytes outside ASCII (a UTF-8
/// filename in `content-disposition`, say) is carried through as text.
pub(crate) fn fulfill_options(
    status: StatusCode,
    headers: &HeaderMap,
    body: Bytes,
) -> FulfillOptions {
    let pairs = headers.iter().filter_map(|(name, value)| {
        let name = name.as_str();
        if name == "content-length" || name == "transfer-encoding" {
            return None;
        }
        Some((name, String::from_utf8_lossy(value.as_bytes())))
    });

    FulfillOptions::builder()
        .status(status.as_u16())
        .headers(merge_headers(pairs, Some(", ")))
        .body(Vec::from(body))
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::route_params::headers_of;

    #[tokio::test]
    async fn request_carries_method_absolute_uri_headers_and_body() {
        let request = request_from_parts(
            "POST",
            "https://app.example/api/items?x=1",
            [("content-type", "text/plain"), ("host", "app.example")],
            Some(b"payload".to_vec()),
        )
        .unwrap();

        assert_eq!(request.method(), Method::POST);
        assert_eq!(request.uri().path(), "/api/items");
        assert_eq!(request.uri().query(), Some("x=1"));
        assert_eq!(request.uri().host(), Some("app.example"));
        assert_eq!(request.headers()["content-type"], "text/plain");
        assert_eq!(request.headers()["host"], "app.example");
        let body = request.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(&body[..], b"payload");
    }

    #[tokio::test]
    async fn request_without_a_body_has_an_empty_body() {
        let request =
            request_from_parts("GET", "http://app.example/", std::iter::empty(), None).unwrap();

        let body = request.into_body().collect().await.unwrap().to_bytes();
        assert!(body.is_empty());
    }

    #[test]
    fn cookie_header_joins_the_jar_and_is_absent_when_empty() {
        let cookie = |name: &str, value: &str| Cookie {
            name: name.to_string(),
            value: value.to_string(),
            domain: "app.test".to_string(),
            path: "/".to_string(),
            expires: -1.0,
            http_only: false,
            secure: false,
            same_site: None,
        };

        assert_eq!(cookie_header(&[]), None);
        assert_eq!(
            cookie_header(&[cookie("session", "abc"), cookie("theme", "dark")]).as_deref(),
            Some("session=abc; theme=dark")
        );
    }

    #[test]
    fn request_carries_non_ascii_header_values() {
        let request = request_from_parts(
            "GET",
            "https://app.example/",
            [("x-name", "r\u{e9}sum\u{e9}")],
            None,
        )
        .unwrap();

        assert_eq!(
            request.headers()["x-name"].as_bytes(),
            "r\u{e9}sum\u{e9}".as_bytes()
        );
    }

    #[test]
    fn request_keeps_header_order_and_repeats() {
        let request = request_from_parts(
            "GET",
            "https://app.example/",
            [("cookie", "a=1"), ("accept", "*/*"), ("cookie", "b=2")],
            None,
        )
        .unwrap();

        let cookies: Vec<&str> = request
            .headers()
            .get_all("cookie")
            .iter()
            .map(|v| v.to_str().unwrap())
            .collect();
        assert_eq!(cookies, ["a=1", "b=2"]);
        assert_eq!(request.headers().len(), 3);
    }

    #[test]
    fn request_skips_pseudo_headers_and_keeps_the_rest() {
        let request = request_from_parts(
            "GET",
            "https://app.example/",
            [(":authority", "app.example"), ("accept", "*/*")],
            None,
        )
        .unwrap();

        assert_eq!(request.headers().len(), 1);
        assert_eq!(request.headers()["accept"], "*/*");
    }

    #[test]
    fn request_rejects_an_unparseable_url() {
        let err = request_from_parts("GET", "not a url", std::iter::empty(), None).unwrap_err();

        assert!(matches!(err, Error::ProtocolError(msg) if msg.contains("not a url")));
    }

    #[test]
    fn request_rejects_an_invalid_method() {
        let err = request_from_parts("G ET", "http://app.example/", std::iter::empty(), None)
            .unwrap_err();

        assert!(matches!(err, Error::ProtocolError(msg) if msg.contains("G ET")));
    }

    #[test]
    fn fulfill_maps_status_headers_and_body() {
        let mut map = HeaderMap::new();
        map.insert("content-type", HeaderValue::from_static("application/json"));
        map.insert("x-one", HeaderValue::from_static("1"));

        let opts = fulfill_options(StatusCode::CREATED, &map, Bytes::from_static(b"{}"));

        assert_eq!(opts.status, Some(201));
        assert_eq!(opts.body.as_deref(), Some(&b"{}"[..]));
        assert_eq!(opts.content_type, None);
        assert_eq!(
            opts.headers.unwrap(),
            headers_of(&[("content-type", "application/json"), ("x-one", "1")])
        );
    }

    #[test]
    fn fulfill_joins_repeated_headers_and_newlines_set_cookie() {
        let mut map = HeaderMap::new();
        map.append("set-cookie", HeaderValue::from_static("a=1"));
        map.append("set-cookie", HeaderValue::from_static("b=2"));
        map.append("vary", HeaderValue::from_static("accept"));
        map.append("vary", HeaderValue::from_static("origin"));

        let merged = fulfill_options(StatusCode::OK, &map, Bytes::new())
            .headers
            .unwrap();

        assert_eq!(merged["set-cookie"], "a=1\nb=2");
        assert_eq!(merged["vary"], "accept, origin");
    }

    #[test]
    fn request_refuses_a_body_the_driver_withheld() {
        let with_body = request_from_parts(
            "POST",
            "https://app.example/",
            [("content-length", "7")],
            Some(b"payload".to_vec()),
        )
        .unwrap();
        assert_eq!(with_body.headers()["content-length"], "7");

        let bodiless_get = request_from_parts(
            "GET",
            "https://app.example/",
            [("content-length", "0"), ("accept", "*/*")],
            None,
        )
        .unwrap();
        assert_eq!(bodiless_get.headers()["accept"], "*/*");

        let withheld = request_from_parts(
            "POST",
            "https://app.example/upload",
            [("content-length", "70000")],
            None,
        )
        .unwrap_err();
        assert!(
            matches!(withheld, Error::ProtocolError(msg) if msg.contains("70000-byte") && msg.contains("withheld")),
        );
    }

    #[test]
    fn fulfill_keeps_non_ascii_header_values() {
        let mut map = HeaderMap::new();
        map.insert(
            "content-disposition",
            HeaderValue::from_bytes("attachment; filename=\"r\u{e9}sum\u{e9}.pdf\"".as_bytes())
                .unwrap(),
        );

        let merged = fulfill_options(StatusCode::OK, &map, Bytes::new())
            .headers
            .unwrap();

        assert_eq!(
            merged["content-disposition"],
            "attachment; filename=\"r\u{e9}sum\u{e9}.pdf\""
        );
    }

    #[test]
    fn fulfill_drops_framing_headers_the_body_makes_stale() {
        let mut map = HeaderMap::new();
        map.insert("content-length", HeaderValue::from_static("999"));
        map.insert("transfer-encoding", HeaderValue::from_static("chunked"));
        map.insert("etag", HeaderValue::from_static("\"v1\""));

        let merged = fulfill_options(StatusCode::OK, &map, Bytes::from_static(b"abc"))
            .headers
            .unwrap();

        assert_eq!(merged, headers_of(&[("etag", "\"v1\"")]));
    }
}
