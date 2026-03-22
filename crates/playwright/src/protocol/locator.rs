// Locator - Lazy element selector with auto-waiting
//
// Locators are the central piece of Playwright's auto-waiting and retry-ability.
// They represent a way to find element(s) on the page at any given moment.
//
// Key characteristics:
// - Lazy: Don't execute until an action is performed
// - Retryable: Auto-wait for elements to match actionability checks
// - Chainable: Can create sub-locators via first(), last(), nth(), locator()
//
// Architecture:
// - Locator is NOT a ChannelOwner - it's a lightweight wrapper
// - Stores selector string and reference to Frame
// - Delegates all operations to Frame with strict=true
//
// See: https://playwright.dev/docs/api/class-locator

use crate::error::Result;
use crate::protocol::Frame;
use serde::Deserialize;
use std::sync::Arc;

/// The bounding box of an element in pixels.
///
/// All values are measured relative to the top-left corner of the page.
///
/// See: <https://playwright.dev/docs/api/class-locator#locator-bounding-box>
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct BoundingBox {
    /// The x coordinate of the top-left corner of the element in pixels.
    pub x: f64,
    /// The y coordinate of the top-left corner of the element in pixels.
    pub y: f64,
    /// The width of the element in pixels.
    pub width: f64,
    /// The height of the element in pixels.
    pub height: f64,
}

/// Escapes text for use in Playwright's internal selector engine.
///
/// JSON-stringifies the text and appends `i` (case-insensitive) or `s` (strict/exact).
/// Matches the `escapeForTextSelector`/`escapeForAttributeSelector` in Playwright TypeScript.
fn escape_for_selector(text: &str, exact: bool) -> String {
    let suffix = if exact { "s" } else { "i" };
    let escaped = serde_json::to_string(text).unwrap_or_else(|_| format!("\"{}\"", text));
    format!("{}{}", escaped, suffix)
}

/// Builds the internal selector string for `get_by_text`.
///
/// - `exact=false` → `internal:text="text"i` (case-insensitive substring)
/// - `exact=true` → `internal:text="text"s` (case-sensitive exact)
pub(crate) fn get_by_text_selector(text: &str, exact: bool) -> String {
    format!("internal:text={}", escape_for_selector(text, exact))
}

/// Builds the internal selector string for `get_by_label`.
///
/// - `exact=false` → `internal:label="text"i`
/// - `exact=true` → `internal:label="text"s`
pub(crate) fn get_by_label_selector(text: &str, exact: bool) -> String {
    format!("internal:label={}", escape_for_selector(text, exact))
}

/// Builds the internal selector string for `get_by_placeholder`.
///
/// - `exact=false` → `internal:attr=[placeholder="text"i]`
/// - `exact=true` → `internal:attr=[placeholder="text"s]`
pub(crate) fn get_by_placeholder_selector(text: &str, exact: bool) -> String {
    format!(
        "internal:attr=[placeholder={}]",
        escape_for_selector(text, exact)
    )
}

/// Builds the internal selector string for `get_by_alt_text`.
///
/// - `exact=false` → `internal:attr=[alt="text"i]`
/// - `exact=true` → `internal:attr=[alt="text"s]`
pub(crate) fn get_by_alt_text_selector(text: &str, exact: bool) -> String {
    format!("internal:attr=[alt={}]", escape_for_selector(text, exact))
}

/// Builds the internal selector string for `get_by_title`.
///
/// - `exact=false` → `internal:attr=[title="text"i]`
/// - `exact=true` → `internal:attr=[title="text"s]`
pub(crate) fn get_by_title_selector(text: &str, exact: bool) -> String {
    format!("internal:attr=[title={}]", escape_for_selector(text, exact))
}

/// Builds the internal selector string for `get_by_test_id`.
///
/// Uses `data-testid` attribute by default (matching Playwright's default).
/// Always uses exact matching (`s` suffix).
pub(crate) fn get_by_test_id_selector(test_id: &str) -> String {
    format!(
        "internal:testid=[data-testid={}]",
        escape_for_selector(test_id, true)
    )
}

/// Escapes text for use in Playwright's attribute role selector.
///
/// Unlike `escape_for_selector` (which uses JSON encoding), this only escapes
/// backslashes and double quotes, matching Playwright's `escapeForAttributeSelector`.
fn escape_for_attribute_selector(text: &str, exact: bool) -> String {
    let suffix = if exact { "s" } else { "i" };
    let escaped = text.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{}\"{}", escaped, suffix)
}

/// Builds the internal selector string for `get_by_role`.
///
/// Format: `internal:role=<role>[prop1=val1][prop2=val2]...`
///
/// Properties are appended in Playwright's required order:
/// checked, disabled, selected, expanded, include-hidden, level, name, pressed.
pub(crate) fn get_by_role_selector(role: AriaRole, options: Option<GetByRoleOptions>) -> String {
    let mut selector = format!("internal:role={}", role.as_str());

    if let Some(opts) = options {
        if let Some(checked) = opts.checked {
            selector.push_str(&format!("[checked={}]", checked));
        }
        if let Some(disabled) = opts.disabled {
            selector.push_str(&format!("[disabled={}]", disabled));
        }
        if let Some(selected) = opts.selected {
            selector.push_str(&format!("[selected={}]", selected));
        }
        if let Some(expanded) = opts.expanded {
            selector.push_str(&format!("[expanded={}]", expanded));
        }
        if let Some(include_hidden) = opts.include_hidden {
            selector.push_str(&format!("[include-hidden={}]", include_hidden));
        }
        if let Some(level) = opts.level {
            selector.push_str(&format!("[level={}]", level));
        }
        if let Some(name) = &opts.name {
            let exact = opts.exact.unwrap_or(false);
            selector.push_str(&format!(
                "[name={}]",
                escape_for_attribute_selector(name, exact)
            ));
        }
        if let Some(pressed) = opts.pressed {
            selector.push_str(&format!("[pressed={}]", pressed));
        }
    }

    selector
}

/// ARIA roles for `get_by_role()` locator.
///
/// Represents WAI-ARIA roles used to locate elements by their accessibility role.
/// Matches Playwright's `AriaRole` enum across all language bindings.
///
/// See: <https://playwright.dev/docs/api/class-page#page-get-by-role>
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AriaRole {
    Alert,
    Alertdialog,
    Application,
    Article,
    Banner,
    Blockquote,
    Button,
    Caption,
    Cell,
    Checkbox,
    Code,
    Columnheader,
    Combobox,
    Complementary,
    Contentinfo,
    Definition,
    Deletion,
    Dialog,
    Directory,
    Document,
    Emphasis,
    Feed,
    Figure,
    Form,
    Generic,
    Grid,
    Gridcell,
    Group,
    Heading,
    Img,
    Insertion,
    Link,
    List,
    Listbox,
    Listitem,
    Log,
    Main,
    Marquee,
    Math,
    Meter,
    Menu,
    Menubar,
    Menuitem,
    Menuitemcheckbox,
    Menuitemradio,
    Navigation,
    None,
    Note,
    Option,
    Paragraph,
    Presentation,
    Progressbar,
    Radio,
    Radiogroup,
    Region,
    Row,
    Rowgroup,
    Rowheader,
    Scrollbar,
    Search,
    Searchbox,
    Separator,
    Slider,
    Spinbutton,
    Status,
    Strong,
    Subscript,
    Superscript,
    Switch,
    Tab,
    Table,
    Tablist,
    Tabpanel,
    Term,
    Textbox,
    Time,
    Timer,
    Toolbar,
    Tooltip,
    Tree,
    Treegrid,
    Treeitem,
}

impl AriaRole {
    /// Returns the lowercase string representation used in selectors.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Alert => "alert",
            Self::Alertdialog => "alertdialog",
            Self::Application => "application",
            Self::Article => "article",
            Self::Banner => "banner",
            Self::Blockquote => "blockquote",
            Self::Button => "button",
            Self::Caption => "caption",
            Self::Cell => "cell",
            Self::Checkbox => "checkbox",
            Self::Code => "code",
            Self::Columnheader => "columnheader",
            Self::Combobox => "combobox",
            Self::Complementary => "complementary",
            Self::Contentinfo => "contentinfo",
            Self::Definition => "definition",
            Self::Deletion => "deletion",
            Self::Dialog => "dialog",
            Self::Directory => "directory",
            Self::Document => "document",
            Self::Emphasis => "emphasis",
            Self::Feed => "feed",
            Self::Figure => "figure",
            Self::Form => "form",
            Self::Generic => "generic",
            Self::Grid => "grid",
            Self::Gridcell => "gridcell",
            Self::Group => "group",
            Self::Heading => "heading",
            Self::Img => "img",
            Self::Insertion => "insertion",
            Self::Link => "link",
            Self::List => "list",
            Self::Listbox => "listbox",
            Self::Listitem => "listitem",
            Self::Log => "log",
            Self::Main => "main",
            Self::Marquee => "marquee",
            Self::Math => "math",
            Self::Meter => "meter",
            Self::Menu => "menu",
            Self::Menubar => "menubar",
            Self::Menuitem => "menuitem",
            Self::Menuitemcheckbox => "menuitemcheckbox",
            Self::Menuitemradio => "menuitemradio",
            Self::Navigation => "navigation",
            Self::None => "none",
            Self::Note => "note",
            Self::Option => "option",
            Self::Paragraph => "paragraph",
            Self::Presentation => "presentation",
            Self::Progressbar => "progressbar",
            Self::Radio => "radio",
            Self::Radiogroup => "radiogroup",
            Self::Region => "region",
            Self::Row => "row",
            Self::Rowgroup => "rowgroup",
            Self::Rowheader => "rowheader",
            Self::Scrollbar => "scrollbar",
            Self::Search => "search",
            Self::Searchbox => "searchbox",
            Self::Separator => "separator",
            Self::Slider => "slider",
            Self::Spinbutton => "spinbutton",
            Self::Status => "status",
            Self::Strong => "strong",
            Self::Subscript => "subscript",
            Self::Superscript => "superscript",
            Self::Switch => "switch",
            Self::Tab => "tab",
            Self::Table => "table",
            Self::Tablist => "tablist",
            Self::Tabpanel => "tabpanel",
            Self::Term => "term",
            Self::Textbox => "textbox",
            Self::Time => "time",
            Self::Timer => "timer",
            Self::Toolbar => "toolbar",
            Self::Tooltip => "tooltip",
            Self::Tree => "tree",
            Self::Treegrid => "treegrid",
            Self::Treeitem => "treeitem",
        }
    }
}

/// Options for `get_by_role()` locator.
///
/// All fields are optional. When not specified, the property is not included
/// in the role selector, meaning it matches any value.
///
/// See: <https://playwright.dev/docs/api/class-page#page-get-by-role>
#[derive(Debug, Clone, Default)]
pub struct GetByRoleOptions {
    /// Whether the element is checked (for checkboxes, radio buttons).
    pub checked: Option<bool>,
    /// Whether the element is disabled.
    pub disabled: Option<bool>,
    /// Whether the element is selected (for options).
    pub selected: Option<bool>,
    /// Whether the element is expanded (for tree items, comboboxes).
    pub expanded: Option<bool>,
    /// Whether to include hidden elements.
    pub include_hidden: Option<bool>,
    /// The heading level (1-6, for heading role).
    pub level: Option<u32>,
    /// The accessible name of the element.
    pub name: Option<String>,
    /// Whether `name` matching is exact (case-sensitive, full-string).
    /// Default is false (case-insensitive substring).
    pub exact: Option<bool>,
    /// Whether the element is pressed (for toggle buttons).
    pub pressed: Option<bool>,
}

/// Options for [`Locator::filter()`].
///
/// Narrows an existing locator according to the specified criteria.
/// All fields are optional; unset fields are ignored.
///
/// See: <https://playwright.dev/docs/api/class-locator#locator-filter>
#[derive(Debug, Clone, Default)]
pub struct FilterOptions {
    /// Matches elements containing the specified text (case-insensitive substring by default).
    pub has_text: Option<String>,
    /// Matches elements that do **not** contain the specified text anywhere inside.
    pub has_not_text: Option<String>,
    /// Narrows to elements that contain a descendant matching this locator.
    ///
    /// The inner locator is queried relative to the outer locator's matched element,
    /// not the document root.
    pub has: Option<Locator>,
    /// Narrows to elements that do **not** contain a descendant matching this locator.
    pub has_not: Option<Locator>,
}

/// Locator represents a way to find element(s) on the page at any given moment.
///
/// Locators are lazy - they don't execute queries until an action is performed.
/// This enables auto-waiting and retry-ability for robust test automation.
///
/// # Examples
///
/// ```ignore
/// use playwright_rs::protocol::{Playwright, SelectOption};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let playwright = Playwright::launch().await?;
///     let browser = playwright.chromium().launch().await?;
///     let page = browser.new_page().await?;
///
///     // Demonstrate set_checked() - checkbox interaction
///     let _ = page.goto(
///         "data:text/html,<input type='checkbox' id='cb'>",
///         None
///     ).await;
///     let checkbox = page.locator("#cb").await;
///     checkbox.set_checked(true, None).await?;
///     assert!(checkbox.is_checked().await?);
///     checkbox.set_checked(false, None).await?;
///     assert!(!checkbox.is_checked().await?);
///
///     // Demonstrate select_option() - select by value, label, and index
///     let _ = page.goto(
///         "data:text/html,<select id='fruits'>\
///             <option value='apple'>Apple</option>\
///             <option value='banana'>Banana</option>\
///             <option value='cherry'>Cherry</option>\
///         </select>",
///         None
///     ).await;
///     let select = page.locator("#fruits").await;
///     select.select_option("banana", None).await?;
///     assert_eq!(select.input_value(None).await?, "banana");
///     select.select_option(SelectOption::Label("Apple".to_string()), None).await?;
///     assert_eq!(select.input_value(None).await?, "apple");
///     select.select_option(SelectOption::Index(2), None).await?;
///     assert_eq!(select.input_value(None).await?, "cherry");
///
///     // Demonstrate select_option_multiple() - multi-select
///     let _ = page.goto(
///         "data:text/html,<select id='colors' multiple>\
///             <option value='red'>Red</option>\
///             <option value='green'>Green</option>\
///             <option value='blue'>Blue</option>\
///             <option value='yellow'>Yellow</option>\
///         </select>",
///         None
///     ).await;
///     let multi = page.locator("#colors").await;
///     let selected = multi.select_option_multiple(&["red", "blue"], None).await?;
///     assert_eq!(selected.len(), 2);
///     assert!(selected.contains(&"red".to_string()));
///     assert!(selected.contains(&"blue".to_string()));
///
///     // Demonstrate get_by_text() - find elements by text content
///     let _ = page.goto(
///         "data:text/html,<button>Submit</button><button>Submit Order</button>",
///         None
///     ).await;
///     let all_submits = page.get_by_text("Submit", false).await;
///     assert_eq!(all_submits.count().await?, 2); // case-insensitive substring
///     let exact_submit = page.get_by_text("Submit", true).await;
///     assert_eq!(exact_submit.count().await?, 1); // exact match only
///
///     // Demonstrate get_by_label, get_by_placeholder, get_by_test_id
///     let _ = page.goto(
///         "data:text/html,<label for='email'>Email</label>\
///             <input id='email' placeholder='you@example.com' data-testid='email-input' />",
///         None
///     ).await;
///     let by_label = page.get_by_label("Email", false).await;
///     assert_eq!(by_label.count().await?, 1);
///     let by_placeholder = page.get_by_placeholder("you@example.com", true).await;
///     assert_eq!(by_placeholder.count().await?, 1);
///     let by_test_id = page.get_by_test_id("email-input").await;
///     assert_eq!(by_test_id.count().await?, 1);
///
///     // Demonstrate screenshot() - element screenshot
///     let _ = page.goto(
///         "data:text/html,<h1 id='title'>Hello World</h1>",
///         None
///     ).await;
///     let heading = page.locator("#title").await;
///     let screenshot = heading.screenshot(None).await?;
///     assert!(!screenshot.is_empty());
///
///     browser.close().await?;
///     Ok(())
/// }
/// ```
///
/// See: <https://playwright.dev/docs/api/class-locator>
#[derive(Clone)]
pub struct Locator {
    frame: Arc<Frame>,
    selector: String,
}

impl Locator {
    /// Creates a new Locator (internal use only)
    ///
    /// Use `page.locator()` or `frame.locator()` to create locators in application code.
    pub(crate) fn new(frame: Arc<Frame>, selector: String) -> Self {
        Self { frame, selector }
    }

    /// Returns the selector string for this locator
    pub fn selector(&self) -> &str {
        &self.selector
    }

    /// Evaluate a JavaScript expression in the frame context.
    ///
    /// Used internally for injecting CSS (e.g., disabling animations) before screenshot assertions.
    pub(crate) async fn evaluate_js<T: serde::Serialize>(
        &self,
        expression: &str,
        _arg: Option<T>,
    ) -> Result<()> {
        self.frame
            .frame_evaluate_expression(expression)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))
    }

    /// Creates a locator for the first matching element.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-first>
    pub fn first(&self) -> Locator {
        Locator::new(
            Arc::clone(&self.frame),
            format!("{} >> nth=0", self.selector),
        )
    }

    /// Creates a locator for the last matching element.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-last>
    pub fn last(&self) -> Locator {
        Locator::new(
            Arc::clone(&self.frame),
            format!("{} >> nth=-1", self.selector),
        )
    }

    /// Creates a locator for the nth matching element (0-indexed).
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-nth>
    pub fn nth(&self, index: i32) -> Locator {
        Locator::new(
            Arc::clone(&self.frame),
            format!("{} >> nth={}", self.selector, index),
        )
    }

    /// Returns a locator that matches elements containing the given text.
    ///
    /// By default, matching is case-insensitive and searches for a substring.
    /// Set `exact` to `true` for case-sensitive exact matching.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-get-by-text>
    pub fn get_by_text(&self, text: &str, exact: bool) -> Locator {
        self.locator(&get_by_text_selector(text, exact))
    }

    /// Returns a locator that matches elements by their associated label text.
    ///
    /// Targets form controls (`input`, `textarea`, `select`) linked via `<label>`,
    /// `aria-label`, or `aria-labelledby`.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-get-by-label>
    pub fn get_by_label(&self, text: &str, exact: bool) -> Locator {
        self.locator(&get_by_label_selector(text, exact))
    }

    /// Returns a locator that matches elements by their placeholder text.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-get-by-placeholder>
    pub fn get_by_placeholder(&self, text: &str, exact: bool) -> Locator {
        self.locator(&get_by_placeholder_selector(text, exact))
    }

    /// Returns a locator that matches elements by their alt text.
    ///
    /// Typically used for `<img>` elements.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-get-by-alt-text>
    pub fn get_by_alt_text(&self, text: &str, exact: bool) -> Locator {
        self.locator(&get_by_alt_text_selector(text, exact))
    }

    /// Returns a locator that matches elements by their title attribute.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-get-by-title>
    pub fn get_by_title(&self, text: &str, exact: bool) -> Locator {
        self.locator(&get_by_title_selector(text, exact))
    }

    /// Returns a locator that matches elements by their `data-testid` attribute.
    ///
    /// Always uses exact matching (case-sensitive).
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-get-by-test-id>
    pub fn get_by_test_id(&self, test_id: &str) -> Locator {
        self.locator(&get_by_test_id_selector(test_id))
    }

    /// Returns a locator that matches elements by their ARIA role.
    ///
    /// This is the recommended way to locate elements, as it matches the way
    /// users and assistive technology perceive the page.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-get-by-role>
    pub fn get_by_role(&self, role: AriaRole, options: Option<GetByRoleOptions>) -> Locator {
        self.locator(&get_by_role_selector(role, options))
    }

    /// Creates a sub-locator within this locator's subtree.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-locator>
    pub fn locator(&self, selector: &str) -> Locator {
        Locator::new(
            Arc::clone(&self.frame),
            format!("{} >> {}", self.selector, selector),
        )
    }

    /// Narrows this locator according to the filter options.
    ///
    /// Can be chained to apply multiple filters in sequence.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use playwright_rs::{Playwright, FilterOptions};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let playwright = Playwright::launch().await?;
    /// let browser = playwright.chromium().launch().await?;
    /// let page = browser.new_page().await?;
    ///
    /// // Filter rows to those containing "Apple"
    /// let rows = page.locator("tr").await;
    /// let apple_row = rows.filter(FilterOptions {
    ///     has_text: Some("Apple".to_string()),
    ///     ..Default::default()
    /// });
    /// # browser.close().await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-filter>
    pub fn filter(&self, options: FilterOptions) -> Locator {
        let mut selector = self.selector.clone();

        if let Some(text) = &options.has_text {
            let escaped = escape_for_selector(text, false);
            selector = format!("{} >> internal:has-text={}", selector, escaped);
        }

        if let Some(text) = &options.has_not_text {
            let escaped = escape_for_selector(text, false);
            selector = format!("{} >> internal:has-not-text={}", selector, escaped);
        }

        if let Some(locator) = &options.has {
            let inner = serde_json::to_string(&locator.selector)
                .unwrap_or_else(|_| format!("\"{}\"", locator.selector));
            selector = format!("{} >> internal:has={}", selector, inner);
        }

        if let Some(locator) = &options.has_not {
            let inner = serde_json::to_string(&locator.selector)
                .unwrap_or_else(|_| format!("\"{}\"", locator.selector));
            selector = format!("{} >> internal:has-not={}", selector, inner);
        }

        Locator::new(Arc::clone(&self.frame), selector)
    }

    /// Creates a locator matching elements that satisfy **both** this locator and `locator`.
    ///
    /// Note: named `and_` because `and` is a Rust keyword.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use playwright_rs::Playwright;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let playwright = Playwright::launch().await?;
    /// let browser = playwright.chromium().launch().await?;
    /// let page = browser.new_page().await?;
    ///
    /// // Find a button that also has a specific title
    /// let button = page.locator("button").await;
    /// let titled = page.locator("[title='Subscribe']").await;
    /// let subscribe_btn = button.and_(&titled);
    /// # browser.close().await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-and>
    pub fn and_(&self, locator: &Locator) -> Locator {
        let inner = serde_json::to_string(&locator.selector)
            .unwrap_or_else(|_| format!("\"{}\"", locator.selector));
        Locator::new(
            Arc::clone(&self.frame),
            format!("{} >> internal:and={}", self.selector, inner),
        )
    }

    /// Creates a locator matching elements that satisfy **either** this locator or `locator`.
    ///
    /// Note: named `or_` because `or` is a Rust keyword.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use playwright_rs::Playwright;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let playwright = Playwright::launch().await?;
    /// let browser = playwright.chromium().launch().await?;
    /// let page = browser.new_page().await?;
    ///
    /// // Find any element that is either a button or a link
    /// let buttons = page.locator("button").await;
    /// let links = page.locator("a").await;
    /// let interactive = buttons.or_(&links);
    /// # browser.close().await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-or>
    pub fn or_(&self, locator: &Locator) -> Locator {
        let inner = serde_json::to_string(&locator.selector)
            .unwrap_or_else(|_| format!("\"{}\"", locator.selector));
        Locator::new(
            Arc::clone(&self.frame),
            format!("{} >> internal:or={}", self.selector, inner),
        )
    }

    /// Returns the number of elements matching this locator.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-count>
    pub async fn count(&self) -> Result<usize> {
        self.frame
            .locator_count(&self.selector)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))
    }

    /// Returns an array of locators, one for each matching element.
    ///
    /// Note: `all()` does not wait for elements to match the locator,
    /// and instead immediately returns whatever is in the DOM.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-all>
    pub async fn all(&self) -> Result<Vec<Locator>> {
        let count = self.count().await?;
        Ok((0..count).map(|i| self.nth(i as i32)).collect())
    }

    /// Returns the text content of the element.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-text-content>
    pub async fn text_content(&self) -> Result<Option<String>> {
        self.frame
            .locator_text_content(&self.selector)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))
    }

    /// Returns the inner text of the element (visible text).
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-inner-text>
    pub async fn inner_text(&self) -> Result<String> {
        self.frame
            .locator_inner_text(&self.selector)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))
    }

    /// Returns the inner HTML of the element.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-inner-html>
    pub async fn inner_html(&self) -> Result<String> {
        self.frame
            .locator_inner_html(&self.selector)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))
    }

    /// Returns the value of the specified attribute.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-get-attribute>
    pub async fn get_attribute(&self, name: &str) -> Result<Option<String>> {
        self.frame
            .locator_get_attribute(&self.selector, name)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))
    }

    /// Returns whether the element is visible.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-is-visible>
    pub async fn is_visible(&self) -> Result<bool> {
        self.frame
            .locator_is_visible(&self.selector)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))
    }

    /// Returns whether the element is enabled.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-is-enabled>
    pub async fn is_enabled(&self) -> Result<bool> {
        self.frame
            .locator_is_enabled(&self.selector)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))
    }

    /// Returns whether the checkbox or radio button is checked.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-is-checked>
    pub async fn is_checked(&self) -> Result<bool> {
        self.frame
            .locator_is_checked(&self.selector)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))
    }

    /// Returns whether the element is editable.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-is-editable>
    pub async fn is_editable(&self) -> Result<bool> {
        self.frame
            .locator_is_editable(&self.selector)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))
    }

    /// Returns whether the element is hidden.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-is-hidden>
    pub async fn is_hidden(&self) -> Result<bool> {
        self.frame
            .locator_is_hidden(&self.selector)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))
    }

    /// Returns whether the element is disabled.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-is-disabled>
    pub async fn is_disabled(&self) -> Result<bool> {
        self.frame
            .locator_is_disabled(&self.selector)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))
    }

    /// Returns whether the element is focused (currently has focus).
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-is-focused>
    pub async fn is_focused(&self) -> Result<bool> {
        self.frame
            .locator_is_focused(&self.selector)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))
    }

    // Action methods

    /// Clicks the element.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-click>
    pub async fn click(&self, options: Option<crate::protocol::ClickOptions>) -> Result<()> {
        self.frame
            .locator_click(&self.selector, options)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))
    }

    /// Wraps an error with selector context for better error messages.
    fn wrap_error_with_selector(&self, error: crate::error::Error) -> crate::error::Error {
        match &error {
            crate::error::Error::ProtocolError(msg) => {
                // Add selector context to protocol errors (timeouts, etc.)
                crate::error::Error::ProtocolError(format!("{} [selector: {}]", msg, self.selector))
            }
            crate::error::Error::Timeout(msg) => {
                crate::error::Error::Timeout(format!("{} [selector: {}]", msg, self.selector))
            }
            _ => error, // Other errors pass through unchanged
        }
    }

    /// Double clicks the element.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-dblclick>
    pub async fn dblclick(&self, options: Option<crate::protocol::ClickOptions>) -> Result<()> {
        self.frame
            .locator_dblclick(&self.selector, options)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))
    }

    /// Fills the element with text.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-fill>
    pub async fn fill(
        &self,
        text: &str,
        options: Option<crate::protocol::FillOptions>,
    ) -> Result<()> {
        self.frame
            .locator_fill(&self.selector, text, options)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))
    }

    /// Clears the element's value.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-clear>
    pub async fn clear(&self, options: Option<crate::protocol::FillOptions>) -> Result<()> {
        self.frame
            .locator_clear(&self.selector, options)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))
    }

    /// Presses a key on the element.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-press>
    pub async fn press(
        &self,
        key: &str,
        options: Option<crate::protocol::PressOptions>,
    ) -> Result<()> {
        self.frame
            .locator_press(&self.selector, key, options)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))
    }

    /// Sets focus on the element.
    ///
    /// Calls the element's `focus()` method. Used to move keyboard focus to a
    /// specific element for subsequent keyboard interactions.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-focus>
    pub async fn focus(&self) -> Result<()> {
        self.frame
            .locator_focus(&self.selector)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))
    }

    /// Removes focus from the element.
    ///
    /// Calls the element's `blur()` method. Moves keyboard focus away from the element.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-blur>
    pub async fn blur(&self) -> Result<()> {
        self.frame
            .locator_blur(&self.selector)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))
    }

    /// Types `text` into the element character by character, as though it was typed
    /// on a real keyboard.
    ///
    /// Use this method when you need to simulate keystrokes with individual key events
    /// (e.g., for autocomplete widgets). For simply setting a field value, prefer
    /// [`Locator::fill()`].
    ///
    /// # Arguments
    ///
    /// * `text` - Text to type into the element
    /// * `options` - Optional [`PressSequentiallyOptions`] (e.g., `delay` between key presses)
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-press-sequentially>
    pub async fn press_sequentially(
        &self,
        text: &str,
        options: Option<crate::protocol::PressSequentiallyOptions>,
    ) -> Result<()> {
        self.frame
            .locator_press_sequentially(&self.selector, text, options)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))
    }

    /// Returns the `innerText` values of all elements matching this locator.
    ///
    /// Unlike [`Locator::inner_text()`] (which uses strict mode and requires exactly one match),
    /// `all_inner_texts()` returns text from all matching elements.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-all-inner-texts>
    pub async fn all_inner_texts(&self) -> Result<Vec<String>> {
        self.frame
            .locator_all_inner_texts(&self.selector)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))
    }

    /// Returns the `textContent` values of all elements matching this locator.
    ///
    /// Unlike [`Locator::text_content()`] (which uses strict mode and requires exactly one match),
    /// `all_text_contents()` returns text from all matching elements.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-all-text-contents>
    pub async fn all_text_contents(&self) -> Result<Vec<String>> {
        self.frame
            .locator_all_text_contents(&self.selector)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))
    }

    /// Ensures the checkbox or radio button is checked.
    ///
    /// This method is idempotent - if already checked, does nothing.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-check>
    pub async fn check(&self, options: Option<crate::protocol::CheckOptions>) -> Result<()> {
        self.frame
            .locator_check(&self.selector, options)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))
    }

    /// Ensures the checkbox is unchecked.
    ///
    /// This method is idempotent - if already unchecked, does nothing.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-uncheck>
    pub async fn uncheck(&self, options: Option<crate::protocol::CheckOptions>) -> Result<()> {
        self.frame
            .locator_uncheck(&self.selector, options)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))
    }

    /// Sets the checkbox or radio button to the specified checked state.
    ///
    /// This is a convenience method that calls `check()` if `checked` is true,
    /// or `uncheck()` if `checked` is false.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-set-checked>
    pub async fn set_checked(
        &self,
        checked: bool,
        options: Option<crate::protocol::CheckOptions>,
    ) -> Result<()> {
        if checked {
            self.check(options).await
        } else {
            self.uncheck(options).await
        }
    }

    /// Hovers the mouse over the element.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-hover>
    pub async fn hover(&self, options: Option<crate::protocol::HoverOptions>) -> Result<()> {
        self.frame
            .locator_hover(&self.selector, options)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))
    }

    /// Returns the value of the input, textarea, or select element.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-input-value>
    pub async fn input_value(&self, _options: Option<()>) -> Result<String> {
        self.frame
            .locator_input_value(&self.selector)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))
    }

    /// Selects one or more options in a select element.
    ///
    /// Returns an array of option values that have been successfully selected.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-select-option>
    pub async fn select_option(
        &self,
        value: impl Into<crate::protocol::SelectOption>,
        options: Option<crate::protocol::SelectOptions>,
    ) -> Result<Vec<String>> {
        self.frame
            .locator_select_option(&self.selector, value.into(), options)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))
    }

    /// Selects multiple options in a select element.
    ///
    /// Returns an array of option values that have been successfully selected.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-select-option>
    pub async fn select_option_multiple(
        &self,
        values: &[impl Into<crate::protocol::SelectOption> + Clone],
        options: Option<crate::protocol::SelectOptions>,
    ) -> Result<Vec<String>> {
        let select_options: Vec<crate::protocol::SelectOption> =
            values.iter().map(|v| v.clone().into()).collect();
        self.frame
            .locator_select_option_multiple(&self.selector, select_options, options)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))
    }

    /// Sets the file path(s) to upload to a file input element.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-set-input-files>
    pub async fn set_input_files(
        &self,
        file: &std::path::PathBuf,
        _options: Option<()>,
    ) -> Result<()> {
        self.frame
            .locator_set_input_files(&self.selector, file)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))
    }

    /// Sets multiple file paths to upload to a file input element.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-set-input-files>
    pub async fn set_input_files_multiple(
        &self,
        files: &[&std::path::PathBuf],
        _options: Option<()>,
    ) -> Result<()> {
        self.frame
            .locator_set_input_files_multiple(&self.selector, files)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))
    }

    /// Sets a file to upload using FilePayload (explicit name, mimeType, buffer).
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-set-input-files>
    pub async fn set_input_files_payload(
        &self,
        file: crate::protocol::FilePayload,
        _options: Option<()>,
    ) -> Result<()> {
        self.frame
            .locator_set_input_files_payload(&self.selector, file)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))
    }

    /// Sets multiple files to upload using FilePayload.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-set-input-files>
    pub async fn set_input_files_payload_multiple(
        &self,
        files: &[crate::protocol::FilePayload],
        _options: Option<()>,
    ) -> Result<()> {
        self.frame
            .locator_set_input_files_payload_multiple(&self.selector, files)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))
    }

    /// Dispatches a DOM event on the element.
    ///
    /// Unlike clicking or typing, `dispatch_event` directly sends the event without
    /// performing any actionability checks. It still waits for the element to be present
    /// in the DOM.
    ///
    /// # Arguments
    ///
    /// * `type_` - The event type to dispatch, e.g. `"click"`, `"focus"`, `"myevent"`.
    /// * `event_init` - Optional event initializer properties (e.g. `{"detail": "value"}` for
    ///   `CustomEvent`). Corresponds to the second argument of `new Event(type, init)`.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The element is not found within the timeout
    /// - The protocol call fails
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-dispatch-event>
    pub async fn dispatch_event(
        &self,
        type_: &str,
        event_init: Option<serde_json::Value>,
    ) -> Result<()> {
        self.frame
            .locator_dispatch_event(&self.selector, type_, event_init)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))
    }

    /// Returns the bounding box of the element, or `None` if the element is not visible.
    ///
    /// The bounding box is in pixels, relative to the top-left corner of the page.
    /// Returns `None` when the element has `display: none` or is otherwise not part of
    /// the layout.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The element is not found within the timeout
    /// - The protocol call fails
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-bounding-box>
    pub async fn bounding_box(&self) -> Result<Option<BoundingBox>> {
        self.frame
            .locator_bounding_box(&self.selector)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))
    }

    /// Scrolls the element into view if it is not already visible in the viewport.
    ///
    /// This is an alias for calling `element.scrollIntoView()` in the browser.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The element is not found within the timeout
    /// - The protocol call fails
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-scroll-into-view-if-needed>
    pub async fn scroll_into_view_if_needed(&self) -> Result<()> {
        self.frame
            .locator_scroll_into_view_if_needed(&self.selector)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))
    }

    /// Takes a screenshot of the element and returns the image bytes.
    ///
    /// This method uses strict mode - it will fail if the selector matches multiple elements.
    /// Use `first()`, `last()`, or `nth()` to refine the selector to a single element.
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-screenshot>
    pub async fn screenshot(
        &self,
        options: Option<crate::protocol::ScreenshotOptions>,
    ) -> Result<Vec<u8>> {
        // Query for the element using strict mode (should return exactly one)
        let element = self
            .frame
            .query_selector(&self.selector)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))?
            .ok_or_else(|| {
                crate::error::Error::ElementNotFound(format!(
                    "Element not found: {}",
                    self.selector
                ))
            })?;

        // Delegate to ElementHandle.screenshot()
        element
            .screenshot(options)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))
    }

    /// Performs a touch-tap on the element.
    ///
    /// This method dispatches a `touchstart` and `touchend` event on the element.
    /// For touch support to work, the browser context must be created with
    /// `has_touch: true`.
    ///
    /// # Arguments
    ///
    /// * `options` - Optional [`TapOptions`] (force, modifiers, position, timeout, trial)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The element is not found within the timeout
    /// - Actionability checks fail (unless `force: true`)
    /// - The browser context was not created with `has_touch: true`
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-tap>
    pub async fn tap(&self, options: Option<crate::protocol::TapOptions>) -> Result<()> {
        self.frame
            .locator_tap(&self.selector, options)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))
    }

    /// Evaluates a JavaScript expression in the scope of the matched element.
    ///
    /// The element is passed as the first argument to the expression. The expression
    /// can be any JavaScript function or expression that returns a JSON-serializable value.
    ///
    /// # Arguments
    ///
    /// * `expression` - JavaScript expression or function, e.g. `"(el) => el.textContent"`
    /// * `arg` - Optional argument passed as the second argument to the function
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The element is not found within the timeout
    /// - The JavaScript expression throws an error
    /// - The return value is not JSON-serializable
    ///
    /// # Example
    ///
    /// ```ignore
    /// use playwright_rs::Playwright;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let playwright = Playwright::launch().await?;
    /// let browser = playwright.chromium().launch().await?;
    /// let page = browser.new_page().await?;
    /// let _ = page.goto("data:text/html,<h1>Hello</h1>", None).await;
    ///
    /// let heading = page.locator("h1").await;
    /// let text: String = heading.evaluate("(el) => el.textContent", None::<()>).await?;
    /// assert_eq!(text, "Hello");
    ///
    /// // With an argument
    /// let result: String = heading
    ///     .evaluate("(el, suffix) => el.textContent + suffix", Some("!"))
    ///     .await?;
    /// assert_eq!(result, "Hello!");
    /// # browser.close().await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-evaluate>
    pub async fn evaluate<R, T>(&self, expression: &str, arg: Option<T>) -> Result<R>
    where
        R: serde::de::DeserializeOwned,
        T: serde::Serialize,
    {
        let raw = self
            .frame
            .locator_evaluate(&self.selector, expression, arg)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))?;
        serde_json::from_value(raw).map_err(|e| {
            crate::error::Error::ProtocolError(format!(
                "evaluate result deserialization failed: {}",
                e
            ))
        })
    }

    /// Evaluates a JavaScript expression in the scope of all elements matching this locator.
    ///
    /// The array of all matched elements is passed as the first argument to the expression.
    /// Unlike [`evaluate()`](Self::evaluate), this does not use strict mode — all matching
    /// elements are collected and passed as an array.
    ///
    /// # Arguments
    ///
    /// * `expression` - JavaScript function that receives an array of elements
    /// * `arg` - Optional argument passed as the second argument to the function
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The JavaScript expression throws an error
    /// - The return value is not JSON-serializable
    ///
    /// # Example
    ///
    /// ```ignore
    /// use playwright_rs::Playwright;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let playwright = Playwright::launch().await?;
    /// let browser = playwright.chromium().launch().await?;
    /// let page = browser.new_page().await?;
    /// let _ = page.goto(
    ///     "data:text/html,<li class='item'>A</li><li class='item'>B</li>",
    ///     None
    /// ).await;
    ///
    /// let items = page.locator(".item").await;
    /// let texts: Vec<String> = items
    ///     .evaluate_all("(elements) => elements.map(e => e.textContent)", None::<()>)
    ///     .await?;
    /// assert_eq!(texts, vec!["A", "B"]);
    /// # browser.close().await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// See: <https://playwright.dev/docs/api/class-locator#locator-evaluate-all>
    pub async fn evaluate_all<R, T>(&self, expression: &str, arg: Option<T>) -> Result<R>
    where
        R: serde::de::DeserializeOwned,
        T: serde::Serialize,
    {
        let raw = self
            .frame
            .locator_evaluate_all(&self.selector, expression, arg)
            .await
            .map_err(|e| self.wrap_error_with_selector(e))?;
        serde_json::from_value(raw).map_err(|e| {
            crate::error::Error::ProtocolError(format!(
                "evaluate_all result deserialization failed: {}",
                e
            ))
        })
    }
}

impl std::fmt::Debug for Locator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Locator")
            .field("selector", &self.selector)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_for_selector_case_insensitive() {
        assert_eq!(escape_for_selector("hello", false), "\"hello\"i");
    }

    #[test]
    fn test_escape_for_selector_exact() {
        assert_eq!(escape_for_selector("hello", true), "\"hello\"s");
    }

    #[test]
    fn test_escape_for_selector_with_quotes() {
        assert_eq!(
            escape_for_selector("say \"hi\"", false),
            "\"say \\\"hi\\\"\"i"
        );
    }

    #[test]
    fn test_get_by_text_selector_case_insensitive() {
        assert_eq!(
            get_by_text_selector("Click me", false),
            "internal:text=\"Click me\"i"
        );
    }

    #[test]
    fn test_get_by_text_selector_exact() {
        assert_eq!(
            get_by_text_selector("Click me", true),
            "internal:text=\"Click me\"s"
        );
    }

    #[test]
    fn test_get_by_label_selector() {
        assert_eq!(
            get_by_label_selector("Email", false),
            "internal:label=\"Email\"i"
        );
    }

    #[test]
    fn test_get_by_placeholder_selector() {
        assert_eq!(
            get_by_placeholder_selector("Enter name", false),
            "internal:attr=[placeholder=\"Enter name\"i]"
        );
    }

    #[test]
    fn test_get_by_alt_text_selector() {
        assert_eq!(
            get_by_alt_text_selector("Logo", true),
            "internal:attr=[alt=\"Logo\"s]"
        );
    }

    #[test]
    fn test_get_by_title_selector() {
        assert_eq!(
            get_by_title_selector("Help", false),
            "internal:attr=[title=\"Help\"i]"
        );
    }

    #[test]
    fn test_get_by_test_id_selector() {
        assert_eq!(
            get_by_test_id_selector("submit-btn"),
            "internal:testid=[data-testid=\"submit-btn\"s]"
        );
    }

    #[test]
    fn test_escape_for_attribute_selector_case_insensitive() {
        assert_eq!(
            escape_for_attribute_selector("Submit", false),
            "\"Submit\"i"
        );
    }

    #[test]
    fn test_escape_for_attribute_selector_exact() {
        assert_eq!(escape_for_attribute_selector("Submit", true), "\"Submit\"s");
    }

    #[test]
    fn test_escape_for_attribute_selector_escapes_quotes() {
        assert_eq!(
            escape_for_attribute_selector("Say \"hello\"", false),
            "\"Say \\\"hello\\\"\"i"
        );
    }

    #[test]
    fn test_escape_for_attribute_selector_escapes_backslashes() {
        assert_eq!(
            escape_for_attribute_selector("path\\to", true),
            "\"path\\\\to\"s"
        );
    }

    #[test]
    fn test_get_by_role_selector_role_only() {
        assert_eq!(
            get_by_role_selector(AriaRole::Button, None),
            "internal:role=button"
        );
    }

    #[test]
    fn test_get_by_role_selector_with_name() {
        let opts = GetByRoleOptions {
            name: Some("Submit".to_string()),
            ..Default::default()
        };
        assert_eq!(
            get_by_role_selector(AriaRole::Button, Some(opts)),
            "internal:role=button[name=\"Submit\"i]"
        );
    }

    #[test]
    fn test_get_by_role_selector_with_name_exact() {
        let opts = GetByRoleOptions {
            name: Some("Submit".to_string()),
            exact: Some(true),
            ..Default::default()
        };
        assert_eq!(
            get_by_role_selector(AriaRole::Button, Some(opts)),
            "internal:role=button[name=\"Submit\"s]"
        );
    }

    #[test]
    fn test_get_by_role_selector_with_checked() {
        let opts = GetByRoleOptions {
            checked: Some(true),
            ..Default::default()
        };
        assert_eq!(
            get_by_role_selector(AriaRole::Checkbox, Some(opts)),
            "internal:role=checkbox[checked=true]"
        );
    }

    #[test]
    fn test_get_by_role_selector_with_level() {
        let opts = GetByRoleOptions {
            level: Some(2),
            ..Default::default()
        };
        assert_eq!(
            get_by_role_selector(AriaRole::Heading, Some(opts)),
            "internal:role=heading[level=2]"
        );
    }

    #[test]
    fn test_get_by_role_selector_with_disabled() {
        let opts = GetByRoleOptions {
            disabled: Some(true),
            ..Default::default()
        };
        assert_eq!(
            get_by_role_selector(AriaRole::Button, Some(opts)),
            "internal:role=button[disabled=true]"
        );
    }

    #[test]
    fn test_get_by_role_selector_include_hidden() {
        let opts = GetByRoleOptions {
            include_hidden: Some(true),
            ..Default::default()
        };
        assert_eq!(
            get_by_role_selector(AriaRole::Button, Some(opts)),
            "internal:role=button[include-hidden=true]"
        );
    }

    #[test]
    fn test_get_by_role_selector_property_order() {
        // All properties: checked, disabled, selected, expanded, include-hidden, level, name, pressed
        let opts = GetByRoleOptions {
            pressed: Some(true),
            name: Some("OK".to_string()),
            checked: Some(false),
            disabled: Some(true),
            ..Default::default()
        };
        assert_eq!(
            get_by_role_selector(AriaRole::Button, Some(opts)),
            "internal:role=button[checked=false][disabled=true][name=\"OK\"i][pressed=true]"
        );
    }

    #[test]
    fn test_get_by_role_selector_name_with_special_chars() {
        let opts = GetByRoleOptions {
            name: Some("Click \"here\" now".to_string()),
            exact: Some(true),
            ..Default::default()
        };
        assert_eq!(
            get_by_role_selector(AriaRole::Link, Some(opts)),
            "internal:role=link[name=\"Click \\\"here\\\" now\"s]"
        );
    }

    #[test]
    fn test_aria_role_as_str() {
        assert_eq!(AriaRole::Button.as_str(), "button");
        assert_eq!(AriaRole::Heading.as_str(), "heading");
        assert_eq!(AriaRole::Link.as_str(), "link");
        assert_eq!(AriaRole::Checkbox.as_str(), "checkbox");
        assert_eq!(AriaRole::Alert.as_str(), "alert");
        assert_eq!(AriaRole::Navigation.as_str(), "navigation");
        assert_eq!(AriaRole::Progressbar.as_str(), "progressbar");
        assert_eq!(AriaRole::Treeitem.as_str(), "treeitem");
    }
}
