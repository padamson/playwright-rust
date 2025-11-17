// Select option variants for dropdown selection
//
// Provides different ways to select options: by value, label, or index.

/// Select option variant
///
/// Represents different ways to select an option in a `<select>` element.
///
/// # Example
///
/// ```ignore
/// use playwright_rs::protocol::SelectOption;
///
/// // Select by value
/// let opt = SelectOption::Value("option1".to_string());
///
/// // Select by label (visible text)
/// let opt = SelectOption::Label("First Option".to_string());
///
/// // Select by index (0-based)
/// let opt = SelectOption::Index(0);
/// ```
///
/// See: <https://playwright.dev/docs/api/class-locator#locator-select-option>
#[derive(Debug, Clone, PartialEq)]
pub enum SelectOption {
    /// Select by option value attribute
    Value(String),
    /// Select by option label (visible text)
    Label(String),
    /// Select by option index (0-based)
    Index(usize),
}

impl SelectOption {
    /// Convert SelectOption to JSON format for protocol
    ///
    /// The JSON format matches Playwright's protocol:
    /// - Value: `{"value": "..."}`
    /// - Label: `{"label": "..."}`
    /// - Index: `{"index": 0}`
    pub(crate) fn to_json(&self) -> serde_json::Value {
        match self {
            SelectOption::Value(v) => serde_json::json!({"value": v}),
            SelectOption::Label(l) => serde_json::json!({"label": l}),
            SelectOption::Index(i) => serde_json::json!({"index": i}),
        }
    }
}

// Implement From<&str> for convenience - treats string as value
impl From<&str> for SelectOption {
    fn from(value: &str) -> Self {
        SelectOption::Value(value.to_string())
    }
}

// Implement From<String> for convenience - treats string as value
impl From<String> for SelectOption {
    fn from(value: String) -> Self {
        SelectOption::Value(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_option_value() {
        let opt = SelectOption::Value("test-value".to_string());
        let json = opt.to_json();

        assert_eq!(json["value"], "test-value");
        assert!(json["label"].is_null());
        assert!(json["index"].is_null());
    }

    #[test]
    fn test_select_option_label() {
        let opt = SelectOption::Label("Test Label".to_string());
        let json = opt.to_json();

        assert_eq!(json["label"], "Test Label");
        assert!(json["value"].is_null());
        assert!(json["index"].is_null());
    }

    #[test]
    fn test_select_option_index() {
        let opt = SelectOption::Index(2);
        let json = opt.to_json();

        assert_eq!(json["index"], 2);
        assert!(json["value"].is_null());
        assert!(json["label"].is_null());
    }

    #[test]
    fn test_from_str() {
        let opt: SelectOption = "my-value".into();
        assert_eq!(opt, SelectOption::Value("my-value".to_string()));
    }

    #[test]
    fn test_from_string() {
        let opt: SelectOption = String::from("my-value").into();
        assert_eq!(opt, SelectOption::Value("my-value".to_string()));
    }
}
