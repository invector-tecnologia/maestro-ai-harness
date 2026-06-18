# TASK 002: Asynchronous Broadcast Bus (Environment)

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** #file:src/domain/models/message.rs
* **Context Anchors:** #file:docs/Maestro_Manifesto/CONVENTIONS.md, #file:docs/Maestro_Manifesto/ARCHITECTURE.md
* **Expected Output:** `src/application/environment.rs` with an async broadcast bus and integrated tests.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* Blocking loops and `std::sync::Mutex` are strictly forbidden.
* Must use `tokio::sync::broadcast` for 1:N Pub/Sub message distribution.
* Internal history state must be thread-safe (`Arc<tokio::sync::RwLock<Vec<Message>>>`).

## 3. EXECUTION PROMPT (Paste into Copilot Chat)
"""
Act as a Principal Rust Software Engineer.
Goal: Implement the Environment async message broker for the application layer.

Milestone 0 has passed. Based on #file:docs/Maestro_Manifesto/CONVENTIONS.md and #file:docs/Maestro_Manifesto/ARCHITECTURE.md, create `src/application/environment.rs`.

Before generating code, open a `<reasoning>` block and plan how to expose a broadcast bus where multiple agents subscribe via `tokio::sync::broadcast`, and how to protect the audit history from race conditions using Tokio async primitives.

Follow these structure specifications:
1. Create `struct Environment` containing:
   - `history`: a protected audit vector (`Arc<tokio::sync::RwLock<Vec<Message>>>`).
   - `tx`: a Tokio broadcast channel sender (`tokio::sync::broadcast::Sender<Message>`).

2. Implement these public methods:
   - `pub fn new(capacity: usize) -> Self` — initializes the vector and broadcast channel.
   - `pub fn publish(&self, msg: Message)` — sends the message via broadcast and pushes it to `history` in the background. Since the method takes `&self`, acquire the RwLock write guard asynchronously without blocking.
   - `pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<Message>` — returns a new channel receiver (`self.tx.subscribe()`).
   - `pub async fn get_history(&self) -> Vec<Message>` — returns a stable clone of the message history.

3. Write an internal `#[cfg(test)]` module validating:
   - That two different async tasks (`tokio::spawn`) can subscribe via `subscribe()`, both receive a simultaneously published message, and the history stores that message correctly.

[Cohesion Mechanism]: Before emitting code, run a Self-Consistency Check confirming that no async thread will be blocked by synchronous locks.

Return ONLY the complete code block for the file. No introduction. Be direct.
"""
