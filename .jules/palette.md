# Palette's Journal

## 2026-02-09 - [CLI Validation and Feedback]
**Learning:** For CLI tools, environment variable validation should occur after argument parsing to allow `--help` to function. Informative error messages with examples and colorized summaries significantly improve user self-correction and scanning efficiency.
**Action:** Always check for required environment variables early in `main` but after `Args::parse()`. Use ANSI escape codes and emojis for visual hierarchy in summaries.
