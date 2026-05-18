## 2026-02-23 - [Radiko Channel ID Sanitization]
**Vulnerability:** URL Injection / SSRF in Radiko API calls.
**Learning:** Station IDs were interpolated directly into API URLs without validation. Using `sanitize_filename` is helpful but strict alphanumeric filtering is more appropriate for URL parameters to prevent injection of query parameters (using `&` or `?`) or path traversal.
**Prevention:** Always validate or sanitize externally-sourced identifiers before using them in URL construction. Prefer strict allow-lists (like alphanumeric) for identifiers.

## 2026-03-16 - [Output Path Traversal in Batch/Cron]
**Vulnerability:** Path Traversal in program list and cron configuration.
**Learning:** User-provided `output_path` was accepted as-is from configuration files and used to construct recording paths. This allowed using `../` to traverse and potentially overwrite system files.
**Prevention:** Never trust paths from external input. Use `Path::file_name()` to extract only the intended filename and apply sanitization (like `sanitize_filename`) before prepending the safe base directory.

## 2026-03-20 - [Insecure Temporary File Creation]
**Vulnerability:** Predictable temporary filenames in shared directories.
**Learning:** Multiple modules (`radiko`, `onsen`, `hibiki`) were using `std::env::temp_dir()` to create temporary files. On multi-user systems, this allows attackers to pre-create files or symbolic links to hijack or overwrite sensitive data (Symlink Attack).
**Prevention:** Always use secure temporary directory utilities like `tempdir` or `tempfile`. These crates create randomized subdirectories with restricted permissions (0700) and ensure safe cleanup via RAII.
