---
applyTo: "src/**/*.rs"
description: "Use when editing Rust code in Maestro to enforce architecture boundaries, error handling, Tokio concurrency, and testing conventions."
---

# Rust Companion Rules

## Architecture
- Keep domain pure and free from direct I/O and provider SDK details.
- Keep orchestration in application; external adapters in infrastructure; parsing and UX in presentation.

## Error and Concurrency
- Never introduce `unwrap()`, `expect()`, or `panic!()` in normal paths.
- Use `thiserror` in domain/application error types and propagate with `?`.
- Use `Arc<tokio::sync::RwLock<T>>` for shared mutable async state by default.

## Quality Gate
- Add or update tests when behavior changes.
- Prefer focused unit tests close to changed module.
- Keep public API changes explicit and documented in PR notes.
