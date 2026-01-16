# AI Coding Agent Instructions for rs-net-radio

This guide helps AI agents understand the architecture, workflows, and conventions of the rs-net-radio project—a Rust-based system for recording internet radio streams from Japanese services (Radiko, Onsen, Hibiki, etc.).

## Project Architecture

**rs-net-radio** is a workspace with two main crates:

1. **`record-lib`** - Core library providing recording logic via service-specific modules
   - `record::onsen` - Onsen.ag radio recording
   - `record::hibiki` - Hibiki-radio.jp recording
   - `record::radiko` - Radiko.jp recording
   - `utils` - Error handling, command execution, archiving
   - `common` - Async command execution and ffmpeg wrapper

2. **`app`** - CLI application using tokio-cron-scheduler to orchestrate recordings
   - Main scheduler entry point
   - Cron-based job definitions for each service

### Key Design Pattern: Service Modules + Trait

Each recording service (onsen, hibiki, radiko) implements similar patterns:
- Fetch/parse service-specific API data structures (JSON/XML)
- Download streams using ffmpeg via `execute_command()`
- Store recordings to `RS_NET_ARCHIVE_PATH/{service_name}/`

The `Record` trait in `record.rs` defines the interface (though not all services fully implement it yet).

### Error Handling Philosophy

Custom `RecordError` enum (in `utils.rs`) with variants for:
- I/O errors (`Io`)
- Environment variables (`EnvVar`)
- HTTP requests (`Reqwest`)
- JSON parsing (`SerdeJson`)
- External command failures (`CommandFailed { command, exit_code, stderr }`)
- Generic catch-all (`Other`)

All errors implement `From` traits for ergonomic `?` operator usage. Services must return `Result<T, RecordError>`.

## Critical Developer Workflows

### Building and Testing

```bash
# Build the entire workspace (release mode recommended for CLI)
cargo build --release

# Run library tests
cargo test --lib

# Run integration tests (e.g., scheduler tests in app/tests/)
cargo test --test "*"

# Run a specific example (if available)
cargo run --example hibiki-toybox
```

### Running the Application

```bash
# Set the archive directory before running
$env:RS_NET_ARCHIVE_PATH = "C:\path\to\archive"  # Windows PowerShell
export RS_NET_ARCHIVE_PATH="/path/to/archive"     # Linux/macOS

# Run the scheduler daemon
cargo run --release
```

### Key Environment Variable

- **`RS_NET_ARCHIVE_PATH`** - Root directory for recording storage. Each service creates a subdirectory automatically via `ensure_archive_path(service_name)`.

## Code Style and Patterns

### Logging
- Use `log` crate macros: `info!()`, `warn!()`, `error!()`, `debug!()`
- App configures logging via `env_logger` in `main.rs`
- Common pattern: log at job entry, command execution, success, and errors

### Async Runtime
- `tokio` runtime with `full` features enabled in app
- Scheduler jobs are synchronous closures; use `tokio::runtime::Runtime::new()?.block_on()` to run async code inside (see `job_radiko` in `main.rs`)
- Keep timeout handling explicit (e.g., `tokio::time::timeout()`)

### Error Propagation
- Prefer `?` operator with `Result<T, RecordError>`
- Convert external errors via `From` impl (e.g., `Command::new(...).spawn().map_err(|e| RecordError::Io(e))?`)
- Never `.unwrap()` in library code; use `.ok_or(RecordError::Other(...))?` or explicit error mapping

### File Operations
- Use `sanitize_filename` crate for safe filenames: `sanitize_filename::sanitize(name)`
- Path separators: use `/` or `Path::new()` for cross-platform compatibility
- Check existence before overwriting: `if path.exists() { warn!(...); continue; }`

### External Command Execution
- Always use `execute_command()` from `common.rs` for async execution with timeout support
- ffmpeg recording wrapper: `record_with_ffmpeg(url, output_path, duration).await?`
- Timeout is duration + 60s buffer to prevent premature kills

## Integration Points and Data Flows

### Service Data Fetching → Recording → Archiving

1. **Service initialization** (e.g., `OnsenProgram::init()`)
   - Calls service API (HTTP)
   - Parses JSON/XML into typed structs
   - Returns `Vec<Program>` or `Program` with downloadable content

2. **Download execution** (e.g., `OnsenProgram::record()`)
   - Determines output filename (sanitized)
   - Checks archive path; skip if exists
   - Calls `execute_command("ffmpeg", ...)` with URL and duration
   - Moves temp file to archive

3. **Scheduler orchestration** (in `app/main.rs`)
   - `job_onsen()`, `job_radiko()`, `job_hibiki()` each spawn a cron job
   - Each job fetches current programs and downloads available content
   - Errors logged but don't stop other jobs

### Cross-Module Dependencies

```
app/ → record-lib/
  └─ record.rs (trait definition)
     ├─ record/onsen.rs (OnsenProgram struct + methods)
     ├─ record/hibiki.rs (HibikiJson + record fn)
     ├─ record/radiko.rs (RecordRadiko struct + methods)
     └─ common.rs (execute_command, record_with_ffmpeg)
  └─ utils.rs (RecordError, ensure_archive_path)
```

## Testing Conventions

- **Unit tests**: Inline in modules or `#[cfg(test)] mod tests { ... }`
- **Integration tests**: In `app/tests/` using tokio test harness (`#[tokio::test]`)
- **Mocking**: `mockito` crate for HTTP mocking (see dev-dependencies)
- **Example pattern** (scheduler_tests.rs): Use `tokio::sync::mpsc` channels to verify async job execution

## Docker and Deployment

- `Dockerfile` uses multi-stage build (Rust builder → Alpine runtime)
- Alpine adds `ffmpeg` as runtime dependency
- Binary location: `/usr/local/bin/rs-net-radio`
- Docker expects `RS_NET_ARCHIVE_PATH` to be mounted as volume

## Special Notes for AI Agents

1. **Incomplete implementations**: Radiko and Hibiki support are "under review" (see README). Exercise caution with these modules during major refactors.

2. **Japanese content**: Service APIs return Japanese text; filenames are sanitized to remove spaces/special characters. Preserve any existing Japanese handling in struct fields.

3. **API stability**: Services (Onsen, Radiko, Hibiki) may change API endpoints. URL parsing is hardcoded in each service module; API changes require targeted module updates.

4. **Process management**: Scheduler holds `JobScheduler` instances with `record_sched.start().await`. Ensure graceful shutdown on app termination.

5. **Workspace member interaction**: When modifying both `record-lib` and `app`, remember they're workspace members; `cargo build` compiles both. Test with `cargo test` from workspace root.

## When Adding New Services

1. Create `record-lib/src/record/{service_name}.rs`
2. Define typed structs for API responses (with serde)
3. Implement fetch logic (HTTP via `reqwest`)
4. Implement recording logic using `execute_command()` or `record_with_ffmpeg()`
5. Return `Result<(), RecordError>` or `Result<ExitStatus, RecordError>`
6. Add module declaration in `record-lib/src/record.rs`
7. Create corresponding job function in `app/src/main.rs`
8. Add job to scheduler in `main()`

## Development Setup Notes

- Rust edition: 2018
- Target: `x86_64-unknown-linux-musl` (for Docker; see Dockerfile builder)
- Local development: Standard Rust toolchain
- External tools required: `ffmpeg` (see README)
