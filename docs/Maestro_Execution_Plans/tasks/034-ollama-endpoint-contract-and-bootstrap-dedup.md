# TASK 034: Ollama Endpoint Contract and Bootstrap Dedup

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** Current YAML config/bootstrap paths and provider adapter endpoint normalization.
* **Context Anchors:** #file:docs/Maestro_Manifesto/ARCHITECTURE.md, #file:docs/Maestro_Manifesto/CONVENTIONS.md
* **Expected Output:** Canonical Ollama endpoint contract and removal of duplicated normalization/bootstrap template code paths.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* Endpoint behavior must be deterministic across generated config, docs, and runtime adapters.
* Do not introduce breaking changes to non-Ollama providers.
* Keep bootstrap behavior reversible and test-covered.

## 3. ACCEPTANCE CRITERIA
* AC1: Shared endpoint utility module centralizes chat and embeddings normalization.
* AC2: Ollama chat endpoint normalization is consistent with generated defaults and avoids startup probe 404 regressions.
* AC3: `DEFAULT_CONFIG_TEMPLATE` is reused by readiness bootstrap; duplicated inline YAML bootstrap string is removed.
* AC4: Docs and packaged default configs align with canonical Ollama endpoint contract.
* AC5: Existing quality gates pass with updated/added tests.

## 4. VALIDATION COMMANDS
* `cargo fmt --check`
* `cargo clippy --all-targets --all-features -- -D warnings`
* `cargo test`
