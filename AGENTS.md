# AGENTS.md - Development Guidelines for Culturelist

This document provides guidelines for agentic coding agents working in the Culturelist Rust codebase.

## Project Overview

Culturelist is a book tracking application built with Rust, using PostgreSQL for data storage. The project follows a clean modular architecture with async/await patterns throughout.

**Stack**: Rust (edition 2024), Tokio, SQLx, PostgreSQL, Tracing, Config
**Architecture**: Library + Binary pattern with modular components

## Build, Lint, and Test Commands

The project uses Just as a task runner. All commands should be executed via `just <command>`:

### Development Commands
```bash
just run              # Run the application once
just dev              # Watch mode - auto-restart on file changes
just default          # Same as dev (default command)
```

### Testing Commands
```bash
just test             # Run all tests with output (--nocapture)
just coverage         # Generate test coverage report (cargo tarpaulin)
```

**Running a single test**:
```bash
cargo test test_name -- --nocapture
# For a specific module:
cargo test module::test_name -- --nocapture
```

### Code Quality Commands
```bash
just lint             # Run clippy with auto-fix
just fmt              # Format code with rustfmt
just audit            # Security audit with cargo audit
just check-unused     # Check unused dependencies (requires nightly)
just check-quality    # Run all quality checks: lint, fmt, audit, check-unused, coverage
```

### Database Commands
```bash
just create-db        # Create the database
just drop-db          # Drop the database
just add-migration TABLE  # Add new migration for TABLE
just run-migration    # Run pending migrations
just revert-migration # Revert last migration
```

### Git Workflow Commands
```bash
just prepare          # Run lint, fmt, and check-quality before commit
just commit NAME      # Stage all changes and commit (requires prepare first)
just push             # Push changes to remote
```

## Code Style Guidelines

### Import Style
```rust
// External crates first, grouped by functionality
use anyhow::Result;
use config::Config;
use sqlx::{Pool, Postgres};

// Local modules
use crate::configuration;
use crate::storage;
```

**Rules**:
- External crates first, then local modules
- Group related imports together
- Use specific imports rather than `use crate::*`
- One `use` statement per line for multiple items from same crate

### Naming Conventions
- **Functions**: `snake_case` (e.g., `get_pool`, `init_configuration`)
- **Structs/Enums**: `PascalCase` (e.g., `App`, `DatabaseConfig`)
- **Constants**: `SCREAMING_SNAKE_CASE` (when needed)
- **Files**: `snake_case.rs` (e.g., `configuration.rs`, `logger.rs`)
- **Modules**: `snake_case` for private modules, `snake_case` for public

### Error Handling Pattern
```rust
use anyhow::Result;

pub async fn function_name(param: &str) -> Result<String> {
    // Implementation
    let result = some_operation().await?;
    Ok(result)
}
```

**Rules**:
- Always use `anyhow::Result<T>` for public functions
- Use `?` operator for error propagation
- Return `Result<()>` for functions without meaningful return values
- Include context with `.context("description")` when helpful

### Module Organization
```rust
// lib.rs - Public API
pub mod configuration;
pub mod logger;
mod storage;  // Private module

pub async fn build(config: &Config) -> Result<App> {
    // Public factory function
}

pub struct App {
    // Main application state
}
```

**Rules**:
- Export public modules from `lib.rs`
- Keep implementation details private
- Use `pub(crate)` for internal public APIs
- Group related functionality in modules

### Async/Await Patterns
```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = app::configuration::init()?;
    let app = app::build(&config).await?;
    app.run().await?;
    Ok(())
}
```

**Rules**:
- Use `#[tokio::main]` for async main functions
- Always use `.await?` for async operations that can fail
- Keep async functions small and focused
- Use `async fn` for any function that does I/O

### Database Patterns
```rust
pub async fn get_pool(config: &Config) -> Result<Pool<Postgres>> {
    let db_url = config.get_string("database.url")?;
    let pool = PgPoolOptions::new()
        .max_connections(8)
        .connect(&db_url)
        .await?;
    sqlx::migrate!().run(&pool).await?;
    Ok(pool)
}
```

**Rules**:
- Use SQLx for all database operations
- Always run migrations on startup
- Use connection pooling with reasonable limits (8 is current default)
- Use compile-time checked queries when possible
- Handle database errors with `anyhow::Result`

### Configuration Pattern
```rust
pub fn init() -> Result<Config, ConfigError> {
    let env = std::env::var("APP_ENVIRONMENT").unwrap_or("development".into());
    // Build hierarchical config: base -> environment -> env vars
    Config::builder()
        .add_source(config::File::from(base).required(true))
        .add_source(config::File::from(file).required(false))
        .add_source(config::Environment::with_prefix("APP").separator("_"))
        .build()
}
```

**Rules**:
- Use hierarchical configuration: base → environment → environment variables
- Prefix environment variables with `APP_`
- Default to "development" environment
- Use `APP_ENVIRONMENT` to specify environment

### Logging Pattern
```rust
pub fn init(config: &Config) -> Result<()> {
    let env = config.get_string("app.environment").unwrap_or("development".into());
    match env.as_str() {
        "production" => {
            // ERROR level for production
            let subscriber = tracing_subscriber::FmtSubscriber::builder()
                .with_max_level(tracing::Level::ERROR)
                .with_file(true)
                .with_line_number(true)
                .with_target(false)
                .pretty()
                .finish();
            tracing::subscriber::set_global_default(subscriber)?;
        }
        _ => {
            // INFO level for development
            // ... similar setup with INFO level
        }
    }
    tracing::info!("logger initialized");
    Ok(())
}
```

**Rules**:
- Use tracing for all logging
- Production: ERROR level only
- Development/Default: INFO level
- Include file and line numbers
- Use pretty formatting
- Log initialization completion

### Code Organization Best Practices

**File Structure**:
```
src/
├── lib.rs              # Library root, exports public modules
├── main.rs             # Binary entry point
├── configuration.rs    # Configuration management
├── logger.rs          # Logging setup
└── storage/
    └── mod.rs         # Database operations and migrations
```

**Adding New Modules**:
1. Create `module_name.rs` file
2. Add `mod module_name;` or `pub mod module_name;` to `lib.rs`
3. Follow existing patterns for structure and error handling
4. Add module-specific tests with `#[cfg(test)]`

### Testing Guidelines
- Use Rust's built-in testing framework
- Place tests in `#[cfg(test)]` modules within source files
- Use `--nocapture` flag to see test output
- Aim for good test coverage (check with `just coverage`)
- Test both success and error paths

### Pre-commit Checklist
Always run `just prepare` before committing:
1. `just lint` - Fix all clippy warnings
2. `just fmt` - Ensure consistent formatting
3. `just check-quality` - Pass all quality checks

### Environment Setup
- Install dependencies with `just install-dependencies`
- Set up PostgreSQL and run `just create-db`
- Configure `.env` file with database URL
- Use `just dev` for development with auto-reload

## Common Gotchas

1. **Nightly Required**: `just check-unused` requires nightly Rust toolchain
2. **Database URL**: Must be set in environment or `.env` file
3. **Migrations**: Auto-run on app startup, can be managed manually with SQLx CLI
4. **Environment**: Defaults to development, set `APP_ENVIRONMENT=production` for production
5. **Logging Level**: Adjust per environment in logger configuration

This codebase prioritizes safety, performance, and maintainability following Rust best practices.