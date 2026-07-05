# culturelist — agent guide

## Dev commands

```
just run        # cargo run
just dev        # watchexec -r -e rs,html,css -- cargo run (auto-reload)
just test       # cargo test -- --nocapture
just lint       # cargo clippy --fix --allow-dirty --allow-staged
just fmt        # cargo fmt --all
just prepare    # lint + fmt + check-quality
just install-dependencies   # installs cargo-binstall, tarpaulin, udeps, audit, sqlx-cli, watchexec
```

All just commands load `.env` (dotenv).  Full list in `justfile`.

## Architecture

- **Single crate** (no workspace), Rust edition **2024** — requires nightly toolchain.
- **Binary** `src/main.rs` → lib `src/lib.rs` (App::build/run).  Module stack: `controllers` → `services` → `storage` (SQLx query files in `queries/`).
- **Router** `router/mod.rs` mounts page handlers only.  Controllers in `controllers/users.rs` define REST handlers but are **not mounted** — WIP.
- **Askama** templates in `templates/` — compile-time checked HTML.  Edit `.html` files to change UI.
- **Datastar** (vendored `public/scripts/datastar.js`) for SSE-driven interactivity via HTML attributes.
- All UI text in **Russian** (`ru` lang).

## Testing

- `cargo test -- --nocapture` — tests need **running PostgreSQL** (see `.env` for `DATABASE_URL`).
- `#[sqlx::test]` in `storage/users_storage.rs` creates test databases — requires `sqlx-cli`.
- Unit tests in `models/user.rs` (~20 validation/password tests).

## Development quirks

- **SQLx compile-time checking** — `query_file_as!` macros need `DATABASE_URL` set at build time, or use `SQLX_OFFLINE=true`.
- **CSRF key** generated at startup with `Key::generate()` — invalidated on every restart.  Not suitable for production.
- **JWT secret** defaults to `"your-secret-key"` when `JWT_SECRET` env var unset.
- Config: `configurations/base.toml` overridden by env vars with `APP_` prefix.
- No CI workflows, no pre-commit hooks, no README.

## Style

- Follow existing patterns: Axum handlers, Result types, module structure.
- `.html` templates are Askama — use `{{ }}` expressions, `{% %}` blocks.
- Add new `.sql` query files under `queries/<entity>/` and reference via `sqlx::query_file_as!`.
