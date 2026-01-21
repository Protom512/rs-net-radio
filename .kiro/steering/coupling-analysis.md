# Coupling Analysis with cargo-coupling

## Overview

`cargo-coupling` is a tool for analyzing coupling in Rust projects based on Vlad Khononov's "Balancing Coupling in Software Design" framework. It evaluates coupling across three dimensions and calculates a Balance Score to guide refactoring decisions.

## Installation

```bash
cargo install cargo-coupling
```

Or use Docker:
```bash
docker pull ghcr.io/nwiizo/cargo-coupling
```

## Quick Start

```bash
# Basic analysis (shows important issues only)
cargo coupling ./src

# Summary mode
cargo coupling --summary ./src

# Japanese output
cargo coupling --summary --japanese ./src

# Show all issues including Low severity
cargo coupling --summary --all ./src

# Interactive web UI (experimental)
cargo coupling --web ./src
```

## The Three Dimensions of Coupling

### 1. Integration Strength (結合強度)

How tightly components depend on each other.

| Level | Description | Rust Example | Score |
|-------|-------------|--------------|-------|
| **Contract** | Dependency on interfaces only | Access via `trait` | 0.25 (weak) |
| **Model** | Dependency on data structures | Sharing type definitions | 0.50 |
| **Functional** | Dependency on behavior | Method calls on concrete types | 0.75 |
| **Intrusive** | Direct dependency on implementation | Direct access to `struct.field` | 1.00 (strong) |

**Guideline**: Prefer trait-based (Contract) coupling over direct implementation access.

### 2. Distance (距離)

The physical or logical distance between dependent components.

| Level | Description | Score |
|-------|-------------|-------|
| **Same Module** | Within the same module | 0.25 (close) |
| **Different Module** | Different module in the same crate | 0.50 |
| **External Crate** | Dependency on external crate | 1.00 (far) |

**Guideline**: Strong coupling should be kept at close distance.

### 3. Volatility (変更頻度)

How frequently a component changes (calculated from Git history).

| Level | Description | Changes (6 months) | Score |
|-------|-------------|-------------------|-------|
| **Low** | Stable, rarely changes | 0-2 times | 0.00 |
| **Medium** | Occasionally changes | 3-10 times | 0.50 |
| **High** | Frequently changes | 11+ times | 1.00 |

**Guideline**: High volatility components should be isolated with stable abstraction layers.

## Khononov's Balance Law

```
BALANCED = (STRENGTH ≤ threshold) OR (DISTANCE = near) OR (VOLATILITY = low)
```

Or equivalently:
```
BALANCE = (STRENGTH XOR DISTANCE) OR NOT VOLATILITY
```

### Design Decision Matrix

| Strength | Distance | Volatility | Decision |
|----------|----------|------------|----------|
| Strong | Close | Low-Medium | OK - High cohesion |
| Weak | Far | Any | OK - Loose coupling |
| Strong | Far | Any | Needs improvement - Global complexity |
| Strong | Any | High | Needs improvement - Cascading change risk |
| Weak | Close | Low | Consider - May be over-modularized |

## Health Grades

| Grade | Description | Criteria |
|-------|-------------|----------|
| **S** | Over-optimized! | Medium density <= 5% with >= 20 couplings |
| **A** | Well-balanced | Medium density 5-10%, no high issues |
| **B** | Healthy | Medium density > 10%, no critical issues |
| **C** | Room for improvement | Any high issues OR medium density > 25% |
| **D** | Attention needed | Any critical issues OR high density > 5% |
| **F** | Immediate action required | More than 3 critical issues |

**Note**: S is a warning, not a reward. Aim for A.

## Job-Focused Commands

```bash
# Find top refactoring targets
cargo coupling --hotspots ./src
cargo coupling --hotspots=10 ./src

# With beginner-friendly explanations
cargo coupling --hotspots --verbose ./src

# Analyze change impact for a specific module
cargo coupling --impact main ./src

# Trace dependencies for a specific function or type
cargo coupling --trace BalanceScore ./src

# CI/CD quality gate (exits with code 1 on failure)
cargo coupling --check --min-grade=B ./src
cargo coupling --check --max-circular=0 ./src

# Machine-readable JSON output
cargo coupling --json ./src

# AI-friendly output (for Claude, Copilot, etc.)
cargo coupling --ai ./src
```

## Improvement Patterns

### Pattern 1: Reducing Coupling Strength via Abstraction

**Problem**: Strong coupling + far distance

```rust
// BEFORE: Intrusive coupling across modules
// module_a.rs
fn process_user(user: &User) {
    let name = &user.name;      // Direct field access
    let email = &user.email_address;
}

// module_b.rs
pub struct User {
    pub name: String,
    pub email_address: String,
}
```

**Solution**: Introduce a Contract (trait)

```rust
// AFTER: Contract-based coupling
// contracts.rs (stable layer)
pub trait UserInfo {
    fn display_name(&self) -> &str;
    fn contact_email(&self) -> &str;
}

// module_b.rs
impl UserInfo for User {
    fn display_name(&self) -> &self.name }
    fn contact_email(&self) -> &self.email_address }
}

// module_a.rs
fn process_user(user: &impl UserInfo) {
    let name = user.display_name();    // Contract coupling
    let email = user.contact_email();
}
```

### Pattern 2: Isolating Volatility

**Problem**: Strong coupling + high volatility

**Solution**: Insert a stable interface layer between volatile components.

## CI/CD Integration

```yaml
# .github/workflows/coupling.yml
name: Coupling Analysis

on: [push, pull_request]

jobs:
  analyze:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0  # Full history for volatility analysis

      - name: Install cargo-coupling
        run: cargo install cargo-coupling

      - name: Run coupling analysis
        run: cargo coupling --summary --timing ./src

      - name: Quality gate check
        run: cargo coupling --check --min-grade=C --max-circular=0 ./src

      - name: Generate report
        run: cargo coupling -o coupling-report.md ./src
```

## Best Practices

### Good: Strong Coupling at Close Distance

```rust
mod user_profile {
    pub struct User { /* ... */ }
    pub struct UserProfile { /* ... */ }

    impl User {
        pub fn get_profile(&self) -> &UserProfile { /* ... */ }
    }
}
```

### Good: Weak Coupling at Far Distance

```rust
// core/src/lib.rs
pub trait NotificationService {
    fn send(&self, message: &str) -> Result<()>;
}

// adapters/email/src/lib.rs
impl NotificationService for EmailService { /* ... */ }
```

### Bad: Strong Coupling at Far Distance

```rust
// src/api/handlers.rs
impl Handler {
    fn handle(&self) {
        // Direct dependency on internal implementation
        let result = database::internal::execute_raw_sql(...);
    }
}
```

### Bad: Circular Dependencies

```rust
// module_a.rs
use crate::module_b::TypeB;  // Creates cycle

// module_b.rs
use crate::module_a::TypeA;  // Creates cycle
```

## Configuration File

Create `.coupling.toml` in your project root:

```toml
# Volatility overrides for specific files
[volatility]
"src/core/*" = "low"      # Override as low volatility
"src/experimental/*" = "high"  # Override as high volatility
```

## Using with AI Assistants

When refactoring with AI (Claude, Copilot, etc.):

1. Generate AI-friendly output:
   ```bash
   cargo coupling --ai ./src > coupling-analysis.txt
   ```

2. Use this prompt with the AI:
   ```
   The following is the output of `cargo coupling --ai`, which analyzes coupling issues in a Rust project.
   For each issue, suggest specific code changes to reduce coupling.
   Focus on introducing traits, moving code closer, or breaking circular dependencies.
   ```

3. Paste the coupling-analysis.txt content

## Limitations

- This tool is a measurement aid, not an absolute authority
- Cannot understand business context - some "problematic" patterns may be intentional
- Does not replace human judgment and code review
- External crate dependencies are excluded from health grade calculation
- Git history affects volatility analysis

## References

- [cargo-coupling GitHub](https://github.com/nwiizo/cargo-coupling)
- [cargo-coupling on crates.io](https://crates.io/crates/cargo-coupling)
- Vlad Khononov - "Balancing Coupling in Software Design"
