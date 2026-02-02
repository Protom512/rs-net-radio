## 2025-05-22 - [Reuse reqwest Clients]
**Learning:** In Rust applications, creating a new `reqwest::Client` for every request is a major performance bottleneck due to the cost of repeatedly establishing TCP and TLS connections. Reusing a single `Client` (or one per thread/purpose) enables connection pooling, significantly reducing latency and resource usage.
**Action:** Always prefer a shared, static, or long-lived `reqwest::Client` instance over local instantiation. Use `std::sync::OnceLock` for lazy, thread-safe initialization of shared clients in Rust.
