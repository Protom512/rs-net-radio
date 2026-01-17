# Contributing to rs-net-radio

Thank you for your interest in contributing to rs-net-radio! This document provides guidelines and instructions for contributing.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Workflow](#development-workflow)
- [Coding Standards](#coding-standards)
- [Submitting Changes](#submitting-changes)

## Code of Conduct

Be respectful, inclusive, and collaborative. Treat others as you would want to be treated.

## Getting Started

### Prerequisites

- Rust toolchain (latest stable version)
- Git

### Setup

```bash
# Clone the repository
git clone https://github.com/protom512/rs-net-radio.git
cd rs-net-radio

# Build the project
cargo build
```

## Development Workflow

1. **Fork and Branch**
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make Changes**
   - Write clear, concise code
   - Add tests for new functionality
   - Update documentation

3. **Test**
   ```bash
   # Run tests
   cargo test

   # Run clippy for lints
   cargo clippy -- -D warnings

   # Format code
   cargo fmt
   ```

## Coding Standards

### Rust Conventions

- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` for formatting
- Address all `clippy` warnings
- Write documentation comments (`///` and `//!`)

### Commit Messages

Use clear, descriptive commit messages:

```
feat: add new radio stream parser
fix: correct buffer overflow in audio decoder
docs: update README with usage examples
test: add integration tests for recording module
```

## Submitting Changes

1. Ensure all tests pass
2. Update documentation if needed
3. Create a pull request with:
   - Clear description of changes
   - Reference to related issues
   - Testing instructions

### Pull Request Checklist

- [ ] Code follows project style guidelines
- [ ] Tests pass locally
- [ ] Documentation is updated
- [ ] Commit messages are clear
- [ ] PR description is complete

## Getting Help

- Open an issue for bugs or feature requests
- Start a discussion for questions
- Check existing documentation first

Thank you for contributing!
