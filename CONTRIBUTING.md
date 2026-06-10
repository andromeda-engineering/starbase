# Contributing to NeonOS

Thanks for your interest in contributing.

## Project Status

NeonOS is pre-MVP. APIs, commands, and internal structure may change frequently.

## Development Setup

```bash
pnpm install
pnpm check
cargo check --workspace
cargo test --workspace
```

## Pull Request Expectations

Before opening a PR, run:

```bash
pnpm check
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Coding Standards

- Rust is intended for the future CLI and local engine.
- TypeScript is intended for future packages, schemas, and UI.
- Prefer clear deterministic behavior over opaque automation.
- Avoid destructive file writes.
- Add tests for behavior changes.

## Commit Style

Use clear conventional-style commits when practical:

```text
feat: add repo detection skeleton
fix: preserve existing generated files
docs: document phase 0 setup
```
