## 2025-05-22 - [Reuse reqwest Clients]
**Learning:** In Rust applications, creating a new `reqwest::Client` for every request is a major performance bottleneck due to the cost of repeatedly establishing TCP and TLS connections. Reusing a single `Client` (or one per thread/purpose) enables connection pooling, significantly reducing latency and resource usage.
**Action:** Always prefer a shared, static, or long-lived `reqwest::Client` instance over local instantiation. Use `std::sync::OnceLock` for lazy, thread-safe initialization of shared clients in Rust.

## 2025-05-23 - [reqwest::Client Cloning Efficiency]
**Learning:** Cloning a `reqwest::Client` (and its blocking counterpart) is a very cheap operation because they internally use `Arc` to share the connection pool. This makes it efficient to pass clients by value or store them in multiple structs without losing the benefits of connection pooling.
**Action:** When using shared clients, it is perfectly fine to clone them for use in different components or tasks, as long as they originate from the same initial instance to maintain the shared connection pool.

## 2025-05-24 - [Pre-compiling Regex and Selectors]
**Learning:** Recompiling  or  in a loop or a frequently called function is a major performance bottleneck. Compiling them once and storing them in a  variable with  can lead to massive speedups (e.g., 400x+).
**Action:** Always pre-compile regexes and CSS selectors used in scrapers or hot paths. Use  for lazy, thread-safe initialization.

## 2025-05-24 - [Pre-compiling Regex and Selectors]
**Learning:** Recompiling `regex::Regex` or `scraper::Selector` in a loop or a frequently called function is a major performance bottleneck. Compiling them once and storing them in a `static` variable with `std::sync::OnceLock` can lead to massive speedups (e.g., 400x+).
**Action:** Always pre-compile regexes and CSS selectors used in scrapers or hot paths. Use `OnceLock` for lazy, thread-safe initialization.
