# Sentinel's Journal

## 2026-02-16 - Library Process Exit and Insecure HTTP
**Vulnerability:** Use of `std::process::exit` in library functions and insecure HTTP for API calls.
**Learning:** Calling `std::process::exit` in a library is a form of local Denial of Service as it prevents the host application from handling errors gracefully (e.g., continuing a batch process). Also, legacy endpoints often still use `http` when `https` is available.
**Prevention:** Always return `Result` from library functions. Favor `https` for all external API calls. Sanitize all variables used in URL construction, even if they seem like internal IDs.
