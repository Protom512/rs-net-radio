## 2026-02-23 - [Radiko Channel ID Sanitization]
**Vulnerability:** URL Injection / SSRF in Radiko API calls.
**Learning:** Station IDs were interpolated directly into API URLs without validation. Using `sanitize_filename` is helpful but strict alphanumeric filtering is more appropriate for URL parameters to prevent injection of query parameters (using `&` or `?`) or path traversal.
**Prevention:** Always validate or sanitize externally-sourced identifiers before using them in URL construction. Prefer strict allow-lists (like alphanumeric) for identifiers.

## 2026-03-16 - [Output Path Traversal in Batch/Cron]
**Vulnerability:** Path Traversal in program list and cron configuration.
**Learning:** User-provided `output_path` was accepted as-is from configuration files and used to construct recording paths. This allowed using `../` to traverse and potentially overwrite system files.
**Prevention:** Never trust paths from external input. Use `Path::file_name()` to extract only the filename component and apply sanitization (like `sanitize_filename`) before prepending the safe base directory.

## 2026-03-24 - [Uncontrolled Process Exit in Library]
**Vulnerability:** Uncontrolled process termination in library code (Availability/DoS).
**Learning:** Library utility functions (`check_http_status`, `handle_html_parsing_error`) utilized `std::process::exit`. This prevented consuming applications from handling errors (like 429 Rate Limiting) gracefully and allowed remote servers to terminate the entire application process via specific HTTP responses.
**Prevention:** Library code must NEVER call `std::process::exit`. Always propagate errors using `Result` or `RecordError` to allow the calling application to determine the appropriate failure strategy (e.g., retry, log, or terminate).
