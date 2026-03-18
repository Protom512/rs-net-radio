//! Benchmarks for Hibiki radio scraping.
//!
//! This module uses Criterion to measure and analyze the performance
//! of Hibiki radio scraping and URL extraction.

use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use record_lib::record::hibiki_scraper::HibikiScraper;
use scraper::Html;

/// Benchmark Hibiki Radio URL extraction from a pre-parsed HTML document.
fn bench_hibiki_extraction(c: &mut Criterion) {
    let scraper = HibikiScraper::new().expect("Failed to create scraper");

    // Sample HTML content similar to what Hibiki Radio uses
    let html_content = r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>Test Program - Hibiki Radio</title>
            <script>
                var unusedVar = "some data";
                var moreUnused = 123;
            </script>
        </head>
        <body>
            <div id="wrapper">
                <header><h1>Hibiki Radio</h1></header>
                <main>
                    <div class="program-info">
                        <h2>Test Program</h2>
                        <div class="player-container"
                             data-streaming-url="https://example.com/stream.m3u8?token=abcdef123456"
                             data-program-id="12345">
                        </div>
                    </div>
                </main>
                <script>
                    (function() {
                        console.log("Initializing player...");
                        var config = {
                            url: "https://example.com/other-url.m3u8",
                            autoplay: false
                        };
                    })();
                </script>
            </div>
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

criterion_group!(benches, bench_hibiki_extraction);
criterion_main!(benches);
