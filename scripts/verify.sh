#!/bin/sh
# Runs exactly what CI runs minus the end-to-end smoke test, in order, stopping at the first
# failure (docs/TESTING.md §2). The front end is built first so the Tauri host finds its
# frontendDist before any cargo step compiles it.
set -eu
cd "$(dirname "$0")/.."

step() {
  printf '\n== %s\n' "$1"
  shift
  "$@"
}

step "frontend build"     npm --prefix app run build
step "frontend typecheck" npm --prefix app run typecheck
step "frontend lint"      npm --prefix app run lint
step "frontend format"    npm --prefix app run format:check
step "frontend tests"     npm --prefix app test
step "rust format"        cargo fmt --all --check
step "rust clippy"        cargo clippy --workspace --all-targets -- -D warnings
step "rust licences"      cargo deny check licenses
step "npm licences"       npm --prefix app run licenses
step "fixtures"           cargo run -q -p keepers-fixtures -- --out tests/fixtures/gen --seed 1 --check tests/fixtures/gen.sha256
step "rust tests"         cargo test --workspace

printf '\nverify: green\n'
