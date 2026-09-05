// Change what the backend says, without restarting anything. The app
// follows on its next load.
*backend.lock().unwrap() =
    Some(r#"{"latest":"0.17.0","versions":["0.17.0"]}"#.to_string());
page.reload(None).await?;
expect(page.locator("#version-switcher a"))
    .to_contain_text("v0.17.0")
    .await?;
