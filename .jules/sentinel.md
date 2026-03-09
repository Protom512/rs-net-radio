## 2026-02-23 - [Radiko Channel ID Sanitization]
**Vulnerability:** URL Injection / SSRF in Radiko API calls.
**Learning:** Station IDs were interpolated directly into API URLs without validation. Using `sanitize_filename` is helpful but strict alphanumeric filtering is more appropriate for URL parameters to prevent injection of query parameters (using `&` or `?`) or path traversal.
**Prevention:** Always validate or sanitize externally-sourced identifiers before using them in URL construction. Prefer strict allow-lists (like alphanumeric) for identifiers.

## 2026-02-23 - [Path Traversal in Program List and Cron Schedules]
**Vulnerability:** Path traversal in `output_path`.
**Learning:** External inputs used to construct file paths were not sanitized, allowing writing to arbitrary locations. Using `Path::file_name()` to extract only the filename part is an effective way to prevent traversal while still allowing users to specify the output filename.
**Prevention:** Always sanitize externally-sourced paths by stripping directory components and using allow-lists for characters if possible.
