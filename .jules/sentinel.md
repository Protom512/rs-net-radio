## 2024-07-26 - [Unplanned Process Termination in Library]
**Vulnerability:** Library utility functions called `std::process::exit` on HTTP errors (403/429) and HTML parsing failures.
**Learning:** Using `exit()` in a library is a Denial of Service (DoS) risk and poor architectural practice. It prevents the host application from handling errors gracefully and can terminate unrelated tasks (e.g., in a batch or scheduled recording).
**Prevention:** Library functions should always return `Result` or custom error types, allowing the caller to decide on the exit strategy or recovery.
