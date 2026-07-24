//! Opt-in test fakes for browser APIs Playwright cannot drive natively.
//!
//! The File System Access API (`window.showSaveFilePicker` /
//! `showOpenFilePicker`) opens native OS dialogs with no DOM presence, so no
//! locator or dialog handler can reach them. The standard cross-binding
//! pattern is to install a deterministic fake before the app's JS runs;
//! [`FakeFileSystem`] packages that pattern so consumers don't hand-roll it.
//!
//! Nothing here is installed unless a test asks for it: a page that never
//! calls [`Page::fake_file_system`](crate::protocol::Page::fake_file_system)
//! keeps the browser's real (or absent) picker functions, so
//! feature-detection and fallback paths stay testable.
//!
//! # Example
//!
//! ```no_run
//! # use playwright_rs::Playwright;
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! # let pw = Playwright::launch().await?;
//! # let page = pw.chromium().launch().await?.new_page().await?;
//! let fs = page.fake_file_system().await?;
//!
//! // Seed a file the app's Open dialog will "pick":
//! fs.set_open_file("plan.json", br#"{"rooms": []}"#).await?;
//!
//! // ... drive the app's Save As flow, then assert what it wrote:
//! page.goto("https://localhost:8080", None).await?;
//! let saved = fs.last_saved_bytes().await?;
//! assert!(saved.is_some());
//! # Ok(())
//! # }
//! ```

use crate::error::Result;
use crate::protocol::Page;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;

/// The JS shim. Installed both as an init script (so it survives
/// navigations) and evaluated immediately (so it works on the current
/// document). All state lives on `window.__pwRsFakeFs`; the guard on the
/// first line makes double-installation a no-op.
const SHIM: &str = r#"(() => {
    if (window.__pwRsFakeFs) return;
    const state = {
        saves: [],            // { name, b64 }
        openFiles: new Map(), // name -> b64
        permission: 'granted',
    };
    const b64encode = (bytes) => {
        let s = '';
        bytes.forEach((b) => { s += String.fromCharCode(b); });
        return btoa(s);
    };
    const b64decode = (b64) => Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
    const toBytes = async (data) => {
        if (typeof data === 'string') return new TextEncoder().encode(data);
        if (data instanceof Blob) return new Uint8Array(await data.arrayBuffer());
        if (data instanceof ArrayBuffer) return new Uint8Array(data);
        if (ArrayBuffer.isView(data)) {
            return new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
        }
        // FileSystemWriteChunkType object form: { type: 'write', data }
        if (data && data.type === 'write') return toBytes(data.data);
        throw new TypeError('fake fs: unsupported write payload');
    };
    const makeHandle = (name) => ({
        // Marks the object as a fake handle so the IndexedDB interception below
        // can swap it for a serializable placeholder before structuredClone.
        __pwRsFakeFsHandle: true,
        kind: 'file',
        name,
        isSameEntry: async (other) => !!other && other.name === name,
        queryPermission: async () => state.permission,
        requestPermission: async () => {
            if (state.permission === 'prompt') state.permission = 'granted';
            return state.permission;
        },
        getFile: async () => {
            const b64 = state.openFiles.get(name) ?? '';
            return new File([b64decode(b64)], name);
        },
        createWritable: async () => {
            const chunks = [];
            return {
                write: async (data) => { chunks.push(await toBytes(data)); },
                seek: async () => {},
                truncate: async () => {},
                abort: async () => {},
                close: async () => {
                    let total = 0;
                    chunks.forEach((c) => { total += c.length; });
                    const all = new Uint8Array(total);
                    let offset = 0;
                    chunks.forEach((c) => { all.set(c, offset); offset += c.length; });
                    const b64 = b64encode(all);
                    state.saves.push({ name, b64 });
                    state.openFiles.set(name, b64);
                },
            };
        },
    });
    window.__pwRsFakeFs = {
        lastSaved: () => (state.saves.length ? state.saves[state.saves.length - 1] : null),
        setOpenFile: (name, b64) => { state.openFiles.set(name, b64); },
        setPermission: (p) => { state.permission = p; },
    };
    // Real FileSystemFileHandle is [Serializable], so apps persist it in
    // IndexedDB and re-query permission on the next load. Our fake carries
    // methods that structuredClone rejects (DataCloneError), which would break
    // that flow. Intercept put/add to store a serializable placeholder keyed by
    // name, and get to rehydrate it into a live handle. Non-handle values pass
    // through untouched, so the app's other IndexedDB usage is unaffected.
    if (window.IDBObjectStore) {
        const MARK = '__pwRsFakeFsHandleName';
        const placeholder = (v) =>
            (v && typeof v === 'object' && v.__pwRsFakeFsHandle) ? { [MARK]: v.name } : v;
        const wrapWrite = (orig) =>
            function (value, ...rest) { return orig.call(this, placeholder(value), ...rest); };
        IDBObjectStore.prototype.put = wrapWrite(IDBObjectStore.prototype.put);
        IDBObjectStore.prototype.add = wrapWrite(IDBObjectStore.prototype.add);
        const realGet = IDBObjectStore.prototype.get;
        IDBObjectStore.prototype.get = function (...args) {
            const req = realGet.apply(this, args);
            // Registered here, before the caller sets onsuccess, so the
            // rehydrated value is in place when their handler reads req.result.
            req.addEventListener('success', () => {
                const val = req.result;
                if (val && typeof val === 'object' && MARK in val) {
                    Object.defineProperty(req, 'result', {
                        configurable: true,
                        value: makeHandle(val[MARK]),
                    });
                }
            });
            return req;
        };
    }
    window.showSaveFilePicker = async (options) => {
        if (state.permission === 'denied') {
            throw new DOMException('fake fs: permission denied', 'NotAllowedError');
        }
        return makeHandle((options && options.suggestedName) || 'untitled');
    };
    window.showOpenFilePicker = async () => {
        if (state.permission === 'denied') {
            throw new DOMException('fake fs: permission denied', 'NotAllowedError');
        }
        const names = [...state.openFiles.keys()];
        if (names.length === 0) {
            throw new DOMException('fake fs: no open file seeded', 'AbortError');
        }
        return [makeHandle(names[names.length - 1])];
    };
})()"#;

/// Handle to the fake File System Access API installed on a [`Page`] by
/// [`Page::fake_file_system`](crate::protocol::Page::fake_file_system).
///
/// See the [module docs](self) for the pattern and a usage example. Scope of
/// the fake: `showSaveFilePicker`, `showOpenFilePicker`, per-handle
/// `getFile`/`createWritable`/`queryPermission`/`requestPermission`, and
/// persisting a handle to IndexedDB — apps that stash the picker handle in
/// IndexedDB and re-`queryPermission` on the next load (the standard
/// startup-reopen pattern) work, because the fake stores a serializable
/// placeholder in place of the method-bearing handle and rehydrates it on read
/// (real `FileSystemFileHandle` is `[Serializable]`; the fake is not, so a raw
/// `put` would otherwise throw `DataCloneError`). The in-memory file *content*
/// and permission state still live in page JS, so they reset on a full page
/// reload. To test an app that reads its last file on *startup* (reading before
/// any test code could re-seed), use
/// [`seed_on_navigation`](Self::seed_on_navigation), which re-establishes
/// content and permission before the app mounts on the next navigation;
/// [`set_open_file`](Self::set_open_file) only takes effect at call time.
/// `showDirectoryPicker` is not faked.
///
/// IndexedDB access requires a real origin: install the fake, then
/// [`goto`](crate::protocol::Page::goto) an `http(s)://` page rather than using
/// `set_content` (opaque origins deny IndexedDB) if the flow under test
/// persists handles.
#[derive(Debug, Clone)]
pub struct FakeFileSystem {
    page: Page,
}

impl FakeFileSystem {
    /// Install the fake on `page` (init script + current document).
    pub(crate) async fn install(page: &Page) -> Result<Self> {
        page.add_init_script(SHIM).await?;
        page.evaluate_expression(SHIM).await?;
        Ok(Self { page: page.clone() })
    }

    /// The file name passed to the most recent completed save, or `None` if
    /// nothing has been saved.
    ///
    /// # Errors
    ///
    /// Returns an error if the page is closed or the evaluation fails.
    pub async fn last_saved_name(&self) -> Result<Option<String>> {
        self.page
            .evaluate(
                "() => { const s = window.__pwRsFakeFs.lastSaved(); return s ? s.name : null; }",
                None::<&()>,
            )
            .await
    }

    /// The bytes written by the most recent completed save (everything
    /// written between `createWritable()` and `close()`), or `None` if
    /// nothing has been saved.
    ///
    /// # Errors
    ///
    /// Returns an error if the page is closed, the evaluation fails, or the
    /// shim returns malformed base64 (which indicates a bug in the shim, not
    /// the caller).
    pub async fn last_saved_bytes(&self) -> Result<Option<Vec<u8>>> {
        let b64: Option<String> = self
            .page
            .evaluate(
                "() => { const s = window.__pwRsFakeFs.lastSaved(); return s ? s.b64 : null; }",
                None::<&()>,
            )
            .await?;
        b64.map(|s| {
            BASE64
                .decode(s)
                .map_err(|e| crate::error::Error::ProtocolError(format!("fake fs base64: {e}")))
        })
        .transpose()
    }

    /// Seed a file that the app's next `showOpenFilePicker()` call will
    /// "pick". Seeding the same name again replaces the content.
    ///
    /// # Errors
    ///
    /// Returns an error if the page is closed or the evaluation fails.
    pub async fn set_open_file(&self, name: &str, bytes: &[u8]) -> Result<()> {
        let arg = (name, BASE64.encode(bytes));
        let _: Option<()> = self
            .page
            .evaluate(
                "([name, b64]) => { window.__pwRsFakeFs.setOpenFile(name, b64); }",
                Some(&arg),
            )
            .await?;
        Ok(())
    }

    /// Set the permission state reported by the fake handles'
    /// `queryPermission` / `requestPermission`: `"granted"`, `"prompt"`, or
    /// `"denied"`. Defaults to `"granted"`. In the `"prompt"` state,
    /// `requestPermission` upgrades to `"granted"`, mirroring the real API's
    /// user-approval path; in `"denied"`, the pickers throw
    /// `NotAllowedError`.
    ///
    /// # Errors
    ///
    /// Returns an error if the page is closed or the evaluation fails.
    pub async fn set_permission(&self, state: &str) -> Result<()> {
        let _: Option<()> = self
            .page
            .evaluate(
                "(state) => { window.__pwRsFakeFs.setPermission(state); }",
                Some(&state),
            )
            .await?;
        Ok(())
    }

    /// Shorthand for [`set_permission("granted")`](Self::set_permission).
    ///
    /// # Errors
    ///
    /// Returns an error if the page is closed or the evaluation fails.
    pub async fn grant_permission(&self) -> Result<()> {
        self.set_permission("granted").await
    }

    /// Re-establishes openable-file content and permission state *before the app
    /// mounts on the next navigation* (including `reload`).
    ///
    /// [`set_open_file`](Self::set_open_file) and
    /// [`set_permission`](Self::set_permission) take effect at call time, which
    /// is too late for an app that reads its last file on startup: a full page
    /// reload clears the fake's in-memory content and resets permission to
    /// `"granted"`, and by the time a test could re-seed, the app has already
    /// mounted and read. This registers an init script (running after the fake's
    /// own, so `window` state exists) that re-seeds the given file and
    /// permission on every subsequent navigation — making the "reopen the last
    /// file on startup" flow testable across a reload. The persisted *handle*
    /// already survives a reload on its own (the fake stores it in IndexedDB and
    /// rehydrates it); only content and permission need re-seeding.
    ///
    /// The seed persists for all later navigations in this context, so seed the
    /// state the app should observe on its next load. `permission` takes the
    /// same values as [`set_permission`](Self::set_permission).
    ///
    /// # Errors
    ///
    /// Returns an error if the page is closed or registering the script fails.
    pub async fn seed_on_navigation(
        &self,
        name: &str,
        bytes: &[u8],
        permission: &str,
    ) -> Result<()> {
        // JSON-encode each value into a safe JS string literal (handles quotes
        // and other characters in names). Serializing a &str/String to JSON is
        // infallible for valid UTF-8, which Rust strings always are.
        let to_js = |s: &str| {
            serde_json::to_string(s).map_err(|e| {
                crate::error::Error::ProtocolError(format!("fake fs seed encode: {e}"))
            })
        };
        let name = to_js(name)?;
        let b64 = to_js(&BASE64.encode(bytes))?;
        let perm = to_js(permission)?;
        let script = format!(
            "(() => {{ const fs = window.__pwRsFakeFs; if (!fs) return; \
             fs.setOpenFile({name}, {b64}); fs.setPermission({perm}); }})()"
        );
        self.page.add_init_script(&script).await
    }
}
