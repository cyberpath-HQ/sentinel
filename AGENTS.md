# 🤖 AGENTS.md - Development Guidelines for Cyberpath Sentinel

This file contains build commands, code style guidelines, and development patterns for agentic coding agents working on the Cyberpath Sentinel codebase.

## 📋 Build, Test, and Lint Commands

### Core Commands (run from workspace root)
```bash
# Build all workspace members
cargo build

# Build with optimizations
cargo build --release

# Run all tests across workspace
cargo test

# Run tests for specific crate
cargo test -p sentinel-dbms
cargo test -p sentinel-crypto
cargo test -p sentinel-cli

# Run single test
cargo test test_name
cargo test -- test_name  # explicit filter

# Run tests with specific features
cargo test --all-features

# Format all code
cargo fmt --all

# Lint with clippy (workspace level)
cargo clippy --all-features -- -D warnings

# Lint specific crate
cargo clippy -p sentinel-dbms --all-features

# Run benchmarks
cargo bench
cargo bench -p sentinel-dbms
cargo bench -p sentinel-crypto
```

### Development Workflow Commands
```bash
# Check code without building
cargo check
cargo check --all-features

# Clean build artifacts
cargo clean

# Update dependencies
cargo update

# Generate documentation
cargo doc --open
cargo doc --all --open
```

## 🎨 Code Style Guidelines

### Imports and Dependencies
- **External crates**: Group at top, use `use` statements
- **Internal modules**: Use `crate::` prefix for internal imports
- **Re-exports**: Public API items should be re-exported from lib.rs
- **Std library**: Import specific items, avoid `use std::*;`

```rust
// External crates first
use chrono::{DateTime, Utc};
use serde_json::Value;
use tokio::fs;

// Internal modules with crate:: prefix
use crate::document::Document;
use crate::error::{Result, SentinelError};

// Standard library
use std::path::Path;
```

### Formatting and Structure
- **Line length**: Maximum 100 characters
- **Indentation**: 4 spaces (no tabs)
- **Brace style**: Allman style for functions, K&R for structs
- **Trailing commas**: Always use in multi-line lists/arrays
- **Empty lines**: One between logical sections, two between major sections

```rust
impl Document {
    pub async fn new(
        id: String,
        data: Value,
        private_key: &SigningKey,
    ) -> Result<Self> {
        // Implementation
    }
}
```

### Naming Conventions
- **Types**: `PascalCase` (structs, enums, traits)
- **Functions/Methods**: `snake_case`
- **Constants**: `SCREAMING_SNAKE_CASE`
- **Private fields**: `snake_case` with `pub(crate)` visibility when needed
- **Acronyms**: Keep consistent case (e.g., `Id`, `Uuid`, `Http`)

```rust
pub const META_SENTINEL_VERSION: u32 = 1;

pub struct Document {
    pub(crate) id: String,
    pub(crate) created_at: DateTime<Utc>,
}

impl Document {
    pub async fn create_document(&self) -> Result<()> {
        // Implementation
    }
}
```

### Types and Generics
- **Prefer concrete types** over generics when possible
- **Use `Result<T>`** for fallible functions (from crate error module)
- **Lifetime parameters**: Use explicit lifetimes only when necessary
- **Generic bounds**: Keep them minimal and clear

```rust
pub async fn insert_document(
    &mut self,
    id: String,
    data: Value,
) -> Result<Document> {
    // Implementation
}
```

### Error Handling
- **Use `crate::Result<T>`** as the standard return type
- **Use `?` operator** for error propagation
- **Never use `.unwrap()`** in production code
- **Prefer `thiserror`** for custom error types
- **Include context** in error messages

```rust
use crate::error::{Result, SentinelError};

pub async fn load_document(&self, id: &str) -> Result<Document> {
    let path = self.get_document_path(id);
    let content = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| SentinelError::IoFailed {
            operation: "read_document".to_string(),
            path: path.clone(),
            source: e,
        })?;
    
    serde_json::from_str(&content)?
}
```

### Async/Await Patterns
- **All I/O operations** must be async
- **Use `.await`** directly, avoid blocking calls
- **Prefer async traits** with `async-trait` crate
- **Use tokio runtime** for async operations

```rust
#[async_trait::async_trait]
pub trait Collection {
    async fn insert(&mut self, doc: Document) -> Result<()>;
    async fn get(&self, id: &str) -> Result<Option<Document>>;
}
```

### Documentation Standards
- **Public APIs**: Must have comprehensive doc comments
- **Examples**: Include usage examples in doc comments
- **Parameters**: Document all parameters and return values
- **Panics**: Document when functions can panic

```rust
/// Creates a new document with the given ID and data.
///
/// # Arguments
/// 
/// * `id` - The unique identifier for the document
/// * `data` - The JSON data to store in the document
/// * `private_key` - The signing key for document integrity
///
/// # Returns
/// 
/// Returns a `Result` containing the created `Document` or an error.
///
/// # Examples
/// 
/// ```rust
/// let doc = Document::new("user-123".to_string(), data, &key).await?;
/// ```
pub async fn new(id: String, data: Value, private_key: &SigningKey) -> Result<Self> {
    // Implementation
}
```

## 🧪 Testing Guidelines

### Unit Tests
- **Every function** must have unit tests
- **Test coverage**: Minimum 90% across codebase
- **Edge cases**: Test empty inputs, invalid data, boundary conditions
- **Test naming**: Use descriptive names with `test_` prefix

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_create_document_with_valid_data() {
        // Test implementation
    }

    #[tokio::test]
    async fn test_create_document_with_empty_id_should_fail() {
        // Test implementation
    }
}
```

### Integration Tests
- **Use `tempfile`** for temporary directories
- **Use `serial_test`** for tests that can't run in parallel
- **Clean up** resources in test teardown

### Benchmarking
- **Use `criterion`** for performance benchmarks
- **Test both best-case and worst-case** scenarios
- **Include regression tests** for performance

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_document_creation(c: &mut Criterion) {
    c.bench_function("create_document", |b| {
        b.iter(|| {
            // Benchmark implementation
        })
    });
}
```

## 🏗️ Architecture Patterns

### Module Organization
- **One concern per module**: Keep modules focused
- **Re-export public API**: Use lib.rs for clean public interface
- **Internal modules**: Use `pub(crate)` for internal visibility

### File Structure
```
src/
├── lib.rs              # Public API re-exports
├── mod1.rs             # Module implementation
├── mod2/
│   ├── mod.rs          # Module re-exports
│   ├── submod1.rs      # Submodule implementation
│   └── submod2.rs
├── error.rs            # Error types
└── tests/              # Integration tests
    └── integration_tests.rs
```

### Dependency Management
- **Workspace dependencies**: Define in root Cargo.toml
- **Feature flags**: Use for optional functionality
- **Version consistency**: Use workspace versions for internal crates

## 🔒 Security Guidelines

### Cryptographic Operations
- **Use sentinel-crypto crate** for all crypto operations
- **Never hardcode keys** or secrets
- **Prefer built-in crypto** over custom implementations
- **Validate all inputs** before crypto operations

### File Operations
- **Use safe file APIs** with proper error handling
- **Validate file paths** to prevent directory traversal
- **Use atomic writes** when possible
- **Clean up temporary files**

## 📝 Quality Assurance

### Pre-commit Checklist
- [ ] Code compiles without warnings
- [ ] All tests pass (`cargo test`)
- [ ] Clippy passes without warnings
- [ ] Code is formatted (`cargo fmt`)
- [ ] Documentation is updated
- [ ] Benchmarks pass (if applicable)

### Code Review Points
- **Error handling**: Are all error cases handled?
- **Async safety**: Are there any blocking calls?
- **Security**: Are inputs validated and sanitized?
- **Performance**: Are there obvious performance issues?
- **Documentation**: Is the API well-documented?

## 🚀 Performance Guidelines

### Async Patterns
- **Avoid blocking operations** in async contexts
- **Use streams** for large datasets
- **Batch operations** when possible
- **Consider connection pooling** for external resources

### Memory Management
- **Prefer references** over cloning when possible
- **Use `Cow<str>`** for optional string ownership
- **Avoid large allocations** in hot paths
- **Profile memory usage** regularly

---

## 📚 Additional Resources

- **Project README**: `/README.md`
- **Implementation Plan**: `/IMPLEMENTATION_PLAN.md`
- **Copilot Guidelines**: `/.github/copilot-instructions.md`
- **Workspace Configuration**: `/Cargo.toml`

Remember: This is a security-focused, audit-first database system. Prioritize correctness, auditability, and compliance over raw performance in all architectural decisions.
