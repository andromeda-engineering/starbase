# NeonOS

NeonOS is an open-source AI-native developer environment for repo setup, agent configuration, local memory, and repeatable coding workflows.

## Status

Experimental. Phase 0 repository setup.

This repo currently contains only a minimal monorepo scaffold:

- one stub TypeScript app
- one stub TypeScript package
- one stub Rust crate
- open-source documentation
- GitHub workflows
- issue templates
- Dependabot

## Repository

```text
github.com/MyNameReallySux/neonos
```

## Package Scope

```text
@neonwave/neonos
```

## Goals

NeonOS will eventually help developers:

- initialize repos with consistent tooling
- configure coding agents
- generate repo context
- store local repo memory
- capture task retrospectives
- support repeatable TDD, spec-driven, and story-driven workflows

## Non-Goals

NeonOS is not:

- a literal operating system
- a cloud IDE
- a hosted SaaS-first product
- a general task manager

## Development

```bash
pnpm install
pnpm check
cargo check --workspace
cargo test --workspace
```

## License

MIT
