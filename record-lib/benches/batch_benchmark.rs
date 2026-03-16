//! Benchmarks for batch recording operations.
//!
//! This module uses Criterion to measure and analyze the performance
//! of batch recording functionality.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use record_lib::batch::BatchRecorder;
use record_lib::domain::metadata::RecordingMetadata;
use record_lib::domain::service::{Program, RecordService};
use record_lib::record::hibiki_scraper::HibikiScraper;
use record_lib::utils::RecordError;
use scraper::Html;
use std::hint::black_box;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

/// Mock recording service for benchmarking.
struct MockRecordService;

#[async_trait::async_trait]
impl RecordService for MockRecordService {
    async fn record(
        &self,
        _url: &str,
        _output_path: &std::path::Path,
    ) -> Result<RecordingMetadata, RecordError> {
        // Simulate some work
        tokio::time::sleep(Duration::from_micros(100)).await;
        Ok(RecordingMetadata::new(
            "Mock".to_string(),
            chrono::Utc::now(),
            chrono::Utc::now(),
            "/tmp/mock.m4a".to_string(),
            1024,
            128,
        ))
    }
}

fn create_programs(count: usize) -> Vec<Program> {
    (0..count)
        .map(|i| Program {
            title: format!("Program {}", i),
            url: format!("http://example.com/{}", i),
            output_path: PathBuf::from(format!("/tmp/{}.m4a", i)),
        })
        .collect()
}

/// Benchmark batch recording with different program counts.
fn bench_batch_recording(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let service = Arc::new(MockRecordService);

    let mut group = c.benchmark_group("batch_recording");
    for count in [1, 5, 10, 20, 50].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, count| {
            b.iter(|| {
                rt.block_on(async {
                    let recorder = BatchRecorder::new(*count, 1, Duration::from_secs(30));
                    let programs = create_programs(*count);
                    let service = Arc::clone(&service);
                    recorder
                        .record_batch(black_box(programs), service)
                        .await
                        .unwrap()
                })
            });
        });
    }
    group.finish();
}

/// Benchmark with different parallel job limits.
/// Benchmark Hibiki HTML scraping logic.
fn bench_hibiki_scraping(c: &mut Criterion) {
    let scraper = HibikiScraper::new().unwrap();
    let html_content = r#"
        <!DOCTYPE html>
        <html>
        <head><title>Hibiki Radio Test</title></head>
        <body>
            <div data-streaming-url="https://example.com/data.m3u8"></div>
            <script>var url = "https://example.com/script.m3u8";</script>
            <script>{"streamingUrl": "https://example.com/json.m3u8"}</script>
            <iframe src="https://example.com/iframe.m3u8"></iframe>
            <script>console.log("nothing here");</script>
            <script>console.log("nothing here either");</script>
            <script>console.log("still nothing");</script>
        </body>
        </html>
    "#;
    let document = Html::parse_document(html_content);

    c.bench_function("hibiki_extract_streaming_url", |b| {
        b.iter(|| {
            scraper
                .extract_streaming_url_from_document(black_box(&document))
                .unwrap()
        })
    });
}

fn bench_parallel_jobs(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let service = Arc::new(MockRecordService);
    let programs = create_programs(10);

    let mut group = c.benchmark_group("parallel_jobs");
    for max_jobs in [1, 2, 3, 5, 10].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(max_jobs),
            max_jobs,
            |b, max_jobs| {
                b.iter(|| {
                    rt.block_on(async {
                        let recorder = BatchRecorder::new(*max_jobs, 1, Duration::from_secs(30));
                        let programs = programs.clone();
                        let service = Arc::clone(&service);
                        recorder
                            .record_batch(black_box(programs), service)
                            .await
                            .unwrap()
                    })
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_batch_recording,
    bench_parallel_jobs,
    bench_hibiki_scraping
);
criterion_main!(benches);
