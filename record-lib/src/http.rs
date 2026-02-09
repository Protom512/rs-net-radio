use std::sync::OnceLock;
use std::time::Duration;

/// Returns a shared, lazily-initialized `reqwest::blocking::Client`.
///
/// Reusing the client allows for connection pooling, which improves performance
/// by avoiding the overhead of repeated TCP and TLS handshakes.
pub fn blocking_client() -> &'static reqwest::blocking::Client {
    static CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create shared blocking reqwest client")
    })
}

/// Returns a shared, lazily-initialized `reqwest::Client`.
///
/// Reusing the client allows for connection pooling, which improves performance
/// by avoiding the overhead of repeated TCP and TLS handshakes.
pub fn async_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create shared async reqwest client")
    })
}

/// Returns a shared, lazily-initialized `reqwest::Client` for streaming.
///
/// Reusing the client allows for connection pooling, which is especially beneficial
/// for long-running streaming downloads.
pub fn streaming_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(600)) // 10 minutes timeout for streaming
            .build()
            .expect("Failed to create shared streaming reqwest client")
    })
}
