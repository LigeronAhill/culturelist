# AGENTS.md - Development Guidelines for Culturelist

Book tracking application built with Rust, PostgreSQL, and async/await patterns.

**Stack**: Rust (edition 2024), Tokio, SQLx, PostgreSQL, Tracing, Config
**Architecture**: Library + Binary pattern with modular components

## Commands

Use Just as task runner: `just <command>`

### Development
```bash
just run              # Run once
just dev              # Watch mode (default)
```

### Testing
```bash
just test             # Run all tests (--nocapture)
just coverage         # Coverage report
cargo test test_name -- --nocapture  # Single test
```

### Code Quality
```bash
just lint             # Clippy with auto-fix
just fmt              # Format code
just audit            # Security audit
just check-unused     # Unused deps (requires nightly)
just check-quality    # Run all quality checks
```

### Database
```bash
just create-db        # Create database
just add-migration TABLE  # Add migration
just run-migration    # Run migrations
```

### Git Workflow
```bash
just prepare          # Pre-commit checks
just commit NAME      # Stage and commit
```

## API Routes

### Authentication Routes
- **POST /api/v1/auth/signin** - User sign in
- **POST /api/v1/auth/signup** - User sign up

### User Management Routes
- **GET /api/v1/users/** - List users (with pagination and search)
- **POST /api/v1/users/** - Create user
- **GET /api/v1/users/{id}** - Get user by ID
- **PUT /api/v1/users/{id}** - Update user
- **DELETE /api/v1/users/{id}** - Delete user

### Authentication Features
- JWT token-based authentication (7-day expiration)
- Password hashing with Argon2
- Email validation and password complexity requirements
- Email uniqueness checks

## Code Style Guidelines

### Import Style
```rust
use anyhow::Result;
use config::Config;
use sqlx::{Pool, Postgres};
use crate::configuration;
use crate::storage;
```

**Rules**: External crates first, then local modules; group related imports; use specific imports; one `use` per line.

### Naming Conventions
- **Functions**: `snake_case` (e.g., `get_pool`, `init_configuration`)
- **Structs/Enums**: `PascalCase` (e.g., `App`, `DatabaseConfig`)
- **Constants**: `SCREAMING_SNAKE_CASE`
- **Files**: `snake_case.rs` (e.g., `configuration.rs`, `logger.rs`)
- **Modules**: `snake_case`

### Error Handling Pattern
```rust
use anyhow::Result;

pub async fn function_name(param: &str) -> Result<String> {
    let result = some_operation().await?;
    Ok(result)
}
```

**Rules**: Always use `anyhow::Result<T>` for public functions; use `?` operator; return `Result<()>` for void functions; include context with `.context()` when helpful.

### Module Organization
```rust
pub mod configuration;
pub mod logger;
mod storage;

pub async fn build(config: &Config) -> Result<App> {
    // Public factory function
}

pub struct App {
    // Main application state
}
```

**Rules**: Export public modules from `lib.rs`; keep implementation details private; use `pub(crate)` for internal APIs; group related functionality.

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

**Rules**: Use `#[tokio::main]` for async main; always use `.await?` for async operations; keep async functions small; use `async fn` for I/O operations.

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

**Rules**: Use SQLx for all database operations; always run migrations on startup; use connection pooling (default 8); use compile-time checked queries; handle errors with `anyhow::Result`.

### Configuration Pattern
```rust
pub fn init() -> Result<Config, ConfigError> {
    let env = std::env::var("APP_ENVIRONMENT").unwrap_or("development".into());
    Config::builder()
        .add_source(config::File::from(base).required(true))
        .add_source(config::File::from(file).required(false))
        .add_source(config::Environment::with_prefix("APP").separator("_"))
        .build()
}
```

**Rules**: Use hierarchical config: base → environment → env vars; prefix env vars with `APP_`; default to "development"; use `APP_ENVIRONMENT` to specify environment.

### Logging Pattern
```rust
pub fn init(config: &Config) -> Result<()> {
    let env = config.get_string("app.environment").unwrap_or("development".into());
    match env.as_str() {
        "production" => {
            let subscriber = tracing_subscriber::FmtSubscriber::builder()
                .with_max_level(tracing::Level::ERROR)
                .with_file(true)
                .with_line_number(true)
                .with_target(false)
                .pretty()
                .finish();
            tracing::subscriber::set_global_default(subscriber)?;
        }
        _ => { /* INFO level for development */ }
    }
    tracing::info!("logger initialized");
    Ok(())
}
```

**Rules**: Use tracing for logging; production: ERROR level only; development: INFO level; include file/line numbers; use pretty formatting.

### Code Organization

**File Structure**:
```
src/
├── lib.rs              # Library root
├── main.rs             # Binary entry point
├── configuration.rs    # Configuration management
├── logger.rs          # Logging setup
└── storage/
    └── mod.rs         # Database operations
```

**Adding New Modules**: Create `module_name.rs`; add `mod module_name;` or `pub mod module_name;` to `lib.rs`; follow existing patterns; add tests with `#[cfg(test)]`.

### Testing Guidelines
- Use Rust's built-in testing framework
- Place tests in `#[cfg(test)]` modules within source files
- Use `--nocapture` flag to see test output
- Aim for good test coverage (check with `just coverage`)
- Test both success and error paths

### Database Testing with sqlx::test
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use fake::{Fake, Faker};
    use sqlx::test;

    #[sqlx::test]
    async fn test_create_user(pool: sqlx::PgPool) -> anyhow::Result<()> {
        // Run migrations first
        sqlx::migrate!().run(&pool).await?;
        
        let storage = UsersStorage::new(pool);
        // Test implementation
        Ok(())
    }
}
```

**Rules**: Always run migrations in test setup; use transaction rollback for isolation; generate realistic test data with fake; test edge cases and error conditions.

### Test Data Generation with fake
```rust
use fake::{Fake, Faker};
use fake::faker::internet::en::{SafeEmail, Username};
use fake::faker::name::en::{FirstName, LastName};

fn create_fake_user() -> CreateUser {
    CreateUser {
        username: Username().fake(),
        email: SafeEmail().fake(),
        password: "Password123!".to_string(),
        first_name: Some(FirstName().fake()),
        last_name: Some(LastName().fake()),
        bio: Some(Faker.fake()),
    }
}
```

**Rules**: Use domain-specific fakers when available; ensure generated data meets validation requirements; use consistent data for related tests.

### Pre-commit Checklist
Always run `just prepare` before committing: `just lint` → `just fmt` → `just check-quality`.

### Environment Setup
- Install dependencies: `just install-dependencies`
- Set up PostgreSQL: `just create-db`
- Configure `.env` file with database URL
- Use `just dev` for development with auto-reload

## Common Gotchas

1. **Nightly Required**: `just check-unused` requires nightly Rust toolchain
2. **Database URL**: Must be set in environment or `.env` file
3. **Migrations**: Auto-run on app startup, can be managed manually with SQLx CLI
4. **Environment**: Defaults to development, set `APP_ENVIRONMENT=production` for production
5. **Logging Level**: Adjust per environment in logger configuration
6. **JWT Secret**: Set `JWT_SECRET` environment variable for token generation (defaults to insecure key in development)

## Environment Variables

Required environment variables:
- `DATABASE_URL` - PostgreSQL connection string
- `JWT_SECRET` - Secret key for JWT token signing

Optional environment variables:
- `APP_ENVIRONMENT` - Environment (development/staging/production)
- `SERVER_PORT` - Server port (defaults to 3000)

## Single Test Execution

```bash
# Run specific test with output
cargo test test_name -- --nocapture

# Run tests in specific module
cargo test storage::users_storage::tests -- --nocapture

# Run with filters
cargo test -- test_create_user test_get_user -- --nocapture

# Run sqlx tests with database setup
DATABASE_URL="postgresql://user:pass@localhost/test" cargo test sqlx_test_name -- --nocapture
```

## Common Testing Patterns

### Storage Layer Testing
- **Test isolation**: Each test gets a clean database state
- **Transaction rollback**: Use `#[sqlx::test]` for automatic cleanup
- **Realistic data**: Use `fake` library for comprehensive test coverage
- **Error cases**: Test database constraints and error handling

### Service Layer Testing  
- **Mock storage**: Use dependency injection for unit tests
- **Business logic**: Test validation and transformation logic
- **Error propagation**: Test proper error handling chain

### Integration Testing
- **End-to-end**: Test full request/response cycles
- **Database integration**: Test with real PostgreSQL instance
- **Authentication**: Test JWT token generation and validation

This codebase prioritizes safety, performance, and maintainability following Rust best practices.