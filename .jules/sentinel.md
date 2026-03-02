## 2026-02-23 - [Radiko Channel ID Sanitization]
**Vulnerability:** URL Injection / SSRF in Radiko API calls.
**Learning:** Station IDs were interpolated directly into API URLs without validation. Using `sanitize_filename` is helpful but strict alphanumeric filtering is more appropriate for URL parameters to prevent injection of query parameters (using `&` or `?`) or path traversal.
**Prevention:** Always validate or sanitize externally-sourced identifiers before using them in URL construction. Prefer strict allow-lists (like alphanumeric) for identifiers.

## 2026-03-02 - [Radiko API HTTPS Upgrade]
**Vulnerability:** Eavesdropping / MITM on Radiko API communication.
**Learning:** Some API endpoints were still using `http` instead of `https`, potentially exposing station XML data and program schedules to eavesdropping or tampering.
**Prevention:** Always use `https` for API communication to ensure data is encrypted in transit and the server's identity is verified.
