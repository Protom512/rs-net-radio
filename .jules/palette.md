## 2025-05-15 - Pairing Colors with Symbols for CLI Accessibility
**Learning:** Pairing ANSI colors with symbols (e.g., emojis like ✅, ❌, ⚠️) ensures that status information is accessible to users with color vision deficiencies and remains clear even in environments where color support might be limited or disabled.
**Action:** Always include textual or symbolic indicators alongside color changes when conveying state or status in the CLI.

## 2025-05-15 - Progress Bar Template Design
**Learning:** In long-running batch processes, removing the ETA ({eta}) in favor of a status message ({msg}) can be perceived as a UX regression. Users value knowing how much time is left.
**Action:** Use templates that include both {eta} and {msg} placeholders to provide a comprehensive progress overview.
