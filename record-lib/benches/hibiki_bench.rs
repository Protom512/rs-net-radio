use criterion::{black_box, criterion_group, criterion_main, Criterion};
use record_lib::record::hibiki_scraper::HibikiScraper;
use scraper::Html;

fn bench_hibiki_extraction(c: &mut Criterion) {
    let scraper = HibikiScraper::new().unwrap();
    let html_content = r#"
        <!DOCTYPE html>
        <html>
        <head><title>Hibiki Radio Test</title></head>
        <body>
            <div class="program-detail">
                <script>
                    var programData = {
                        "id": 1234,
                        "title": "Test Program",
                        "streamingUrl": "https://example.com/stream/playlist.m3u8?token=abcdef123456"
                    };
                </script>
                <div data-streaming-url="https://example.com/stream/fallback.m3u8"></div>
                <iframe src="https://example.com/player?url=https://example.com/stream/iframe.m3u8"></iframe>
            </div>
        </body>
        </html>
    "#;
    let document = Html::parse_document(html_content);

    c.bench_function("hibiki_extract_streaming_url", |b| {
        b.iter(|| {
            let _ = scraper
                .extract_streaming_url_from_document(black_box(&document))
                .unwrap();
        })
    });
}

criterion_group!(benches, bench_hibiki_extraction);
criterion_main!(benches);
