# Memory Management and Borrow Checker (Rust)

## 1. Golden Rule (Lifetimes vs Allocation)
In async systems and multi-agent pipelines (like Maestro), fighting lifetimes (`'a`) inside structs usually adds unnecessary complexity.
* **CORRECT:** Prefer owned types like `String` and `Vec<T>` in domain entities.
* **EXCEPTION:** Use references (`&str`, `&[T]`) only in temporary function signatures that do not store data.

## 2. Correct Clone Usage
* **FORBIDDEN:** Using `.clone()` indiscriminately to silence compiler errors.
* **ALLOWED:** Cloning immutable event payloads (like bus `Message` values).
* **STATE SHARING:** For heavy structures or mutable shared state, never clone data. Clone the smart pointer:
  ```rust
  // Correct
  let state = Arc::clone(&self.shared_state);
	```

## 3. Strong Typing and Safety
* Avoid stringly-typed development.
* Create newtypes for IDs and domain concepts.
    ```rust
    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct AgentId(pub String);
    ```

## 4. Error Handling Without Panics

* **NEVER USE:** `unwrap()`, `expect()`, or `panic!()`.
* **ALWAYS USE:** Error propagation with `?`.
