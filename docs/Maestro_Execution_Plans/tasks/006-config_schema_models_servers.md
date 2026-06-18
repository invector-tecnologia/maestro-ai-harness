# TASK 006: Configuration Schema for Models, Servers, and Runtime

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** #file:README.md as a style reference.
* **Context Anchors:** #file:docs/Maestro_Manifesto/CONVENTIONS.md, #file:docs/Maestro_Manifesto/ARCHITECTURE.md
* **Expected Output:** Robust external configuration for providers, models, and execution policies.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* Secrets must never be hardcoded.
* Invalid configuration must fail with a human-readable error.
* Must respect XDG location fallback on Linux.

## 3. EXECUTION PROMPT (Paste into Copilot Chat)
"""
Act as a Principal Platform Engineer in Rust.
Goal: Create a robust external configuration layer for the MLP.

Before generating code, open a `<reasoning>` block and evaluate consistency between schema, defaults, and per-environment overrides.

Execute:
1. Model the schema for:
   - Providers and servers (logical name, endpoint, auth_mode, timeout).
   - Model catalog per provider.
   - Execution policies (retry, limits, concurrency).
2. Implement a config loader with:
   - Explicit path via argument.
   - XDG fallback at `~/.config/maestro`.
   - Secret overrides via environment variables.
3. Implement full schema validation with actionable error messages.
4. Add tests for:
   - Valid file.
   - Missing required fields.
   - Invalid types.
   - Broken cross-references.

[Cohesion Mechanism]:
- Confirm absence of domain-infrastructure coupling.

Return ONLY the modified code blocks in Markdown. No introduction.
"""
