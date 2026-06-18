# TASK 008: Provider Registry and Connector Factory

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** #file:src/domain/ports/llm_provider.rs and the configuration schema.
* **Context Anchors:** #file:docs/Maestro_Manifesto/ARCHITECTURE.md, #file:docs/Maestro_Manifesto/CONVENTIONS.md
* **Expected Output:** A registry for provider selection driven by configuration.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* No infrastructure dependency in the domain.
* Provider resolution failures must be typed.
* The fallback flow must be predictable and auditable.

## 3. EXECUTION PROMPT (Paste into Copilot Chat)
"""
Act as an Infrastructure Architect for AI systems.
Goal: Allow any provider to be used without modifying the domain.

Before generating code, open a `<reasoning>` block and validate the boundaries between domain and infrastructure.

Execute:
1. Implement a provider registry keyed by logical name.
2. Implement a connector factory driven by the loaded configuration.
3. Handle the following cases:
   - Provider not found.
   - Model missing from the provider.
   - Inconsistent configuration.
4. Add unit tests for resolution success and each error case.

[Cohesion Mechanism]:
- Confirm pluggable extension for new providers without touching the domain.

Return ONLY the modified code blocks in Markdown. No introduction.
"""
