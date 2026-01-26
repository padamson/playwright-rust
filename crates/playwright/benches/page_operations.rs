// Benchmarks for page operations
//
// This benchmark suite measures the performance of key page operations:
// - Page navigation (goto)
// - Element queries (locator operations)
// - JavaScript evaluation
// - Screenshots

use criterion::{Criterion, criterion_group, criterion_main};
use playwright_rs::{Page, Playwright};
use std::hint::black_box;
use std::time::Duration;

// Helper to set up a test server with basic HTML
const TEST_HTML: &str = r#"
<!DOCTYPE html>
<html>
<head>
    <title>Benchmark Page</title>
</head>
<body>
    <h1 id="title">Benchmark Test Page</h1>
    <div class="container">
        <button id="button1" class="btn">Button 1</button>
        <button id="button2" class="btn">Button 2</button>
        <button id="button3" class="btn">Button 3</button>
        <input type="text" id="input" placeholder="Enter text">
        <select id="select">
            <option value="1">Option 1</option>
            <option value="2">Option 2</option>
            <option value="3">Option 3</option>
        </select>
    </div>
    <p>Lorem ipsum dolor sit amet, consectetur adipiscing elit.</p>
    <div id="dynamic"></div>
    <script>
        // Add some dynamic content for benchmarking
        for (let i = 0; i < 100; i++) {
            const div = document.createElement('div');
            div.className = 'item';
            div.textContent = 'Item ' + i;
            document.getElementById('dynamic').appendChild(div);
        }
    </script>
</body>
</html>
"#;

async fn setup_page() -> (Playwright, Page, String) {
    let playwright = Playwright::launch().await.unwrap();
    let browser = playwright.chromium().launch().await.unwrap();
    let page = browser.new_page().await.unwrap();

    // Create a data URL for the test HTML
    let data_url = format!("data:text/html,{}", urlencoding::encode(TEST_HTML));

    // Navigate to the test page
    page.goto(&data_url, None).await.unwrap();

    (playwright, page, data_url)
}

fn page_navigation_benchmark(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    // Setup once
    let (_playwright, page, data_url) = runtime.block_on(setup_page());

    let mut group = c.benchmark_group("page_navigation");
    group.measurement_time(Duration::from_secs(20));
    group.sample_size(30);

    // Benchmark page.goto()
    group.bench_function("goto_data_url", |b| {
        b.iter(|| {
            runtime.block_on(async {
                page.goto(&data_url, None).await.unwrap();
            });
        });
    });

    // Benchmark page.reload()
    group.bench_function("reload", |b| {
        b.iter(|| {
            runtime.block_on(async {
                page.reload(None).await.unwrap();
            });
        });
    });

    group.finish();

    // Cleanup
    runtime.block_on(async {
        page.close().await.unwrap();
    });
}

fn element_query_benchmark(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    // Setup once
    let (_playwright, page, _data_url) = runtime.block_on(setup_page());

    let mut group = c.benchmark_group("element_queries");
    group.measurement_time(Duration::from_secs(15));
    group.sample_size(100);

    // Benchmark locator by ID
    group.bench_function("locator_by_id", |b| {
        b.iter(|| {
            runtime.block_on(async {
                let locator = page.locator("#button1").await;
                black_box(locator);
            });
        });
    });

    // Benchmark locator by class (multiple elements)
    group.bench_function("locator_by_class", |b| {
        b.iter(|| {
            runtime.block_on(async {
                let locator = page.locator(".btn").await;
                black_box(locator);
            });
        });
    });

    // Benchmark locator count
    group.bench_function("locator_count", |b| {
        b.iter(|| {
            runtime.block_on(async {
                let locator = page.locator(".item").await;
                let count = locator.count().await;
                let _ = black_box(count);
            });
        });
    });

    // Benchmark is_visible check
    group.bench_function("is_visible", |b| {
        b.iter(|| {
            runtime.block_on(async {
                let locator = page.locator("#title").await;
                let is_visible = locator.is_visible().await;
                let _ = black_box(is_visible);
            });
        });
    });

    group.finish();

    // Cleanup
    runtime.block_on(async {
        page.close().await.unwrap();
    });
}

fn javascript_evaluation_benchmark(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    // Setup once
    let (_playwright, page, _data_url) = runtime.block_on(setup_page());

    let mut group = c.benchmark_group("javascript");
    group.measurement_time(Duration::from_secs(15));
    group.sample_size(100);

    // Benchmark simple expression evaluation
    group.bench_function("evaluate_simple", |b| {
        b.iter(|| {
            runtime.block_on(async {
                let result = page.evaluate_value("1 + 1").await.unwrap();
                black_box(result);
            });
        });
    });

    // Benchmark DOM query evaluation
    group.bench_function("evaluate_dom_query", |b| {
        b.iter(|| {
            runtime.block_on(async {
                let result = page
                    .evaluate_value("document.querySelectorAll('.item').length")
                    .await
                    .unwrap();
                black_box(result);
            });
        });
    });

    // Benchmark function evaluation
    group.bench_function("evaluate_function", |b| {
        b.iter(|| {
            runtime.block_on(async {
                let result = page
                    .evaluate_value(
                        "(() => { return Array.from({length: 10}, (_, i) => i * 2); })()",
                    )
                    .await
                    .unwrap();
                black_box(result);
            });
        });
    });

    group.finish();

    // Cleanup
    runtime.block_on(async {
        page.close().await.unwrap();
    });
}

fn screenshot_benchmark(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    // Setup once
    let (_playwright, page, _data_url) = runtime.block_on(setup_page());

    let mut group = c.benchmark_group("screenshot");
    group.measurement_time(Duration::from_secs(20));
    group.sample_size(20); // Screenshots are expensive

    // Benchmark full page screenshot
    group.bench_function("full_page", |b| {
        b.iter(|| {
            runtime.block_on(async {
                let screenshot = page.screenshot(None).await.unwrap();
                black_box(screenshot);
            });
        });
    });

    group.finish();

    // Cleanup
    runtime.block_on(async {
        page.close().await.unwrap();
    });
}

criterion_group!(
    benches,
    page_navigation_benchmark,
    element_query_benchmark,
    javascript_evaluation_benchmark,
    screenshot_benchmark
);
criterion_main!(benches);
