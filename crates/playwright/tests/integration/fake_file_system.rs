// Tests for Page::fake_file_system — the opt-in File System Access API fake.
//
// The fake replaces window.showSaveFilePicker / showOpenFilePicker with
// deterministic in-memory implementations so save/open flows are testable
// without a native picker dialog. Net-new convenience; upstream Playwright
// has no equivalent (microsoft/playwright#11288).

/// A page script that saves through showSaveFilePicker lands in the fake,
/// and the test can read back what was written.
#[tokio::test]
async fn test_save_flow_captures_bytes_and_name() {
    let (_pw, browser, page) = crate::common::setup().await;
    let fs = page
        .fake_file_system()
        .await
        .expect("fake_file_system should install");

    page.set_content("<html><body></body></html>", None)
        .await
        .expect("set_content");

    page.evaluate::<(), serde_json::Value>(
        r#"async () => {
            const handle = await window.showSaveFilePicker({ suggestedName: "plan.json" });
            const writable = await handle.createWritable();
            await writable.write("hello fake fs");
            await writable.close();
        }"#,
        None,
    )
    .await
    .expect("save flow should run");

    assert_eq!(
        fs.last_saved_name().await.expect("last_saved_name"),
        Some("plan.json".to_string())
    );
    assert_eq!(
        fs.last_saved_bytes().await.expect("last_saved_bytes"),
        Some(b"hello fake fs".to_vec())
    );

    browser.close().await.expect("browser close");
}

/// Bytes seeded with set_open_file come back through showOpenFilePicker.
#[tokio::test]
async fn test_open_flow_reads_seeded_file() {
    let (_pw, browser, page) = crate::common::setup().await;
    let fs = page.fake_file_system().await.expect("install");

    page.set_content("<html><body></body></html>", None)
        .await
        .expect("set_content");

    fs.set_open_file("notes.txt", b"seeded content")
        .await
        .expect("set_open_file");

    let read: String = page
        .evaluate(
            r#"async () => {
                const [handle] = await window.showOpenFilePicker();
                const file = await handle.getFile();
                return handle.name + ":" + (await file.text());
            }"#,
            None::<&()>,
        )
        .await
        .expect("open flow should run");

    assert_eq!(read, "notes.txt:seeded content");

    browser.close().await.expect("browser close");
}

/// The fake's permission state is settable, and requestPermission upgrades
/// prompt to granted like the real API's user-approval path.
#[tokio::test]
async fn test_permission_state_is_controllable() {
    let (_pw, browser, page) = crate::common::setup().await;
    let fs = page.fake_file_system().await.expect("install");

    page.set_content("<html><body></body></html>", None)
        .await
        .expect("set_content");
    fs.set_open_file("f.txt", b"x").await.expect("seed");
    fs.set_permission("prompt").await.expect("set_permission");

    let states: String = page
        .evaluate(
            r#"async () => {
                const [handle] = await window.showOpenFilePicker();
                const before = await handle.queryPermission({ mode: "readwrite" });
                const requested = await handle.requestPermission({ mode: "readwrite" });
                const after = await handle.queryPermission({ mode: "readwrite" });
                return [before, requested, after].join(",");
            }"#,
            None::<&()>,
        )
        .await
        .expect("permission flow");

    assert_eq!(states, "prompt,granted,granted");

    browser.close().await.expect("browser close");
}

/// A save round-trips: after the app writes a file, reopening through the
/// picker yields the written content (the in-place save/reopen loop).
#[tokio::test]
async fn test_save_then_reopen_round_trip() {
    let (_pw, browser, page) = crate::common::setup().await;
    let _fs = page.fake_file_system().await.expect("install");

    page.set_content("<html><body></body></html>", None)
        .await
        .expect("set_content");

    let reread: String = page
        .evaluate(
            r#"async () => {
                const handle = await window.showSaveFilePicker({ suggestedName: "doc.txt" });
                const writable = await handle.createWritable();
                await writable.write(new Blob(["written via blob"]));
                await writable.close();
                const [reopened] = await window.showOpenFilePicker();
                return await (await reopened.getFile()).text();
            }"#,
            None::<&()>,
        )
        .await
        .expect("round trip");

    assert_eq!(reread, "written via blob");

    browser.close().await.expect("browser close");
}

/// A fake handle survives being persisted to IndexedDB (the spec-recommended
/// pattern for apps that reopen the last file on startup) and rehydrates into a
/// working handle. Without structured-clone-safe handles, `indexedDB.put(handle)`
/// throws `DataCloneError` and an app's save flow silently breaks while the fake
/// still records the write — a false green.
#[tokio::test]
async fn test_handle_persists_to_indexeddb() {
    let (_pw, browser, page) = crate::common::setup().await;
    let fs = page.fake_file_system().await.expect("install");

    // IndexedDB is denied on opaque origins (about:blank / set_content), so
    // serve a real http:// origin — the same condition under which the bug
    // manifests for real apps.
    let server = crate::test_server::TestServer::start().await;
    page.goto(&server.url(), None).await.expect("navigate");
    fs.set_open_file("plan.json", b"saved bytes")
        .await
        .expect("seed");

    let round_trip: String = page
        .evaluate(
            r#"async () => {
                const [handle] = await window.showOpenFilePicker();
                const db = await new Promise((res, rej) => {
                    const r = indexedDB.open("pw-rs-fake-fs-test", 1);
                    r.onupgradeneeded = () => r.result.createObjectStore("handles");
                    r.onsuccess = () => res(r.result);
                    r.onerror = () => rej(r.error);
                });
                // Persist the handle the way real apps do (it is [Serializable]).
                await new Promise((res, rej) => {
                    const tx = db.transaction("handles", "readwrite");
                    tx.objectStore("handles").put(handle, "last");
                    tx.oncomplete = res;
                    tx.onerror = () => rej(tx.error);
                });
                const restored = await new Promise((res, rej) => {
                    const tx = db.transaction("handles", "readonly");
                    const rq = tx.objectStore("handles").get("last");
                    rq.onsuccess = () => res(rq.result);
                    rq.onerror = () => rej(rq.error);
                });
                const perm = await restored.queryPermission({ mode: "readwrite" });
                const text = await (await restored.getFile()).text();
                return restored.name + "|" + perm + "|" + text;
            }"#,
            None::<&()>,
        )
        .await
        .expect("persist round-trip should not DataCloneError");

    assert_eq!(round_trip, "plan.json|granted|saved bytes");

    server.shutdown();
    browser.close().await.expect("browser close");
}

/// `seed_on_navigation` re-establishes file content and permission state before
/// the app mounts on the next navigation, so the "reopen last file on startup"
/// pattern is testable across a `reload()` — the fake's in-memory content and
/// permission otherwise reset when the page reloads.
#[tokio::test]
async fn test_seed_on_navigation_survives_reload() {
    let (_pw, browser, page) = crate::common::setup().await;
    let fs = page.fake_file_system().await.expect("install");

    page.set_content("<html><body></body></html>", None)
        .await
        .expect("set_content");

    // A plain reload loses the in-memory content: seed now, reload, and without
    // seed_on_navigation the file would come back empty.
    fs.set_open_file("plan.json", b"before reload")
        .await
        .expect("seed");

    // Arrange for content + a lapsed ("prompt") permission to be present BEFORE
    // any app code runs on the next navigation — the auto-reopen-on-startup case.
    fs.seed_on_navigation("plan.json", b"seeded on nav", "prompt")
        .await
        .expect("seed_on_navigation");

    page.reload(None).await.expect("reload");

    let after: String = page
        .evaluate(
            r#"async () => {
                const [handle] = await window.showOpenFilePicker();
                const perm = await handle.queryPermission({ mode: "readwrite" });
                const text = await (await handle.getFile()).text();
                return handle.name + "|" + perm + "|" + text;
            }"#,
            None::<&()>,
        )
        .await
        .expect("post-reload read");

    assert_eq!(after, "plan.json|prompt|seeded on nav");

    browser.close().await.expect("browser close");
}

/// The fake is opt-in: a page that never asked for it sees no shim state.
#[tokio::test]
async fn test_fake_is_opt_in() {
    let (_pw, browser, page) = crate::common::setup().await;

    page.set_content("<html><body></body></html>", None)
        .await
        .expect("set_content");

    let untouched: bool = page
        .evaluate("() => window.__pwRsFakeFs === undefined", None::<&()>)
        .await
        .expect("probe");

    assert!(untouched, "shim state must not exist without opt-in");

    browser.close().await.expect("browser close");
}
