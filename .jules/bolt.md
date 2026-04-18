## 2025-05-22 - [Reuse reqwest Clients]
**Learning:** In Rust applications, creating a new `reqwest::Client` for every request is a major performance bottleneck due to the cost of repeatedly establishing TCP and TLS connections. Reusing a single `Client` (or one per thread/purpose) enables connection pooling, significantly reducing latency and resource usage.
**Action:** Always prefer a shared, static, or long-lived `reqwest::Client` instance over local instantiation. Use `std::sync::OnceLock` for lazy, thread-safe initialization of shared clients in Rust.

## 2025-05-23 - [reqwest::Client Cloning Efficiency]
**Learning:** Cloning a `reqwest::Client` (and its blocking counterpart) is a very cheap operation because they internally use `Arc` to share the connection pool. This makes it efficient to pass clients by value or store them in multiple structs without losing the benefits of connection pooling.
**Action:** When using shared clients, it is perfectly fine to clone them for use in different components or tasks, as long as they originate from the same initial instance to maintain the shared connection pool.

## 2026-04-18 - [Extraction Order in Scrapers]
**Learning:** In HTML scrapers that use multiple methods (attributes, iframes, script parsing), the order of execution significantly impacts performance. Parsing large numbers of `<script>` tags and running multiple regexes on their content is much more expensive than looking up DOM attributes.
**Action:** Always prioritize simple attribute and element lookups before falling back to heavy text-based parsing and regex matching on script contents.
