## 2026-02-23 - [Radiko Channel ID Sanitization]
**Vulnerability:** URL Injection / SSRF in Radiko API calls.
**Learning:** Station IDs were interpolated directly into API URLs without validation. Using `sanitize_filename` is helpful but strict alphanumeric filtering is more appropriate for URL parameters to prevent injection of query parameters (using `&` or `?`) or path traversal.
**Prevention:** Always validate or sanitize externally-sourced identifiers before using them in URL construction. Prefer strict allow-lists (like alphanumeric) for identifiers.
