# TASK 003: The AI Contract (LlmProvider Port)

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** None (boundary interface creation)
* **Context Anchors:** #file:docs/Maestro_Manifesto/CONVENTIONS.md, #file:docs/Maestro_Manifesto/ARCHITECTURE.md
* **Expected Output:** `src/domain/ports/llm_provider.rs` containing the async trait.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* The domain must not depend on infrastructure libraries such as `reqwest` or HTTP clients.
* Must use the `#[async_trait]` macro to enable thread-safe async signatures (`Send + Sync`).
* Error handling must be tied to `RoleError` or a new domain error enum via `thiserror`.

## 3. EXECUTION PROMPT (Paste into Copilot Chat)
"""
Act as a Senior Rust Software Architect.
Goal: Define the AI communication port in the Domain layer, decoupling the Core from any specific provider (Ollama, OpenAI, etc.).

Based on #file:docs/Maestro_Manifesto/CONVENTIONS.md and #file:docs/Maestro_Manifesto/ARCHITECTURE.md, create `src/domain/ports/llm_provider.rs`.

Before generating code, open a `<reasoning>` block and plan how to expose a function signature that accepts a text prompt and returns a String, while enforcing thread safety (`Send + Sync`) for use in Tokio tasks without leaking infrastructure details into the domain.

Follow these structure specifications:
1. Import `async_trait::async_trait` and `RoleError` from `crate::domain::ports::role::RoleError`.

2. Define the `LlmProvider` trait:
   ```rust
   #[async_trait]
   pub trait LlmProvider: Send + Sync {
       async fn generate_completion(&self, prompt: &str) -> Result<String, RoleError>;
   }
   ```

3. Write an internal `#[cfg(test)]` module containing a compile-time check proving that `Arc<dyn LlmProvider>` is Object Safe and can be safely shared across threads.

4. Expose the new module hierarchically in `ports/mod.rs`.

[Cohesion Mechanism]: Run a Self-Consistency Check confirming that no external network dependency or concrete struct was created in this file. It must be 100% abstract.

Return ONLY the complete code block for the file. No introduction. Be direct.
"""
