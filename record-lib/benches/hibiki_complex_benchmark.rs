use criterion::{criterion_group, criterion_main, Criterion};
use record_lib::record::hibiki_scraper::HibikiScraper;
use scraper::Html;
use std::hint::black_box;

fn bench_hibiki_complex_extraction(c: &mut Criterion) {
    let scraper = HibikiScraper::new().expect("Failed to create scraper");

    let mut html_content = String::from("<!DOCTYPE html><html><body>");

    // Add 100 script tags without URLs
    for i in 0..100 {
        html_content.push_str(&format!("<script>var x{} = 'some data without url'; console.log(x{});</script>", i, i));
    }

    // Add 100 div tags without relevant attributes
    for i in 0..100 {
        html_content.push_str(&format!("<div class='item{}' data-id='{}'>Item {}</div>", i, i, i));
    }

    // Add the target div at the end
    html_content.push_str(r#"
        <div class="player-container"
             data-url="https://example.com/stream.m3u8">
        </div>
    "#);

    html_content.push_str("</body></html>");

    let document = Html::parse_document(&html_content);

    c.bench_function("hibiki_extract_complex", |b| {
        b.iter(|| {
            scraper
                .extract_streaming_url_from_document(black_box(&document))
                .unwrap()
        })
    });
}

fn bench_hibiki_script_fallback(c: &mut Criterion) {
    let scraper = HibikiScraper::new().expect("Failed to create scraper");

    let mut html_content = String::from("<!DOCTYPE html><html><body>");

    // Add 100 script tags without URLs
    for i in 0..100 {
        html_content.push_str(&format!("<script>var x{} = 'some data without url';</script>", i));
    }

    // Add the target script at the end
    html_content.push_str(r#"
        <script>
            var streamingUrl = "https://example.com/stream.m3u8";
        </script>
    "#);

    html_content.push_str("</body></html>");

    let document = Html::parse_document(&html_content);

    c.bench_function("hibiki_extract_script_fallback", |b| {
        b.iter(|| {
            scraper
                .extract_streaming_url_from_document(black_box(&document))
                .unwrap()
        })
    });
}

criterion_group!(benches, bench_hibiki_complex_extraction, bench_hibiki_script_fallback);
criterion_main!(benches);
