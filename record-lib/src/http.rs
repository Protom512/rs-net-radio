use std::sync::OnceLock;
use std::time::Duration;

fn build_blocking_client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap_or_else(|err| {
            tracing::error!("Failed to build shared blocking reqwest client: {err}; using defaults");
            reqwest::blocking::Client::new()
        })
}

fn build_async_client(timeout_secs: u64) -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .unwrap_or_else(|err| {
            tracing::error!(
                "Failed to build shared async reqwest client (timeout={timeout_secs}s): {err}; using defaults"
            );
            reqwest::Client::new()
        })
}

/// Returns a shared, lazily-initialized `reqwest::blocking::Client`.
///
/// Reusing the client allows for connection pooling, which improves performance
/// by avoiding the overhead of repeated TCP and TLS handshakes.
pub fn blocking_client() -> &'static reqwest::blocking::Client {
    static CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();
    CLIENT.get_or_init(build_blocking_client)
}

/// Returns a shared, lazily-initialized `reqwest::Client`.
///
/// Reusing the client allows for connection pooling, which improves performance
/// by avoiding the overhead of repeated TCP and TLS handshakes.
pub fn async_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| build_async_client(30))
}

/// Returns a shared, lazily-initialized `reqwest::Client` for long-running streaming downloads.
///
/// Reusing the client allows for connection pooling, which improves performance
/// by avoiding the overhead of repeated TCP and TLS handshakes.
pub fn streaming_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| build_async_client(600))
}
