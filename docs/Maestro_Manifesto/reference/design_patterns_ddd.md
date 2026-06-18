# Hexagonal Architecture and DDD in Rust

## 1. Domain Isolation (The Core)
The `src/domain/` layer is sacred.
* **ABSOLUTE RULE:** Zero I/O, network, database, or AI API dependencies (such as `reqwest` or LLM SDKs) inside the domain.
* The domain defines **contracts (ports)** that the outside world must satisfy.

## 2. Ports (Contracts via Traits)
To define output ports (for example, LLM or database communication), use `async_trait`.

```rust
use async_trait::async_trait;

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn generate(&self, prompt: &str) -> Result<String,DomainError>;
}
```

## 3. Adapters (Infrastructure)
The `src/infrastructure/` layer implements domain traits.
* This is where HTTP requests, disk access, and provider integrations live.
* Infrastructure depends on domain. Domain must never depend on infrastructure.

## 4. Dependency Injection (DI)
Do not use heavy DI frameworks. Pass dependencies through constructors using generics or trait objects (`Arc<dyn Trait>`).

```rust
// Correct: inject the interface, not the concrete implementation
pub struct Agent {
    id: AgentId,
    llm: Arc<dyn LlmProvider>, 
}

impl Agent {
    pub fn new(id: AgentId, llm: Arc<dyn LlmProvider>) -> Self {
        Self { id, llm }
    }
}
```

## 5. Error Mapping
* **In Domain:** Use `thiserror` to define specific error enums.
* **In Application/CLI:** Use `anyhow` to aggregate infrastructure errors and surface actionable feedback.
