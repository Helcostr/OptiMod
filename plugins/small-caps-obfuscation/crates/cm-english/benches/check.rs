use cm_core::{Checker, TWITCH_SMALL_CAPS_SPAM};
use cm_english::EnglishChecker;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::fs;
use std::path::PathBuf;

// Mirrors fixture_tests::repo_root in src/lib.rs.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn bench_inputs() -> Vec<(&'static str, String)> {
    let root = repo_root();
    let spam = fs::read_to_string(
        root.join("fixtures")
            .join(TWITCH_SMALL_CAPS_SPAM)
            .join("input.txt"),
    )
    .expect("spam input")
    .trim()
    .to_string();

    vec![
        ("ascii_short", "hello stream, thanks for watching!".repeat(3)),
        ("ascii_140", "a".repeat(140)),
        ("mixed_small_caps", "ʏᴏ ʙʀᴏ ᴊᴜꜱᴛ ᴡᴀɴᴛᴇᴅ ᴛᴏ ꜱʜᴏᴡ".to_string()),
        ("spam_full", spam),
    ]
}

fn bench_group<F>(c: &mut Criterion, group_name: &str, run: F)
where
    F: Fn(&EnglishChecker, &str) + Copy,
{
    let checker = EnglishChecker::default();
    let mut group = c.benchmark_group(group_name);

    for (name, input) in bench_inputs() {
        group.bench_function(name, |b| {
            b.iter(|| run(&checker, black_box(&input)));
        });
    }

    group.finish();
}

fn english_is_good_benchmark(c: &mut Criterion) {
    bench_group(c, "english_is_good", |checker, input| {
        let _ = checker.is_good(input);
    });
}

fn english_evaluate_benchmark(c: &mut Criterion) {
    bench_group(c, "english_evaluate", |checker, input| {
        let _ = checker.evaluate(input);
    });
}

criterion_group!(benches, english_is_good_benchmark, english_evaluate_benchmark);
criterion_main!(benches);
