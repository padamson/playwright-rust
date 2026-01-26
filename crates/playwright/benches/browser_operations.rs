// Benchmarks for browser operations
//
// This benchmark suite measures the performance of key browser operations:
// - Browser launch time
// - Browser context creation
// - Browser close time

use criterion::{Criterion, criterion_group, criterion_main};
use playwright_rs::Playwright;
use std::hint::black_box;
use std::time::Duration;

fn browser_launch_benchmark(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("browser_launch");
    group.measurement_time(Duration::from_secs(30)); // Longer time for browser operations
    group.sample_size(10); // Fewer samples since browser launch is expensive

    // Benchmark Chromium launch
    group.bench_function("chromium", |b| {
        b.iter(|| {
            runtime.block_on(async {
                let playwright = Playwright::launch().await.unwrap();
                let browser = playwright.chromium().launch().await.unwrap();
                browser.close().await.unwrap();
            });
        });
    });

    // Benchmark Firefox launch
    group.bench_function("firefox", |b| {
        b.iter(|| {
            runtime.block_on(async {
                let playwright = Playwright::launch().await.unwrap();
                let browser = playwright.firefox().launch().await.unwrap();
                browser.close().await.unwrap();
            });
        });
    });

    // Benchmark WebKit launch
    group.bench_function("webkit", |b| {
        b.iter(|| {
            runtime.block_on(async {
                let playwright = Playwright::launch().await.unwrap();
                let browser = playwright.webkit().launch().await.unwrap();
                browser.close().await.unwrap();
            });
        });
    });

    group.finish();
}

fn browser_context_benchmark(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    // Set up browser once for context benchmarks
    let playwright = runtime.block_on(async { Playwright::launch().await.unwrap() });
    let browser = runtime.block_on(async { playwright.chromium().launch().await.unwrap() });

    let mut group = c.benchmark_group("browser_context");
    group.measurement_time(Duration::from_secs(20));
    group.sample_size(50); // More samples since context creation is faster

    // Benchmark context creation
    group.bench_function("new_context", |b| {
        b.iter(|| {
            runtime.block_on(async {
                let context = browser.new_context().await.unwrap();
                context.close().await.unwrap();
            });
        });
    });

    // Benchmark page creation within context
    group.bench_function("new_page", |b| {
        b.iter(|| {
            runtime.block_on(async {
                let page = browser.new_page().await.unwrap();
                page.close().await.unwrap();
            });
        });
    });

    group.finish();

    // Clean up
    runtime.block_on(async {
        browser.close().await.unwrap();
    });
}

fn playwright_launch_benchmark(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("playwright");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(20);

    // Benchmark Playwright server launch
    group.bench_function("launch", |b| {
        b.iter(|| {
            runtime.block_on(async {
                let playwright = Playwright::launch().await.unwrap();
                black_box(playwright); // Prevent optimization
            });
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    playwright_launch_benchmark,
    browser_launch_benchmark,
    browser_context_benchmark
);
criterion_main!(benches);
