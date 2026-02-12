# AGENTS.md

This file contains guidelines for agentic coding agents working in this repository.

## Project Overview

This is a Rust CLI application called `devshell` for composing and running developer containers using Docker fragments. The project uses a modular structure with clear separation of concerns.

## Build, Test, and Lint Commands

### Core Commands
```bash
# Build the project
cargo build

# Build for release
cargo build --release

# Run tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run a specific test
cargo test test_name

# Check code without building
cargo check

# Format code
cargo fmt

# Run clippy lints
cargo clippy

# Check for security vulnerabilities (if cargo-audit installed)
cargo audit
```

### Running the Application
```bash
# Run the CLI tool
cargo run

# Run with specific subcommand
cargo run -- run config_name
cargo run -- show @base/debian
cargo run -- doctor
```

## Code Style Guidelines

### Imports and Dependencies
- Use `use crate::` for internal module imports
- Group imports: std library first, then external crates, then internal modules
- Example import order:
  ```rust
  use std::process::Command;
  use clap::{Parser, Subcommand};
  use crate::error::{DevshellError, IoErrorContext};
  use crate::config::load;
  ```

### Module Organization
- Keep modules focused on single responsibility
- Each module in its own file within `src/`
- Use `mod.rs` for module declarations only
- Structure: `src/{module_name}.rs` or `src/{module_name}/mod.rs`

### Naming Conventions
- **Functions**: `snake_case` with descriptive verbs (e.g., `build_image`, `load_config`)
- **Types/Structs**: `PascalCase` (e.g., `DevshellError`, `Config`)
- **Constants**: `SCREAMING_SNAKE_CASE`
- **File names**: `snake_case`
- **Module names**: `snake_case`

### Error Handling
- Use the custom `DevshellError` enum for all errors
- Implement `IoErrorContext` trait for IO operations with context
- Use `?` operator consistently
- Provide clear error messages with context and file paths when relevant
- Example:
  ```rust
  fs::write(&path, content).with_context_and_file(
      "Writing config file", 
      &path.to_string_lossy()
  )?;
  ```

### Type Definitions
- Use `#[derive(Debug, Clone, Serialize, Deserialize)]` for config structs
- Prefer `Option<T>` over nullable values
- Use `Result<T, DevshellError>` for fallible functions
- Add `#[serde(default)]` for optional vector fields in config

### Function Design
- Keep functions small and focused (typically < 50 lines)
- Use descriptive function names that indicate action
- Accept references (`&str`, `&[T]`) for read-only parameters
- Return owned types (`String`, `Vec<T>`) when creating new data

### Documentation
- Add doc comments to public functions and complex logic
- Use `///` for public API documentation
- Include examples in doc comments when helpful
- Use inline comments for complex business logic

### Code Organization Patterns
- **CLI**: Use `clap` with derive macros for command parsing
- **Config**: Use `serde` for TOML serialization/deserialization
- **Docker**: Separate build, run, and container management
- **Error**: Centralized error handling with context
- **Fragments**: Support both embedded and disk-based fragments

### Testing Guidelines
- Write unit tests for each module in a `#[cfg(test)]` block
- Test error paths as well as success paths
- Use descriptive test names that indicate what is being tested
- Example:
  ```rust
  #[test]
  fn test_load_config_with_valid_file() {
      // test implementation
  }
  ```

### Performance Considerations
- Avoid unnecessary allocations in hot paths
- Use `String` and `Vec` when ownership is needed
- Consider `Cow<str>` for string processing that might not need allocation
- Use iterators over manual loops when possible

### Docker Integration
- All Docker operations should return `DevshellError::DockerError` on failure
- Include both stdout and stderr in error messages for debugging
- Clean up temporary files after Docker operations
- Use descriptive container names with `devshell-` prefix

### Configuration Management
- Support both local and global configurations
- Use `dirs` crate for cross-platform directory paths
- Validate configuration on load
- Provide sensible defaults for optional fields

## Development Workflow

1. Run `cargo check` before commits
2. Run `cargo clippy` and fix lint issues
3. Run `cargo test` to ensure tests pass
4. Run `cargo fmt` to format code
5. Test the CLI functionality manually with `cargo run`

## Architecture Notes

- **CLI Layer**: Command parsing and orchestration (`cli.rs`)
- **Config Layer**: Configuration loading and validation (`config/`)
- **Docker Layer**: Container operations (`docker/`)
- **Fragment Layer**: Dockerfile fragment resolution (`fragments/`)
- **Error Layer**: Centralized error handling (`error.rs`)
- **Utils Layer**: System utilities and doctor functionality (`util.rs`)
- **FS Layer**: File system operations and directory management (`fs.rs`)

The application follows a dependency-injection pattern where higher-level modules depend on abstractions, making testing and maintenance easier.