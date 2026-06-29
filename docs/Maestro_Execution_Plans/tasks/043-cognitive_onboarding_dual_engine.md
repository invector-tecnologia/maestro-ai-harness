# TASK 043: Cognitive Onboarding — Dual-Engine Interview and Model-Availability SENSE

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** Onboarding/interview today runs a scripted `InterviewBot`
  (`src/application/interview_bot.rs`) in parallel with the live Maestro persona
  cognitive loop (`PersonaRuntimeRole` observe/think/act in
  `src/application/persona_operations.rs`). Both publish as "Maestro", producing a
  dual voice, self-contradiction ("I'm just a text AI" vs. file-authoring persona),
  turn-counter drift, and out-of-order Q&A. Model availability is only inferred from
  a TCP reachability check (`src/application/readiness.rs::endpoint_is_reachable`);
  there is no real probe of whether a model is loaded/served. File authoring already
  flows through `MarkdownGovernance` (`src/application/markdown_governance.rs`).
* **Context Anchors:** #file:src/domain/ports/role.rs,
  #file:src/domain/ports/llm_provider.rs, #file:src/application/interview_bot.rs,
  #file:src/application/persona_operations.rs, #file:src/application/readiness.rs,
  #file:src/application/markdown_governance.rs, #file:src/presentation/tui/interview.rs,
  #file:docs/Maestro_Manifesto/ARCHITECTURE.md
* **Expected Output:** Onboarding becomes a first-class cognitive agent on the
  existing `observe → think → act` loop. A new SENSE stage (`LlmProvider::probe`)
  detects whether the configured model is actually available and drives a dual-engine
  switch: Option B (single-voice LLM interview) when a model is loaded; Option A
  (deterministic guided setup) when not, auto-promoting to B once setup succeeds.
  Every engine performs full CRUD (Create/Read/Update/Edit/Delete) on `maestro/`
  through governance, with Delete implemented as archive. The canonical cognitive
  pattern (SENSE → OBSERVE → THINK → ACT → AUDIT → DELIVER) is documented and applied
  across interview, workspace orchestration, and RAG.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* Reuse the existing `Role` cognitive loop and `AgentRuntime` orchestration; do not
  introduce a parallel control flow. Exactly one producer of "Maestro" messages at a
  time during the interview.
* The Maestro persona stays immutable; persona/skill mutations targeting Maestro are
  rejected by governance and must remain rejected.
* SENSE is deterministic and side-effect free beyond a read-only health request; the
  probe must never author files or mutate state.
* Delete semantics = `archive_document` (recoverable), never hard filesystem delete.
* No production `unwrap()`/`expect()`/`panic!()`; `thiserror` in domain/application,
  `anyhow` only at the CLI/presentation boundary; `tracing` for observability.
* Edition-2018 positional format arguments only (no inline `{var}` captures) where the
  surrounding file already follows that rule.
* Architecture boundaries: `ProviderStatus` + `probe` are domain-port concerns; the
  health-check HTTP lives in infrastructure adapters; engine/role orchestration is
  application; TUI wiring is presentation.

## 3. ACCEPTANCE CRITERIA
* AC1: `LlmProvider` exposes `async fn probe(&self) -> ProviderStatus` with
  `ProviderStatus { Available, Unreachable, Unauthorized, ModelMissing }`; the default
  implementation maps a minimal completion ping to `Available`/`Unreachable`, keeping
  existing providers object-safe and backward compatible.
* AC2: Each built-in adapter (Ollama, OpenAI, Anthropic, Gemini) overrides `probe`
  with a real health check — Ollama `GET /api/tags`, OpenAI `GET /models`, Anthropic
  `GET /v1/models` (with `x-api-key` + `anthropic-version`), Gemini access-token
  verification — mapping connection failure → `Unreachable`, 401/403 → `Unauthorized`,
  success with the configured model present → `Available`, success without it →
  `ModelMissing`.
* AC3: `ReadinessState` carries a `model_loaded: bool`; an async
  `run_checks_with_probe`/`probe_default_provider` resolves the default provider and
  sets `model_loaded` from the probe, while the synchronous `run_checks` stays
  unchanged for existing callers.
* AC4: A dual-engine switch selects Option B when `model_loaded` is true and Option A
  otherwise; Option A guides config setup, re-senses, and auto-promotes to B when the
  probe reports `Available`.
* AC5: Option B is a single-voice cognitive interview — the generic persona runtime no
  longer double-posts during interview; turn counting has one owner and Q/A ordering is
  deterministic; the prompt asserts file-authoring capability (no "text-only AI"
  disclaimer).
* AC6: All engines author through governance with full CRUD; Delete archives to
  `maestro/archive/`; Maestro persona/skills remain immutable.
* AC7: The canonical cognitive pattern is documented
  (`docs/Maestro_Manifesto/reference/COGNITIVE_PATTERN.md` + `ARCHITECTURE.md`) and the
  RAG and workspace flows are mapped onto it without breaking their public APIs.
* AC8: All quality gates pass with new tests covering probe→status mapping, readiness
  `model_loaded`, engine selection and A→B promotion, single-voice behavior, CRUD
  round-trips through governance, and Maestro immutability.

## 4. VALIDATION COMMANDS
* `cargo fmt --all -- --check`
* `cargo clippy --all-targets -- -D warnings`
* `cargo test --all-targets`
* `scripts/quality-gate.sh`
* `scripts/check-doc-links.sh`

## 5. INCREMENT MAP (PR sequence under this task)
* PR1: SENSE foundation — `ProviderStatus` + `probe` default (domain port); per-adapter
  health checks (4 adapters) + `endpoint_utils` URL/catalog helpers; readiness
  `model_loaded` + async probe entry. (AC1, AC2, AC3)
* PR2: Cognitive interview core — `InterviewEngine`, `FileOp`, `DirectiveFileChange`,
  capability-aware prompt, `propose` JSON parsing, `MaestroInterviewRole` in
  `persona_operations.rs`. (AC5)
* PR3: Option A guided setup + auto-promotion + CRUD-through-governance apply path.
  (AC4, AC6)
* PR4: TUI wiring — engine selection, suppress double-post, op-plan approval modal,
  status line. (AC4, AC5)
* PR5: RAG cognitive wrapper (non-breaking) + SENSE fallback. (AC7)
* PR6: Workspace SENSE pre-stage + `COGNITIVE_PATTERN.md` + docs sync. (AC7)
* PRn: Tests + quality gate + evidence per increment. (AC8)

## 5b. VALIDATION EVIDENCE
_To be filled at completion of each increment with executed commands and outcomes._

### PR1 — SENSE foundation (AC1, AC2, AC3)
* `cargo fmt --all` — clean.
* `cargo clippy --all-targets -- -D warnings` — no warnings.
* `cargo test --all-targets` — 164 passed; 0 failed. New tests:
  `domain::ports::llm_provider::tests::default_probe_reports_available_when_completion_succeeds`,
  `application::readiness::tests::run_checks_leaves_model_loaded_false`,
  `application::readiness::tests::probe_default_provider_is_unreachable_without_config`,
  `infrastructure::llm::endpoint_utils::tests::{derives_ollama_tags_endpoint,
  derives_openai_models_endpoint, derives_anthropic_models_endpoint,
  matches_model_in_catalog_with_tag_tolerance}`.
* `scripts/check-doc-links.sh` — link integrity passed.

## 6. RESIDUAL RISKS
* Gemini model enumeration is awkward against a `generateContent` endpoint; PR1 verifies
  access-token acquisition (auth) rather than a full catalog listing and documents the
  gap. A future increment can adopt `models.list` when the endpoint shape allows.
* The full cross-cutting retrofit spans domain, infrastructure, application,
  presentation, and docs; it is delivered as the small PR sequence above to keep merges
  reviewable and CI-green, per the repository's small-PR governance.
