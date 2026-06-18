# Tokio Patterns (Concurrency and State)

## 1. Safe Shared State
Never block the async runtime thread. For shared mutable state:
* **WRONG:** `std::sync::Mutex`
* **CORRECT:** `tokio::sync::RwLock` (preferred for read-heavy paths) or `tokio::sync::Mutex`.
* **REQUIRED SHAPE:**
  ```rust
  type SharedState = Arc<tokio::sync::RwLock<MyStruct>>;
  ```
## 2. Actor Communication (Channels)
Maestro uses message passing, not raw shared-memory coupling.
* **Point-to-Point:** Use `tokio::sync::mpsc` for single-agent work queues.
* **Pub/Sub (Global Bus):** Use `tokio::sync::broadcast` for Environment fan-out notifications. Receivers must be created via `tx.subscribe()`.

## 3. Task Spawning (Green Threads)
Every agent runs in the background.
* Agent runtime contracts must satisfy `Send + Sync + 'static`.
* Use `tokio::spawn` for each agent main loop.
