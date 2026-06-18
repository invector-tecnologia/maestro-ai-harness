# TASK 001: Core Domain Validation (Milestone 0)

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** #file:src/domain/models/message.rs, #file:src/domain/ports/role.rs
* **Context Anchors:** #file:docs/Maestro_Manifesto/CONVENTIONS.md, #file:docs/Maestro_Manifesto/ARCHITECTURE.md
* **Pipeline:** Input -> <reasoning> -> Self-Consistency Check -> Strict Output

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* Use of `unwrap()`, `expect()`, or blocking loops is forbidden.
* Final code must pass `cargo clippy -- -D warnings`.
* Output must contain ONLY the modified code blocks. No explanatory text.

## 3. EXECUTION PROMPT (Paste into Copilot Chat)
"""
Act as a Senior Rust Software Engineer and TDD specialist.
Goal: Implement structured unit tests for the Core Domain, strictly following the guidelines in #file:docs/Maestro_Manifesto/CONVENTIONS.md and #file:docs/Maestro_Manifesto/ARCHITECTURE.md.

Before generating any code, open a `<reasoning>` block and mentally evaluate the async concurrency impacts (`Send + Sync + 'static`) of the requested structures.

Execute the following steps directly:

1. In #file:src/domain/models/message.rs, add an internal `#[cfg(test)]` module validating:
   - Uniqueness of `MessageId` (UUID v4) across isolated calls to `Message::new`.
   - Integrity and immutability of `sender`, `content`, and `cause_by` after clone and transfer operations.

2. In #file:src/domain/ports/role.rs, add an internal `#[cfg(test)]` module. Write a mock struct `DummyRole` that fully implements the `Role` trait. The sole purpose is to prove to the Rust compiler that the trait is Object Safe and satisfies thread-safety requirements.

[Cohesion Mechanism]: Before emitting output, run a Self-Consistency Check confirming that no code violates the anchored architectural guidelines.

Return ONLY ready code blocks in Markdown. No introductory text. Be direct.
"""
