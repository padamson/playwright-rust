// HAR recording options for Tracing::start_har.
//
// Kept in its own module (rather than in tracing.rs) so the pure
// RecordHarOptions serialization is covered by mutation testing, while the
// integration-only start_har/stop_har stay out of scope.

use serde::Serialize;
use serde_json::Value;

/// How resource bodies are stored in a recorded HAR.
///
/// See: <https://playwright.dev/docs/api/class-tracing#tracing-start-har-option-content>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum HarContent {
    /// Do not store bodies (smallest HAR).
    Omit,
    /// Inline bodies into the HAR as base64 (the default for a non-`.zip` path).
    Embed,
    /// Store bodies as separate files / zip entries (the default for a `.zip` path).
    Attach,
}

/// Level of detail recorded in a HAR.
///
/// See: <https://playwright.dev/docs/api/class-tracing#tracing-start-har-option-mode>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum HarMode {
    /// Record everything (default).
    Full,
    /// Record only essentials (size, timing) and omit headers/bodies/cookies.
    Minimal,
}

/// Options for [`Tracing::start_har`](crate::protocol::Tracing::start_har).
///
/// See: <https://playwright.dev/docs/api/class-tracing#tracing-start-har>
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct StartHarOptions {
    /// How resource bodies are stored. Defaults to `Attach` for a `.zip` path,
    /// `Embed` otherwise.
    pub content: Option<HarContent>,
    /// Level of detail. Defaults to [`HarMode::Full`].
    pub mode: Option<HarMode>,
    /// Glob pattern; only requests whose URL matches are recorded.
    pub url_filter: Option<String>,
    /// Directory to store `attach`-mode resource files in (for non-zip paths).
    pub resources_dir: Option<String>,
}

impl StartHarOptions {
    /// Build the protocol `RecordHarOptions` object for the given output path.
    ///
    /// `harPath` is intentionally omitted: setting it makes the driver write its
    /// own archive at that path (appending `.zip`), which would duplicate the
    /// file we already produce via `harExport` + unzip in `stop_har`. The path
    /// here only selects the default `content` mode.
    pub(crate) fn to_record_har_json(&self, path: &str) -> Value {
        let is_zip = path.ends_with(".zip");
        let content = self.content.unwrap_or(if is_zip {
            HarContent::Attach
        } else {
            HarContent::Embed
        });
        let mode = self.mode.unwrap_or(HarMode::Full);

        let mut o = serde_json::json!({});
        o["content"] = serde_json::to_value(content).expect("serialize HarContent cannot fail");
        o["mode"] = serde_json::to_value(mode).expect("serialize HarMode cannot fail");
        if let Some(glob) = &self.url_filter {
            o["urlGlob"] = serde_json::json!(glob);
        }
        if let Some(dir) = &self.resources_dir {
            o["resourcesDir"] = serde_json::json!(dir);
        }
        o
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_har_options_zip_defaults_to_attach() {
        let json = StartHarOptions::default().to_record_har_json("run.har.zip");
        assert_eq!(json["content"], "attach");
        assert_eq!(json["mode"], "full");
        // harPath is deliberately not sent (avoids the driver double-writing).
        assert!(json.get("harPath").is_none());
    }

    #[test]
    fn test_start_har_options_plain_defaults_to_embed() {
        let json = StartHarOptions::default().to_record_har_json("run.har");
        assert_eq!(json["content"], "embed");
    }

    #[test]
    fn test_start_har_options_explicit() {
        let opts = StartHarOptions {
            content: Some(HarContent::Omit),
            mode: Some(HarMode::Minimal),
            url_filter: Some("**/api/**".to_string()),
            resources_dir: None,
        };
        let json = opts.to_record_har_json("run.har");
        assert_eq!(json["content"], "omit");
        assert_eq!(json["mode"], "minimal");
        assert_eq!(json["urlGlob"], "**/api/**");
        assert!(json.get("resourcesDir").is_none());
    }
}
