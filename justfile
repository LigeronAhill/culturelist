# https://just.systems

set shell := ["zsh", "-c"]
set windows-shell := ["pwsh.exe", "-NoLogo", "-Command"]
set dotenv-load := true

# watch application
[group("app")]
default: dev

# run application
[group("app")]
run:
    cargo run

# watch application
[group("app")]
dev:
    watchexec -r -e rs,html,css -- cargo run

# list available commands
[group("just")]
help:
    just --list

# run tests
[group("test")]
test:
    cargo test -- --nocapture

# test coverage
[group("test")]
coverage:
    cargo tarpaulin --ignore-tests

# run linter
[group("code quality")]
lint:
    cargo clippy --fix --allow-dirty --allow-staged

# format code
[group("code quality")]
fmt:
    cargo fmt --all

# audit code
[group("code quality")]
audit:
    cargo audit --ignore "RUSTSEC-2023-0071" fix

# check unused dependencies
[group("code quality")]
check-unused:
    cargo +nightly udeps

# check code quality
[group("code quality")]
check-quality: lint fmt audit check-unused coverage

# install helpers
[group("dependencies")]
install-dependencies:
    cargo install cargo-binstall
    cargo binstall cargo-tarpaulin
    cargo install --git https://github.com/est31/cargo-udeps --locked
    cargo install cargo-audit --features=fix
    cargo install sqlx-cli
    cargo binstall watchexec-cli

# create database
[group("database")]
create-db:
    sqlx database create

# drop database
[group("database")]
drop-db:
    sqlx database drop

# add migration
[group("database")]
add-migration TABLE:
    sqlx migrate add -r {{ TABLE }}

# run migrations
[group("database")]
run-migration:
    sqlx migrate run

# revert migration
[group("database")]
revert-migration:
    sqlx migrate revert

# prepare for commit
[group("git")]
prepare: lint fmt check-quality

# commit changes
[group("git")]
commit NAME: prepare
    git add .
    git commit -m {{ NAME }}

# push changes
[group("git")]
push:
    git push
