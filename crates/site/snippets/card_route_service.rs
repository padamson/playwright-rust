let app: Router = Router::new()
    .route("/api/todos", get(|| async { Json(vec!["write tests"]) }))
    .fallback_service(ServeDir::new("dist"));

// No port, no server task: the browser talks to `app` in-process.
page.route_service("https://app.test/**", app).await?;
page.goto("https://app.test/", None).await?;
