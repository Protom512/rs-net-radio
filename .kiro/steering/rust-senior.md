

---

# Rust Best Practices & Architecture Rules

## 1. De Facto Ecosystem (The Standard Stack)
Unless explicitly instructed otherwise, assume these libraries are the standard solution. Do not hallucinate obscure alternatives.

- **Async Runtime**: `tokio` (The industry standard). Avoid `async-std` unless requested.
- **Web Framework**: `axum` (First choice for modularity) or `actix-web` (Performance).
- **Serialization**: `serde` (with `derive` feature).
- **Error Handling**: 
  - Application layer: `anyhow` (for easy error propagation).
  - Library layer: `thiserror` (for typed, meaningful errors).
- **Logging/Tracing**: `tracing` & `tracing-subscriber`. Avoid `log` crate directly.
- **Database**: `sqlx` (Compile-time checked SQL) or `diesel` (ORM).
- **HTTP Client**: `reqwest`.
- **CLI Parsing**: `clap` (derive pattern).

## 2. Rust-Specific Abstraction Patterns
Rust implies **Composition over Inheritance**.

### A. The Newtype Pattern
Use tuple structs to create distinct types from primitives to ensure type safety.
- **Good**: `struct UserId(uuid::Uuid);`
- **Why**: Prevents passing a `GroupId` where a `UserId` is expected.

### B. The Builder Pattern
Use for constructing complex structs with optional configurations.
- **Rule**: If a struct has more than 3 `Option<T>` fields, implement a Builder or use `derive_builder`.
- **Why**: Keeps call sites clean and allows for validation during construction.

### C. The Typestate Pattern (Advanced)
Encode state in the type system to make invalid states unrepresentable.
- **Example**: `Order<Pending>` -> `Order<Paid>` -> `Order<Shipped>`.
- **Why**: It becomes compile-time impossible to ship an unpaid order.

### D. From/Into Traits
Prefer implementing `From<T>` over defining ad-hoc conversion methods like `to_user()`.
- **Rule**: Implement `From` (which gives `Into` for free).
- **Why**: Enables idiomatic `.into()` conversions.

## 3. Error Handling Strategy
- **Panic is Failure**: Never panic in production code. Panics are only for unrecoverable bugs (assertions).
- **Result Propagation**: Use the `?` operator extensively.
- **Context**: When using `anyhow`, always attach context:
  ```rust
  // BAD
  file.open()?;
  
  // GOOD
  file.open().context("Failed to open configuration file")?;
  ```

## 4. Performance & Memory Guidelines
- **Zero-Cost Abstractions**: Use Iterators (`.map()`, `.filter()`, `.collect()`) over raw `for` loops where readable. Rust optimizes iterators better than manual loops.
- **Allocation Awareness**:
  - Avoid `String` cloning in loops.
  - Use `&str` for function arguments if ownership isn't needed.
  - Use `Cow<'a, str>` (Clone-on-Write) if you might return a borrowed string OR an owned string.
- **Boxed Traits**: Use `Box<dyn Trait>` for dynamic dispatch only when necessary (e.g., heterogeneous collections). Prefer Generics `fn foo<T: Trait>(item: T)` (static dispatch) for performance.

## 5. Anti-Patterns (STRICTLY PROHIBITED)
AI must verify code against these anti-patterns before outputting.

### X. The "Clone to Fix Borrow Checker"
- **Anti-Pattern**: Randomly adding `.clone()` until the compiler shuts up.
- **Correction**: Analyze the lifetime. Can you use a reference? Can you wrap it in `Arc`?

### Y. Stringly Typed Programming
- **Anti-Pattern**: Passing `String` everywhere for IDs, Statuses, or Enums.
- **Correction**: Use `enum`, `Newtype`, or strong types.

### Z. Self-Referential Structs
- **Anti-Pattern**: Creating a struct that holds a reference to one of its own fields.
- **Correction**: This is extremely hard in Rust. Redesign the data structure. Use handles/indices or `Rc`/`Arc`.

### W. Excessive `RefCell` / `Mutex`
- **Anti-Pattern**: Wrapping everything in `Arc<Mutex<T>>` because you don't understand borrowing.
- **Correction**: Redesign ownership flow. Mutexes are for shared state across threads, not for bypassing the borrow checker in a single thread.

---

## 6. Project Structure Guidelines (Modular Monolith)
Follow the "Separation of Concerns" strictly.

- `src/main.rs`: Entry point, setup tracing, dependency injection only.
- `src/lib.rs`: Exposes modules.
- `src/domain/`: Core business logic, pure Rust, no DB/HTTP dependencies.
- `src/infra/`: Database implementations, external API calls.
- `src/api/`: DTOs, Handlers (Axum/Actix), Routers.
- `src/shared/`: Utilities common across layers.

---
