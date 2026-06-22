---
name: rust
description: >
  Comprehensive Rust implementation and review guidance for Maestro. Use when
  writing, reviewing, or refactoring Rust code, especially around ownership,
  error handling, async/Tokio patterns, architecture boundaries, observability,
  and testing.
---

# Rust Skill

Use this skill for Rust code changes and reviews in Maestro.

## When to Apply

- Writing or refactoring modules in `src/**/*.rs`
- Reviewing PRs for Rust correctness and safety
- Implementing async flows, shared state, or error boundaries
- Improving observability, test coverage, or performance-sensitive code

## Core Priorities

1. Preserve architecture boundaries from `docs/Maestro_Manifesto/ARCHITECTURE.md`.
2. Follow conventions from `docs/Maestro_Manifesto/CONVENTIONS.md`.
3. Keep changes minimal, testable, and reversible.
4. Favor explicit, typed errors and structured telemetry.

## Critical Rules

- Avoid `unwrap()`, `expect()`, and `panic!()` in production paths.
- Use `thiserror` for domain/application error types.
- Use `anyhow` only at presentation and CLI boundaries.
- Use `?` for propagation and keep error context actionable.
- Keep domain logic pure and free of direct I/O and SDK coupling.
- Keep orchestration in `application` and adapters in `infrastructure`.
- Use `tracing` instrumentation instead of `println!` and `dbg!`.
- Add or update tests for any behavior change.

## Category Checklist

### Ownership and Borrowing

- Prefer borrowing (`&T`, `&str`, `&[T]`) over cloning.
- Use `Arc<T>` for shared ownership across tasks.
- Use interior mutability types intentionally (`RwLock`, `Mutex`, `RefCell`).
- Keep ownership transfers explicit at API boundaries.

### Error Handling

- Return `Result<T, E>` for recoverable failures.
- Keep error messages precise, consistent, and contextual.
- Preserve error chains so root causes are diagnosable.
- Do not swallow errors silently.

### Async and Concurrency

- Do not hold locks across `.await`.
- Use bounded channels where backpressure matters.
- Use `tokio::fs` and async-safe APIs in async contexts.
- Use cancellation-aware patterns for long-running tasks.

### API and Type Design

- Use newtypes and enums to model invariants.
- Prefer clear, explicit conversions (`From`/`TryFrom`).
- Keep public APIs stable and unsurprising.
- Avoid stringly-typed control flow where typed models fit.

### Observability and Ops

- Add spans/fields for critical operations and state transitions.
- Log once at the right layer; avoid duplicate error logs.
- Never log secrets or sensitive content.
- Ensure important cache/runtime paths are auditable.

### Testing and Validation

- Place focused unit tests close to changed logic.
- Add integration tests for cross-layer behavior when needed.
- Cover edge cases and failure paths, not only happy paths.
- Keep test names descriptive and behavior-oriented.

## Review Prompts

Use these prompts while reviewing Rust changes:

- Are architecture boundaries preserved across domain/application/infrastructure/presentation?
- Were new failure paths mapped to typed errors and propagated correctly?
- Is async shared state implemented safely and without lock-across-await risks?
- Is observability sufficient to diagnose incidents and regressions?
- Are tests aligned with changed behavior and acceptance criteria?

## Validation Commands

- `cargo fmt --all`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --all-targets`

## Notes

- This skill is intentionally self-contained and does not depend on external `rules/` files.
- Prefer repository-specific instructions when they conflict with generic Rust guidance.