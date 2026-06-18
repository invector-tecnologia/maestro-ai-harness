# TASK 009: Ollama Reference Adapter

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** Provider registry and provider contracts.
* **Context Anchors:** #file:docs/Maestro_Manifesto/CONVENTIONS.md, #file:docs/Maestro_Manifesto/ARCHITECTURE.md
* **Expected Output:** First working connector for an end-to-end flow.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* Timeouts are mandatory.
* Network errors must be mapped to typed domain errors.
* HTTP implementation details must not leak into the domain.

## 3. EXECUTION PROMPT (Paste into Copilot Chat)
"""
Act as an LLM Integration Engineer in Rust.
Goal: Deliver a robust reference connector for the local Ollama environment.

Before generating code, open a `<reasoning>` block and evaluate timeout, retry, and error-mapping risks.

Execute:
1. Implement a connector for the local OpenAI-compatible endpoint (Ollama).
2. Map the response to a domain contract completion.
3. Add `tracing` instrumentation for latency, status, and failures.
4. Add tests for:
   - Successful completion.
   - Timeout.
   - Connection error.
   - Invalid payload.

[Cohesion Mechanism]:
- Confirm that infrastructure is isolated in the correct layer.

Return ONLY the modified code blocks in Markdown. No introduction.
"""
