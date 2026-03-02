## 2025-05-22 - [Reuse reqwest Clients]
**Learning:** In Rust applications, creating a new `reqwest::Client` for every request is a major performance bottleneck due to the cost of repeatedly establishing TCP and TLS connections. Reusing a single `Client` (or one per thread/purpose) enables connection pooling, significantly reducing latency and resource usage.
**Action:** Always prefer a shared, static, or long-lived `reqwest::Client` instance over local instantiation. Use `std::sync::OnceLock` for lazy, thread-safe initialization of shared clients in Rust.

## 2025-05-23 - [reqwest::Client Cloning Efficiency]
**Learning:** Cloning a `reqwest::Client` (and its blocking counterpart) is a very cheap operation because they internally use `Arc` to share the connection pool. This makes it efficient to pass clients by value or store them in multiple structs without losing the benefits of connection pooling.
**Action:** When using shared clients, it is perfectly fine to clone them for use in different components or tasks, as long as they originate from the same initial instance to maintain the shared connection pool.

## 2025-05-24 - [Cache Regex and Selectors]
**Learning:** Re-compiling `regex::Regex` and `scraper::Selector` on every function call is a massive performance bottleneck. In `HibikiScraper`, caching these objects using `std::sync::OnceLock` improved extraction performance from ~411µs to ~5.6µs per iteration (73x speedup).
**Action:** Always cache regex and CSS selector objects in static variables using `OnceLock` or `lazy_static` when they are used in hot paths.
