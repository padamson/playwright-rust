use super::BrowserContext;
use crate::error::{Error, Result};
use crate::server::channel_owner::ChannelOwner;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Context state: cookies, storage state, headers, permissions, geolocation, credentials, offline.
///
/// Restoring a session cookie through the storage state:
///
/// ```no_run
/// # use playwright_rs::Playwright;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// # let pw = Playwright::launch().await?;
/// # let browser = pw.chromium().launch().await?;
/// # let context = browser.new_context().await?;
/// use playwright_rs::protocol::{Cookie, StorageState};
///
/// let state = StorageState::default().cookies(vec![
///     Cookie::new("session", "token123")
///         .domain("example.com")
///         .path("/")
///         .http_only(true)
///         .secure(true)
///         .same_site("Lax"),
/// ]);
/// context.set_storage_state(state).await?;
/// # Ok(())
/// # }
/// ```
impl BrowserContext {
    /// Returns storage state for this browser context.
    ///
    /// Contains current cookies and local storage snapshots.
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-storage-state>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn storage_state(
        &self,
        options: impl Into<Option<StorageStateOptions>>,
    ) -> Result<StorageState> {
        let params = serde_json::to_value(options.into().unwrap_or_default())
            .map_err(|e| Error::ProtocolError(format!("Failed to serialize options: {e}")))?;
        let response: StorageState = self.channel().send("storageState", params).await?;
        Ok(response)
    }

    /// Replaces this context's storage state in-place via the driver's
    /// `setStorageState`, matching `browserContext.setStorageState()` in the
    /// JS/Python APIs. Useful for restoring authentication state without
    /// recreating the context.
    ///
    /// This is a **replace**, not a merge, and the driver is thorough about
    /// it. Beyond installing the cookies, origins and passkeys carried by
    /// `state`, it also:
    ///
    /// - clears the HTTP cache;
    /// - clears storage (localStorage, sessionStorage, IndexedDB, service
    ///   workers) for **every origin the context has visited**, not only the
    ///   origins listed in `state`;
    /// - when `state` carries no `credentials`, disposes an installed
    ///   virtual authenticator along with its passkeys. Capture the state
    ///   with [`StorageStateOptions::credentials`] if the context being
    ///   restored into should keep WebAuthn working.
    ///
    /// No client-visible page is opened: the driver uses an internal page,
    /// navigated to each origin, to apply origin-scoped state.
    ///
    /// # Errors
    ///
    /// Returns an error if the state fails to serialize or the driver
    /// rejects it, or if the context has closed.
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-set-storage-state>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn set_storage_state(&self, state: StorageState) -> Result<()> {
        // Delegates to the driver rather than reconstructing the state
        // client-side. The previous implementation cleared cookies, re-added
        // them, then opened a throwaway page per origin to replay
        // localStorage through `evaluate`. That could only ever restore what
        // it knew how to replay, so WebAuthn passkeys and
        // IndexedDB were silently dropped, and every origin cost a page
        // navigation.
        let storage_state = serde_json::to_value(&state)
            .map_err(|e| Error::ProtocolError(format!("Failed to serialize storage state: {e}")))?;

        self.channel()
            .send_no_result(
                "setStorageState",
                serde_json::json!({ "storageState": storage_state }),
            )
            .await
    }

    /// Adds cookies into this browser context.
    ///
    /// All pages within this context will have these cookies installed. Cookies can be granularly specified
    /// with `name`, `value`, `url`, `domain`, `path`, `expires`, `httpOnly`, `secure`, `sameSite`.
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-add-cookies>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid(), count = cookies.len()))]
    pub async fn add_cookies(&self, cookies: &[Cookie]) -> Result<()> {
        self.channel()
            .send_no_result(
                "addCookies",
                serde_json::json!({
                    "cookies": cookies
                }),
            )
            .await
    }

    /// Returns cookies for this browser context, optionally filtered by URLs.
    ///
    /// If `urls` is `None` or empty, all cookies are returned.
    ///
    /// # Arguments
    ///
    /// * `urls` - Optional list of URLs to filter cookies by
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Context has been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-cookies>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid(), count = tracing::field::Empty))]
    pub async fn cookies(&self, urls: Option<&[&str]>) -> Result<Vec<Cookie>> {
        let url_list: Vec<&str> = urls.unwrap_or(&[]).to_vec();
        #[derive(serde::Deserialize)]
        struct CookiesResponse {
            cookies: Vec<Cookie>,
        }
        let response: CookiesResponse = self
            .channel()
            .send("cookies", serde_json::json!({ "urls": url_list }))
            .await?;
        tracing::Span::current().record("count", response.cookies.len());
        Ok(response.cookies)
    }

    /// Clears cookies from this browser context, with optional filters.
    ///
    /// When called with no options, all cookies are removed. Use `ClearCookiesOptions`
    /// to filter which cookies to clear by name, domain, or path.
    ///
    /// # Arguments
    ///
    /// * `options` - Optional filters for which cookies to clear
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Context has been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-clear-cookies>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn clear_cookies(
        &self,
        options: impl Into<Option<ClearCookiesOptions>>,
    ) -> Result<()> {
        let options = options.into();
        let params = match options {
            None => serde_json::json!({}),
            Some(opts) => serde_json::to_value(opts).unwrap_or(serde_json::json!({})),
        };
        self.channel().send_no_result("clearCookies", params).await
    }

    /// Sets extra HTTP headers that will be sent with every request from this context.
    ///
    /// These headers are merged with per-page extra headers set with `page.set_extra_http_headers()`.
    /// If the page has specific headers that conflict, page-level headers take precedence.
    ///
    /// # Arguments
    ///
    /// * `headers` - Map of header names to values. All header names are lowercased.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Context has been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-set-extra-http-headers>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid(), count = headers.len()))]
    pub async fn set_extra_http_headers(&self, headers: HashMap<String, String>) -> Result<()> {
        // Playwright protocol expects an array of {name, value} objects
        let headers_array = crate::protocol::route_params::header_array(headers);
        self.channel()
            .send_no_result(
                "setExtraHTTPHeaders",
                serde_json::json!({ "headers": headers_array }),
            )
            .await
    }

    /// Grants browser permissions to the context.
    ///
    /// Permissions are granted for all pages in the context. The optional `origin`
    /// in `GrantPermissionsOptions` restricts the grant to a specific URL origin.
    ///
    /// Common permissions: `"geolocation"`, `"notifications"`, `"camera"`,
    /// `"microphone"`, `"clipboard-read"`, `"clipboard-write"`.
    ///
    /// # Arguments
    ///
    /// * `permissions` - List of permission strings to grant
    /// * `options` - Optional options, including `origin` to restrict the grant
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Permission name is not recognised
    /// - Context has been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-grant-permissions>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn grant_permissions(
        &self,
        permissions: &[&str],
        options: impl Into<Option<GrantPermissionsOptions>>,
    ) -> Result<()> {
        let options = options.into();
        let mut params = serde_json::json!({ "permissions": permissions });
        if let Some(opts) = options
            && let Some(origin) = opts.origin
        {
            params["origin"] = serde_json::Value::String(origin);
        }
        self.channel()
            .send_no_result("grantPermissions", params)
            .await
    }

    /// Clears all permission overrides for this browser context.
    ///
    /// Reverts all permissions previously set with `grant_permissions()` back to
    /// the browser default state.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Context has been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-clear-permissions>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn clear_permissions(&self) -> Result<()> {
        self.channel()
            .send_no_result("clearPermissions", serde_json::json!({}))
            .await
    }

    /// Sets or clears the geolocation for all pages in this context.
    ///
    /// Pass `Some(Geolocation { ... })` to set a specific location, or `None` to
    /// clear the override and let the browser handle location requests naturally.
    ///
    /// Note: Geolocation access requires the `"geolocation"` permission to be granted
    /// via `grant_permissions()` for navigator.geolocation to succeed.
    ///
    /// # Arguments
    ///
    /// * `geolocation` - Location to set, or `None` to clear
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Latitude or longitude is out of range
    /// - Context has been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-set-geolocation>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid()))]
    pub async fn set_geolocation(&self, geolocation: Option<Geolocation>) -> Result<()> {
        // Playwright protocol: omit the "geolocation" key entirely to clear;
        // passing null causes a validation error on the server side.
        let params = match geolocation {
            Some(geo) => serde_json::json!({ "geolocation": geo }),
            None => serde_json::json!({}),
        };
        self.channel()
            .send_no_result("setGeolocation", params)
            .await
    }

    /// Replaces the credentials used for HTTP authentication.
    ///
    /// Each request uses the first entry whose `origin` matches it; an entry
    /// without an origin matches anything. Pass an empty vector to clear.
    ///
    /// # Errors
    ///
    /// Returns an error if the context is closed or the driver rejects the
    /// credentials.
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-set-http-credentials>
    pub async fn set_http_credentials(&self, credentials: Vec<HttpCredentials>) -> Result<()> {
        self.channel()
            .send_no_result(
                "setHTTPCredentials",
                serde_json::json!({ "httpCredentials": credentials }),
            )
            .await
    }

    /// Toggles the offline mode for this browser context.
    ///
    /// When `true`, all network requests from pages in this context will fail with
    /// a network error. Set to `false` to restore network connectivity.
    ///
    /// # Arguments
    ///
    /// * `offline` - `true` to go offline, `false` to go back online
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Context has been closed
    /// - Communication with browser process fails
    ///
    /// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-set-offline>
    #[tracing::instrument(level = "debug", skip_all, fields(guid = %self.guid(), offline))]
    pub async fn set_offline(&self, offline: bool) -> Result<()> {
        self.channel()
            .send_no_result("setOffline", serde_json::json!({ "offline": offline }))
            .await
    }
}

/// Geolocation coordinates.
///
/// See: <https://playwright.dev/docs/api/class-browser#browser-new-context>
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Geolocation {
    /// Latitude between -90 and 90
    pub latitude: f64,
    /// Longitude between -180 and 180
    pub longitude: f64,
    /// Optional accuracy in meters (default: 0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accuracy: Option<f64>,
}

/// When to send HTTP credentials.
///
/// See: <https://playwright.dev/docs/api/class-browser#browser-new-context-option-http-credentials>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum HttpCredentialsSend {
    /// Send the `Authorization` header up front rather than waiting to be
    /// challenged.
    ///
    /// **Only honored by [`APIRequestContext`](crate::protocol::APIRequestContext)
    /// fetches**, matching upstream: browser navigation stays reactive on
    /// every engine, so this is a no-op for page loads. Reach for it when a
    /// server answers `403` instead of `401`, which leaves nothing to react
    /// to.
    Always,
    /// Send it only after the server answers `401`. The default.
    Unauthorized,
}

/// Credentials for HTTP authentication.
///
/// A context can hold several: the first whose `origin` matches the request
/// is used, and an entry without an origin matches any request.
///
/// See: <https://playwright.dev/docs/api/class-browser#browser-new-context-option-http-credentials>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct HttpCredentials {
    /// Username to authenticate with.
    pub username: String,
    /// Password to authenticate with.
    pub password: String,
    /// Restrict these credentials to one origin (scheme, host, and port,
    /// e.g. `https://example.com`). Without it they match any request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    /// When to send the header. Defaults to after a `401`, and only an
    /// [`APIRequestContext`](crate::protocol::APIRequestContext) honors
    /// anything else; see [`HttpCredentialsSend`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub send: Option<HttpCredentialsSend>,
}

impl HttpCredentials {
    /// Credentials matching any origin.
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
            origin: None,
            send: None,
        }
    }

    /// Restrict these credentials to one origin.
    pub fn origin(mut self, origin: impl Into<String>) -> Self {
        self.origin = Some(origin.into());
        self
    }

    /// Choose when the `Authorization` header is sent.
    pub fn send(mut self, send: HttpCredentialsSend) -> Self {
        self.send = Some(send);
        self
    }
}

/// Cookie information for storage state.
///
/// See: <https://playwright.dev/docs/api/class-browser#browser-new-context-option-storage-state>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Cookie {
    /// Cookie name
    pub name: String,
    /// Cookie value
    pub value: String,
    /// Cookie domain (use dot prefix for subdomain matching, e.g., ".example.com")
    pub domain: String,
    /// Cookie path
    pub path: String,
    /// Unix timestamp in seconds; -1 for session cookies
    pub expires: f64,
    /// HTTP-only flag
    pub http_only: bool,
    /// Secure flag
    pub secure: bool,
    /// SameSite attribute ("Strict", "Lax", "None")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub same_site: Option<String>,
}

impl Cookie {
    /// Create a session cookie (no expiry) with the given name and value.
    /// Set `domain`+`path` (or serve it for a URL) before adding it.
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            domain: String::new(),
            path: "/".to_string(),
            expires: -1.0,
            http_only: false,
            secure: false,
            same_site: None,
        }
    }
    /// Cookie domain (e.g. "example.com").
    pub fn domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = domain.into();
        self
    }
    /// Cookie path.
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }
    /// Expiry as Unix time in seconds (-1 for a session cookie).
    pub fn expires(mut self, expires: f64) -> Self {
        self.expires = expires;
        self
    }
    /// Mark the cookie HttpOnly.
    pub fn http_only(mut self, http_only: bool) -> Self {
        self.http_only = http_only;
        self
    }
    /// Mark the cookie Secure.
    pub fn secure(mut self, secure: bool) -> Self {
        self.secure = secure;
        self
    }
    /// SameSite attribute ("Strict", "Lax", or "None").
    pub fn same_site(mut self, same_site: impl Into<String>) -> Self {
        self.same_site = Some(same_site.into());
        self
    }
}

/// Local storage item for storage state.
///
/// See: <https://playwright.dev/docs/api/class-browser#browser-new-context-option-storage-state>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LocalStorageItem {
    /// Storage key
    pub name: String,
    /// Storage value
    pub value: String,
}

impl LocalStorageItem {
    /// A single localStorage key/value pair.
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
}

/// Origin with local storage items for storage state.
///
/// See: <https://playwright.dev/docs/api/class-browser#browser-new-context-option-storage-state>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct Origin {
    /// Origin URL (e.g., `https://example.com`)
    pub origin: String,
    /// Local storage items for this origin
    pub local_storage: Vec<LocalStorageItem>,
    /// IndexedDB contents for this origin, as the driver's opaque payload.
    ///
    /// Populated when the state was captured with
    /// [`StorageStateOptions::indexed_db`], and passed back verbatim on
    /// restore. Kept as raw JSON rather than modelled: the shape is an
    /// implementation detail of the driver's snapshot format, and the only
    /// supported operation is carrying it back unchanged.
    #[serde(rename = "indexedDB", default, skip_serializing_if = "Option::is_none")]
    pub indexed_db: Option<serde_json::Value>,
    /// This origin's private file system, as the driver's opaque payload.
    ///
    /// Populated when the state was captured with
    /// [`StorageStateOptions::opfs`], and passed back verbatim on restore.
    /// Kept as raw JSON for the same reason as `indexed_db`: the shape is
    /// the driver's snapshot format, and the only supported operation is
    /// carrying it back unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opfs: Option<serde_json::Value>,
}

impl Origin {
    /// Storage entries for one origin.
    pub fn new(origin: impl Into<String>, local_storage: Vec<LocalStorageItem>) -> Self {
        Self {
            origin: origin.into(),
            local_storage,
            indexed_db: None,
            opfs: None,
        }
    }
}

/// Storage state containing cookies and local storage.
///
/// Used to populate a browser context with saved authentication state,
/// enabling session persistence across context instances.
///
/// See: <https://playwright.dev/docs/api/class-browser#browser-new-context-option-storage-state>
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct StorageState {
    /// List of cookies
    pub cookies: Vec<Cookie>,
    /// List of origins with local storage
    pub origins: Vec<Origin>,
    /// WebAuthn passkeys held by the context's virtual authenticator.
    /// Only populated when the state was captured with
    /// [`StorageStateOptions::credentials`]; omitted from the wire when empty
    /// so a state captured without them is unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credentials: Option<Vec<crate::protocol::VirtualCredential>>,
}

impl StorageState {
    /// Cookies to seed the context with.
    pub fn cookies(mut self, cookies: Vec<Cookie>) -> Self {
        self.cookies = cookies;
        self
    }
    /// Per-origin storage (localStorage) to seed the context with.
    pub fn origins(mut self, origins: Vec<Origin>) -> Self {
        self.origins = origins;
        self
    }
    /// WebAuthn passkeys to seed the context's virtual authenticator with.
    pub fn credentials(mut self, credentials: Vec<crate::protocol::VirtualCredential>) -> Self {
        self.credentials = Some(credentials);
        self
    }
}

/// Options for [`BrowserContext::storage_state`].
///
/// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-storage-state>
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct StorageStateOptions {
    /// Include IndexedDB contents in the captured state.
    // camelCase would render this "indexedDb"; the protocol field is
    // "indexedDB", and the driver drops unknown parameters silently, so the
    // wrong casing is not an error but a no-op.
    #[serde(rename = "indexedDB", skip_serializing_if = "Option::is_none")]
    pub indexed_db: Option<bool>,
    /// Include the virtual authenticator's WebAuthn passkeys.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials: Option<bool>,
    /// Include each origin's private file system in the captured state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opfs: Option<bool>,
}

impl StorageStateOptions {
    /// Include IndexedDB contents in the captured state.
    pub fn indexed_db(mut self, include: bool) -> Self {
        self.indexed_db = Some(include);
        self
    }
    /// Include the virtual authenticator's WebAuthn passkeys.
    pub fn credentials(mut self, include: bool) -> Self {
        self.credentials = Some(include);
        self
    }
    /// Include each origin's private file system in the captured state.
    pub fn opfs(mut self, include: bool) -> Self {
        self.opfs = Some(include);
        self
    }
}

/// Options for filtering which cookies to clear with `BrowserContext::clear_cookies()`.
///
/// All fields are optional; when provided they act as AND-combined filters.
///
/// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-clear-cookies>
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ClearCookiesOptions {
    /// Filter by cookie name (exact match).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Filter by cookie domain.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// Filter by cookie path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

impl ClearCookiesOptions {
    /// Only clear cookies with this name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
    /// Only clear cookies for this domain.
    pub fn domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = Some(domain.into());
        self
    }
    /// Only clear cookies for this path.
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }
}

/// Options for `BrowserContext::grant_permissions()`.
///
/// See: <https://playwright.dev/docs/api/class-browsercontext#browser-context-grant-permissions>
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct GrantPermissionsOptions {
    /// Optional origin to restrict the permission grant to.
    ///
    /// For example `"https://example.com"`.
    pub origin: Option<String>,
}

impl GrantPermissionsOptions {
    /// Restrict the grant to the given origin.
    pub fn origin(mut self, origin: impl Into<String>) -> Self {
        self.origin = Some(origin.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_state_options_serialize_with_protocol_casing() {
        // The driver validates "indexedDB" and silently drops unknown keys,
        // so serde's camelCase ("indexedDb") would make the flag a no-op
        // that reports success.
        let opts = StorageStateOptions::default()
            .indexed_db(true)
            .credentials(true);
        let value = serde_json::to_value(&opts).unwrap();
        assert_eq!(
            value,
            serde_json::json!({ "indexedDB": true, "credentials": true })
        );
    }

    #[test]
    fn http_credentials_serialize_with_protocol_casing() {
        // The driver validates `send` against an enum, so a wrong spelling is
        // a hard error rather than a silent drop; and an unset field must not
        // serialize as null.
        let bare = HttpCredentials::new("user", "secret");
        assert_eq!(
            serde_json::to_value(&bare).unwrap(),
            serde_json::json!({ "username": "user", "password": "secret" })
        );

        let full = HttpCredentials::new("user", "secret")
            .origin("https://example.test")
            .send(HttpCredentialsSend::Always);
        assert_eq!(
            serde_json::to_value(&full).unwrap(),
            serde_json::json!({
                "username": "user",
                "password": "secret",
                "origin": "https://example.test",
                "send": "always",
            })
        );

        assert_eq!(
            serde_json::to_value(HttpCredentialsSend::Unauthorized).unwrap(),
            serde_json::json!("unauthorized")
        );
    }

    #[test]
    fn storage_state_round_trips_credentials_content() {
        // A captured state's value is in being restorable; this pins that a
        // save/load through JSON preserves credential content, not just
        // count.
        // Exactly the driver's VirtualCredential schema: five required
        // string fields.
        let json = serde_json::json!({
            "cookies": [],
            "origins": [],
            "credentials": [{
                "id": "Y3JlZA",
                "rpId": "example.com",
                "userHandle": "dXNlcg",
                "privateKey": "cGtleQ",
                "publicKey": "cHVi"
            }]
        });
        let state: StorageState = serde_json::from_value(json.clone()).unwrap();
        let back = serde_json::to_value(&state).unwrap();
        assert_eq!(back["credentials"], json["credentials"]);
    }

    #[test]
    fn origin_round_trips_indexed_db_payload_verbatim() {
        // The payload is the driver's opaque snapshot format; the contract
        // is carrying it back unchanged, under the protocol's exact casing.
        let json = serde_json::json!({
            "origin": "https://example.com",
            "localStorage": [],
            "indexedDB": [{"name": "db", "version": 1, "stores": []}]
        });
        let origin: Origin = serde_json::from_value(json.clone()).unwrap();
        let back = serde_json::to_value(&origin).unwrap();
        assert_eq!(back["indexedDB"], json["indexedDB"]);
        assert!(back.get("indexedDb").is_none());
    }
}
