use core::watcher;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_get_cached_state(c: &mut Criterion) {
    watcher::start_watcher();
    std::thread::sleep(std::time::Duration::from_millis(2200));

    c.bench_function("watcher/get_cached_state", |b| {
        b.iter(|| {
            let state = watcher::get_cached_state();
            black_box(state)
        })
    });
}

criterion_group!(benches, bench_get_cached_state);
criterion_main!(benches);
