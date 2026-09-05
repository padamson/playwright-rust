// The app under test: this site's wasm bundle plus the JSON its version
// switcher fetches, on one axum router. The test owns that JSON and can
// change it mid-run. Nothing is listening, and the origin is made up.
let backend = Arc::new(Mutex::new(Some(
    r#"{"latest":"9.9.9","versions":["9.9.9","0.17.0"]}"#.to_string(),
)));
let answers = backend.clone();
let app = Router::new()
    .route("/versions.json", get(move || {
        let answer = answers.lock().unwrap().clone();
        async move {
            match answer {
                Some(json) => ([(CONTENT_TYPE, "application/json")], json).into_response(),
                None => StatusCode::SERVICE_UNAVAILABLE.into_response(),
            }
        }
    }))
    .fallback_service(ServeDir::new("dist"));

page.route_service("https://playwright-rust.test/**", app).await?;
page.goto("https://playwright-rust.test/", None).await?;
expect(page.locator("#hero-title"))
    .to_have_text("Playwright for Rust")
    .await?;
