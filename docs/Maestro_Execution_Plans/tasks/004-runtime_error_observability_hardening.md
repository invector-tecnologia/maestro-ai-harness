# TASK 004: Runtime Error Handling and Observability Hardening

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** #file:src/main.rs, #file:src/application/environment.rs, #file:src/domain/ports/role.rs
* **Context Anchors:** #file:docs/Maestro_Manifesto/CONVENTIONS.md, #file:docs/Maestro_Manifesto/ARCHITECTURE.md
* **Expected Output:** Runtime with no implicit panics, structured logs, and explicit error contracts.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* Use of `unwrap()`, `expect()`, `panic!()`, and `println!` in operational flows is forbidden.
* Application errors must be typed and propagatable.
* No synchronous blocking inside the Tokio runtime.

## 3. EXECUTION PROMPT (Paste into Copilot Chat)
"""
Act as a Senior Rust Architect focused on production reliability.
Goal: Strengthen Maestro's error contracts and observability without breaking the hexagonal architecture.

Before generating code, open a `<reasoning>` block and evaluate async concurrency impacts (`Send + Sync + 'static`) and failure propagation paths.

Execute:
1. Refactor the entrypoint to use structured `tracing` instead of print statements.
2. Refactor the `Environment` publish contract to return an explicit error.
3. Define typed application errors for broadcast, history, and runtime failures.
4. Ensure error propagation with the `?` operator at appropriate layers.
5. Add tests covering:
   - Bus send failure.
   - Behavior with no subscribers.
   - History integrity under concurrency.

[Cohesion Mechanism]:
- Verify full adherence to error handling and observability conventions.
- Verify absence of blocking calls.

Return ONLY the modified code blocks in Markdown. No introduction.
"""
