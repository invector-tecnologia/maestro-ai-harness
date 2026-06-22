# Maestro Copilot Operating Directives

## Mission Split
- Product mission: keep Maestro AI Harness language-agnostic and architecture-first.
- Companion mission: be deeply specialized in the current task stack (Rust, TypeScript, Python, infra, docs, or CI) while preserving product neutrality.

## Global Priorities
1. Follow explicit user intent and current task goal.
2. Preserve hexagonal architecture and domain boundaries from `docs/Maestro_Manifesto/ARCHITECTURE.md`.
3. Enforce coding conventions from `docs/Maestro_Manifesto/CONVENTIONS.md` when touching Rust code.
4. Prefer minimal, testable, reversible changes.

## Rust-Specific Non-Negotiables
- Never use `unwrap()`, `expect()`, or `panic!()` in production paths.
- Use `thiserror` for domain and application error mapping.
- Use `anyhow` only at presentation and CLI boundaries.
- For shared async state, prefer `Arc<tokio::sync::RwLock<T>>`.
- Use `tracing` for observability, not `println!` or `dbg!`.

## Delivery Behavior
- Implement requested changes end-to-end, then validate with tests or checks.
- Surface risks and missing evidence clearly.
- When directives conflict, prefer: user request > repository manifesto > this file > local style preferences.

## Build & Validation Commands
- Test: `cargo test --all-targets`
- Lint: `cargo clippy --all-targets -- -D warnings`
- Format: `cargo fmt --all`
- Quality gate: `scripts/quality-gate.sh`
- Doc links: `scripts/check-doc-links.sh`

## Layer -> Folder Map
- domain -> `src/domain/` (pure domain logic, no direct I/O or provider SDK concerns)
- application -> `src/application/` (orchestration, use case coordination)
- infrastructure -> `src/infrastructure/` (external adapters, providers, persistence, network)
- presentation -> `src/presentation/` (CLI/TUI boundary and user-facing parsing)

## Scope Reminder
- Do not force Rust guidance on non-Rust tasks.
- Do not force non-Rust guidance on Rust internals.
- Keep recommendations contextual to the files being edited.
