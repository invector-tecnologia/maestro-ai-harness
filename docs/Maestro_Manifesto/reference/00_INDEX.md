# Passive Knowledge Map (RAG)

This index maps the technical references used by Maestro.
**Action Rule:** If you are unsure how to implement a required pattern during complexity analysis, load the right context first with `/read docs/Maestro_Manifesto/reference/<FILE>.md`.

## Reference Index

* **Concurrency and Asynchrony (Tokio):** Read `tokio_concurrency.md` for `Arc`, `Mutex`, `RwLock`, and `mpsc`/`broadcast` channel rules.
* **Memory Management (Rust):** Read `rust_borrow_checker.md` for lifetimes, ownership, and avoiding unnecessary `clone()` calls.
* **Domain-Driven Design in Rust:** Read `design_patterns_ddd.md` for Ports and Adapters patterns and async trait boundaries.
* **Canonical Cognitive Pattern:** Read [COGNITIVE_PATTERN.md](COGNITIVE_PATTERN.md) for the shared `SENSE → OBSERVE → THINK → ACT → AUDIT → DELIVER` cycle and its code map.
