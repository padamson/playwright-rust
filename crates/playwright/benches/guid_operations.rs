// Benchmarks for GUID operations
//
// This benchmark suite measures the performance of GUID-related operations
// to establish a baseline before optimizations.

use criterion::{Criterion, criterion_group, criterion_main};
use std::collections::HashMap;
use std::sync::Arc;

// Simulate current GUID usage patterns
fn benchmark_guid_string_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("guid_operations");

    // Benchmark String cloning (current approach)
    group.bench_function("string_clone", |b| {
        let guid = String::from("browser-type@756fe8ab659e0a266817ff3aa31c424c");
        b.iter(|| {
            let cloned = guid.clone();
            std::hint::black_box(cloned);
        });
    });

    // Benchmark Arc<str> cloning (potential optimization)
    group.bench_function("arc_str_clone", |b| {
        let guid: Arc<str> = Arc::from("browser-type@756fe8ab659e0a266817ff3aa31c424c");
        b.iter(|| {
            let cloned = guid.clone();
            std::hint::black_box(cloned);
        });
    });

    // Benchmark HashMap insertion with String keys
    group.bench_function("hashmap_string_insert", |b| {
        b.iter(|| {
            let mut map: HashMap<String, i32> = HashMap::new();
            for i in 0..100 {
                let guid = format!("object@{:x}", i);
                map.insert(guid, i);
            }
            std::hint::black_box(map);
        });
    });

    // Benchmark HashMap insertion with Arc<str> keys
    group.bench_function("hashmap_arc_str_insert", |b| {
        b.iter(|| {
            let mut map: HashMap<Arc<str>, i32> = HashMap::new();
            for i in 0..100 {
                let guid: Arc<str> = Arc::from(format!("object@{:x}", i));
                map.insert(guid, i);
            }
            std::hint::black_box(map);
        });
    });

    // Benchmark HashMap lookup with String keys
    group.bench_function("hashmap_string_lookup", |b| {
        let mut map: HashMap<String, i32> = HashMap::new();
        for i in 0..1000 {
            let guid = format!("object@{:x}", i);
            map.insert(guid, i);
        }

        b.iter(|| {
            let key = String::from("object@1f4");
            let value = map.get(&key);
            std::hint::black_box(value);
        });
    });

    // Benchmark HashMap lookup with Arc<str> keys
    group.bench_function("hashmap_arc_str_lookup", |b| {
        let mut map: HashMap<Arc<str>, i32> = HashMap::new();
        for i in 0..1000 {
            let guid: Arc<str> = Arc::from(format!("object@{:x}", i));
            map.insert(guid, i);
        }
        let lookup_key: Arc<str> = Arc::from("object@1f4");

        b.iter(|| {
            let value = map.get(&lookup_key);
            std::hint::black_box(value);
        });
    });

    // Benchmark memory usage for storing 1000 GUIDs as String
    group.bench_function("memory_string_1000", |b| {
        b.iter(|| {
            let mut guids: Vec<String> = Vec::with_capacity(1000);
            for i in 0..1000 {
                guids.push(format!("browser-type@{:032x}", i));
            }
            std::hint::black_box(guids);
        });
    });

    // Benchmark memory usage for storing 1000 GUIDs as Arc<str>
    group.bench_function("memory_arc_str_1000", |b| {
        b.iter(|| {
            let mut guids: Vec<Arc<str>> = Vec::with_capacity(1000);
            for i in 0..1000 {
                guids.push(Arc::from(format!("browser-type@{:032x}", i)));
            }
            std::hint::black_box(guids);
        });
    });

    group.finish();
}

criterion_group!(benches, benchmark_guid_string_operations);
criterion_main!(benches);
