# Palette's Journal - CRITICAL UX/accessibility learnings

## 2025-05-22 - Colorized terminal output for batch summaries
**Learning:** Terminal applications can be dense and difficult to parse. Using ANSI colors and symbols helps users quickly identify successes and failures in batch operations.
**Action:** Use standard ANSI escape codes for colorization and pair them with emojis to ensure accessibility for color-blind users.

## 2025-05-22 - Rust macro capturing limitations with constants
**Learning:** In Rust (1.58+), `println!` and `format!` macros can capture variables from the surrounding scope, but they cannot directly capture `const` identifiers within the format string (e.g., `"{CONST}"`).
**Action:** To use constants as named captures in format strings, assign them to local variables first or pass them as explicit named arguments.
