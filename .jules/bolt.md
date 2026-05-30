## 2025-05-22 - [Reuse reqwest Clients]
**Learning:** In Rust applications, creating a new `reqwest::Client` for every request is a major performance bottleneck due to the cost of repeatedly establishing TCP and TLS connections. Reusing a single `Client` (or one per thread/purpose) enables connection pooling, significantly reducing latency and resource usage.
**Action:** Always prefer a shared, static, or long-lived `reqwest::Client` instance over local instantiation. Use `std::sync::OnceLock` for lazy, thread-safe initialization of shared clients in Rust.

## 2025-05-23 - [reqwest::Client Cloning Efficiency]
**Learning:** Cloning a `reqwest::Client` (and its blocking counterpart) is a very cheap operation because they internally use `Arc` to share the connection pool. This makes it efficient to pass clients by value or store them in multiple structs without losing the benefits of connection pooling.
**Action:** When using shared clients, it is perfectly fine to clone them for use in different components or tasks, as long as they originate from the same initial instance to maintain the shared connection pool.

## 2025-05-24 - [Optimizing HTML Scraping]
**Learning:** Performance of HTML scraping in Rust can be significantly improved by:
1. **Prioritizing Search Order:** Reorder extraction methods to try fast attribute-based lookups before expensive regex matching on large script tags.
2. **Batching Selectors:** Combine multiple CSS selectors into a single comma-separated selector (e.g., `[data-url], [data-src]`) to reduce document traversals.
3. **Smart Joining:** Avoid unconditional `collect().join()` on text nodes. Checking if only one node exists allows processing it directly, saving allocations.
4. **Fast-path Short-circuiting:** Adding a simple `.contains()` check before executing a complex `Regex` can drastically speed up the negative case (when the pattern definitely won't match).
**Action:** Always consider search order and document traversal count in scraping logic. Use simple string checks to short-circuit regex when possible.
