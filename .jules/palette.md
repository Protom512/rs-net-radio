## 2025-05-15 - [Batch summary colorization and progress bar improvement]
**Learning:** For CLI tools, adding color and bold formatting to summaries significantly improves the scannability of the output. Using dynamic thresholds for colors (e.g., green for 100%, yellow for >=80%) provides immediate visual feedback on the success of a batch operation. Adding `{msg}` to `indicatif` templates is essential for `finish_with_message` to be effective.
**Action:** Always include colorized summaries for batch operations and ensure progress bar templates include the message placeholder.
