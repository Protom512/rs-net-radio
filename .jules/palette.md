## 2025-05-15 - [Color-coded CLI Summaries]
**Learning:** In CLI tools, pairing colors with symbols (e.g., emojis like ✅, ❌) is CRITICAL for accessibility (color-blindness) and quick scanning. Using ANSI escape codes directly avoids adding external dependencies while providing immediate visual hierarchy.
**Action:** Always combine color coding with descriptive symbols or text to ensure the UI remains functional for all users, and define constants for ANSI codes to maintain readability.

## 2025-05-15 - [Progress Bar Templates]
**Learning:** When using `indicatif` in Rust, the progress bar template MUST explicitly include `{msg}` if you intend to use `finish_with_message` or `set_message`. Without it, the bar will finish but the final status message will be swallowed.
**Action:** Always verify that the progress bar template includes all necessary placeholders for the information you want to display to the user.
