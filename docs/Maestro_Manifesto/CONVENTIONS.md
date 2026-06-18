MAESTRO AI HARNESS: CODE AND ARCHITECTURE CONVENTIONS (RUST)

This document defines non-negotiable syntax and architecture rules for code generation.

## 1. Error Handling (Zero Tolerance for Panics)
* **FORBIDDEN:** `unwrap()`, `expect()`, or `panic!()`.
* **Domain and Application:** Use `thiserror` for domain/application error mapping.
* **Presentation and CLI:** Use `anyhow` to propagate user-facing errors.
* Always propagate errors using the `?` operator.

## 2. State Management and Concurrency (Tokio)
* **Shared State:** For async mutability, use only `Arc<tokio::sync::RwLock<T>>` or `Arc<tokio::sync::Mutex<T>>`.
* **Thread Blocking:** Never use `std::sync::Mutex` or blocking I/O calls inside Tokio async runtime paths.
* **Messaging:** Prefer `tokio::sync::broadcast` for bus fan-out (1:N) and `tokio::sync::mpsc` for actor queues (1:1).

## 3. Typing and Memory Safety
* **Newtypes:** Wrap primitives in structs for compile-time safety (for example: `struct AgentId(String);`).
* **Clones:** Avoid `.clone()` on heavy structures. Clone only smart pointers (`Arc`) or lightweight bus payloads.
* **Lifetimes:** Avoid polluting domain structs with explicit lifetimes (`'a`). Prefer owned data (`String`, `Vec`).

## 4. Traceability (Observability)
* **FORBIDDEN:** `println!` or `dbg!`.
* **REQUIRED:** Use `tracing`.
* Emit structured logs with `tracing::info!`, `tracing::debug!`, and `tracing::error!` to track the agent lifecycle (Observe, Think, Act).

## 5. Testing (TDD)
* Every domain module must include a `#[cfg(test)]` block.
* Use `mockall` to mock infrastructure traits (ports) in unit tests.

## 6. Startup and UI (Readiness and Internationalization)
* **Readiness Check:** Every Maestro startup must run a dependency and requirements check flow.
* **Failure Feedback:** If checks fail ("Not Passed"), the system must display a detailed Readiness section showing explicit pass/fail items.
* **Language:** All user-facing Maestro content, labels, commands, manuals (including `README.md`), and runtime messages must be US English.

## 7. AI Harness and Prompt Management
* **Context Isolation:** Prompts must not be hardcoded in business logic. Use templates or isolated prompt files under strict Harness control.
* **Safety and Limits:** Treat every AI output as untrusted input. Validate destructive actions through the Harness before execution and enforce context-window limits.
