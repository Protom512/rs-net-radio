## 2026-02-23 - [Radiko Channel ID Sanitization]
**Vulnerability:** URL Injection / SSRF in Radiko API calls.
**Learning:** Station IDs were interpolated directly into API URLs without validation. Using `sanitize_filename` is helpful but strict alphanumeric filtering is more appropriate for URL parameters to prevent injection of query parameters (using `&` or `?`) or path traversal.
**Prevention:** Always validate or sanitize externally-sourced identifiers before using them in URL construction. Prefer strict allow-lists (like alphanumeric) for identifiers.

## 2026-03-16 - [Output Path Traversal in Batch/Cron]
**Vulnerability:** Path Traversal in program list and cron configuration.
**Learning:** User-provided `output_path` was accepted as-is from configuration files and used to construct recording paths. This allowed using `../` to traverse and potentially overwrite system files.
**Prevention:** Never trust paths from external input. Use `Path::file_name()` to extract only the intended filename and apply sanitization (like `sanitize_filename`) before prepending the safe base directory.

## 2026-04-10 - [Library-Induced Denial of Service (DoS)]
**Vulnerability:** Library functions calling `std::process::exit` based on external server responses.
**Learning:** Functions in `record-lib` would terminate the entire host process when encountering HTTP 403/429 or HTML parsing errors. This allowed external radio services to effectively "shut down" the recording application by returning specific status codes, creating a self-induced DoS vulnerability and making the application unrecoverable during batch processing.
**Prevention:** Library code must NEVER terminate the host process. Always propagate errors using `Result` or custom error types, allowing the calling application to decide how to handle failures (e.g., skip a single program in a batch instead of crashing the entire run).
