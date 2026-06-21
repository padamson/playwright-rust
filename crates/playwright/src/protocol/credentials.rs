//! WebAuthn virtual-authenticator credentials (Playwright 1.61.0).
//!
//! Obtained via [`BrowserContext::credentials`](crate::protocol::BrowserContext::credentials).
//! Install a virtual authenticator, then register / list / delete passkeys
//! programmatically to drive `navigator.credentials.create()/get()` ceremonies
//! in tests without real hardware.
//!
//! ```no_run
//! # use playwright_rs::Playwright;
//! # async fn ex() -> playwright_rs::Result<()> {
//! # let pw = Playwright::launch().await?;
//! # let browser = pw.chromium().launch().await?;
//! # let context = browser.new_context().await?;
//! let creds = context.credentials();
//! creds.install().await?;
//! let cred = creds.create("example.com", None).await?;
//! assert_eq!(creds.get(None).await?.len(), 1);
//! creds.delete(&cred.id).await?;
//! # Ok(())
//! # }
//! ```
//!
//! See: <https://playwright.dev/docs/api/class-credentials>

use crate::error::Result;
use crate::server::channel::Channel;
use serde_json::json;

/// A virtual WebAuthn credential (passkey) held by the virtual authenticator.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct VirtualCredential {
    /// Base64url credential ID.
    pub id: String,
    /// Relying-party (origin) ID the credential is scoped to.
    pub rp_id: String,
    /// Base64url user handle, if the credential has one.
    #[serde(default)]
    pub user_handle: String,
    /// Base64url-encoded PKCS#8 private key.
    #[serde(default)]
    pub private_key: String,
    /// Base64url-encoded public key.
    #[serde(default)]
    pub public_key: String,
}

/// Optional fields for [`Credentials::create`]. When omitted, the authenticator
/// generates them.
#[derive(Debug, Default, Clone)]
#[non_exhaustive]
pub struct CredentialsCreateOptions {
    /// Explicit base64url credential ID.
    pub id: Option<String>,
    /// Base64url user handle to associate.
    pub user_handle: Option<String>,
    /// Base64url PKCS#8 private key to import.
    pub private_key: Option<String>,
    /// Base64url public key to import.
    pub public_key: Option<String>,
}

impl CredentialsCreateOptions {
    /// Set an explicit credential ID.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }
    /// Set the user handle.
    pub fn user_handle(mut self, user_handle: impl Into<String>) -> Self {
        self.user_handle = Some(user_handle.into());
        self
    }
    /// Import a specific private key (base64url PKCS#8).
    pub fn private_key(mut self, private_key: impl Into<String>) -> Self {
        self.private_key = Some(private_key.into());
        self
    }
    /// Import a specific public key (base64url).
    pub fn public_key(mut self, public_key: impl Into<String>) -> Self {
        self.public_key = Some(public_key.into());
        self
    }
}

/// Filters for [`Credentials::get`]. With no filter set, all credentials are
/// returned.
#[derive(Debug, Default, Clone)]
#[non_exhaustive]
pub struct CredentialsGetOptions {
    /// Only return credentials scoped to this relying-party ID.
    pub rp_id: Option<String>,
    /// Only return the credential with this ID.
    pub id: Option<String>,
}

impl CredentialsGetOptions {
    /// Filter by relying-party ID.
    pub fn rp_id(mut self, rp_id: impl Into<String>) -> Self {
        self.rp_id = Some(rp_id.into());
        self
    }
    /// Filter by credential ID.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }
}

/// Manages the browser context's virtual WebAuthn authenticator.
///
/// See: <https://playwright.dev/docs/api/class-credentials>
#[derive(Clone)]
pub struct Credentials {
    channel: Channel,
}

impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credentials").finish_non_exhaustive()
    }
}

impl Credentials {
    pub(crate) fn new(channel: Channel) -> Self {
        Self { channel }
    }

    /// Installs a virtual WebAuthn authenticator on the context. Call before
    /// registering credentials or driving `navigator.credentials` ceremonies.
    pub async fn install(&self) -> Result<()> {
        self.channel
            .send_no_result("credentialsInstall", json!({}))
            .await
    }

    /// Registers a virtual credential scoped to `rp_id`, returning the created
    /// credential (with any authenticator-generated fields filled in).
    pub async fn create(
        &self,
        rp_id: &str,
        options: Option<CredentialsCreateOptions>,
    ) -> Result<VirtualCredential> {
        let mut params = json!({ "rpId": rp_id });
        if let Some(o) = options {
            if let Some(id) = o.id {
                params["id"] = json!(id);
            }
            if let Some(uh) = o.user_handle {
                params["userHandle"] = json!(uh);
            }
            if let Some(pk) = o.private_key {
                params["privateKey"] = json!(pk);
            }
            if let Some(pk) = o.public_key {
                params["publicKey"] = json!(pk);
            }
        }
        #[derive(serde::Deserialize)]
        struct R {
            credential: VirtualCredential,
        }
        let r: R = self.channel.send("credentialsCreate", params).await?;
        Ok(r.credential)
    }

    /// Lists virtual credentials, optionally filtered by relying-party or ID.
    pub async fn get(
        &self,
        options: Option<CredentialsGetOptions>,
    ) -> Result<Vec<VirtualCredential>> {
        let mut params = json!({});
        if let Some(o) = options {
            if let Some(rp_id) = o.rp_id {
                params["rpId"] = json!(rp_id);
            }
            if let Some(id) = o.id {
                params["id"] = json!(id);
            }
        }
        #[derive(serde::Deserialize)]
        struct R {
            credentials: Vec<VirtualCredential>,
        }
        let r: R = self.channel.send("credentialsGet", params).await?;
        Ok(r.credentials)
    }

    /// Deletes the credential with the given ID.
    pub async fn delete(&self, id: &str) -> Result<()> {
        self.channel
            .send_no_result("credentialsDelete", json!({ "id": id }))
            .await
    }
}
