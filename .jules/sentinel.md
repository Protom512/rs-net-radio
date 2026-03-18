## 2026-02-23 - [Radiko Channel ID Sanitization]
**Vulnerability:** URL Injection / SSRF in Radiko API calls.
**Learning:** Station IDs were interpolated directly into API URLs without validation. Using `sanitize_filename` is helpful but strict alphanumeric filtering is more appropriate for URL parameters to prevent injection of query parameters (using `&` or `?`) or path traversal.
**Prevention:** Always validate or sanitize externally-sourced identifiers before using them in URL construction. Prefer strict allow-lists (like alphanumeric) for identifiers.

## 2026-02-24 - [Path Traversal in Recording Output Paths]
**Vulnerability:** Path Traversal via user-provided `output_path`.
**Learning:** Simply using `Path::file_name()` to mitigate path traversal is too restrictive as it prevents users from using legitimate subdirectories. A better approach is to iterate through `Path::components()` and only allow `Normal` components, which naturally strips `..`, root, and prefixes.
**Prevention:** When processing user-provided paths that should stay within a specific directory, reconstruct the path component-by-component, allowing only `Normal` components and sanitizing each one.
