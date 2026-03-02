use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use record_lib::record::hibiki_scraper::HibikiScraper;
use scraper::Html;

fn bench_hibiki_scraper(c: &mut Criterion) {
    let scraper = HibikiScraper::new().unwrap();
    let html_content = r#"
        <!DOCTYPE html>
        <html>
        <head><title>Test</title></head>
        <body>
            <script>
                var streamingUrl = "https://example.com/stream.m3u8?token=abc123";
                console.log(streamingUrl);
            </script>
            <div data-streaming-url="https://example.com/video.m3u8"></div>
            <iframe src="https://example.com/player.m3u8"></iframe>
        </body>
        </html>
    "#;
    let document = Html::parse_document(html_content);

    c.bench_function("HibikiScraper::extract_streaming_url_from_document", |b| {
        b.iter(|| {
            scraper.extract_streaming_url_from_document(black_box(&document)).unwrap();
        })
    });
}

criterion_group!(benches, bench_hibiki_scraper);
criterion_main!(benches);
