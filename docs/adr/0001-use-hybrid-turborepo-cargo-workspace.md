# ADR 0001: Use Hybrid Turborepo + Cargo Workspace

## Status

Accepted

## Context

NeonOS will likely need both Rust and TypeScript.

## Decision

Use Turborepo for JavaScript/TypeScript workspaces and Cargo workspace for Rust crates.

## Consequences

The repo can support a Rust-first CLI while leaving room for TypeScript packages and future UI.
