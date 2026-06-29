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

### PR2 — Cognitive interview core (AC5)
* Added `InterviewEngine { LlmDriven, GuidedSetup }` with `from_provider_status` /
  `from_model_loaded` selectors, `FileOp { Create, Read, Update, Edit, Delete }`,
  `DirectiveFileChange`, the `parse_directive_proposals` JSON parser
  (`ProposalParseError`), and `maestro_capability_preamble()` in
  `src/application/interview_bot.rs`.
* Added single-voice `MaestroInterviewRole` (capability-aware prompt) on the existing
  `observe → think → act` loop in `src/application/persona_operations.rs`.
* `cargo fmt --all` — clean.
* `cargo clippy --all-targets -- -D warnings` — no warnings.
* `cargo test --all-targets` — 173 passed; 0 failed. New tests:
  `interview_engine_follows_provider_status`, `file_op_parses_full_crud_vocabulary`,
  `parses_fenced_json_proposal_with_surrounding_prose`, `parses_bare_json_array_proposal`,
  `proposal_parser_rejects_write_without_content`,
  `proposal_parser_rejects_skill_without_persona`,
  `proposal_parser_reports_missing_json_block`, `capability_preamble_asserts_file_authoring`,
  `maestro_interview_role_is_single_capability_aware_voice`.
* `scripts/check-doc-links.sh` — link integrity passed.

### PR3 — CRUD-through-governance apply path + Option A guided setup (AC4, AC6)
* Added `SCOPE_FIELD_ALIASES` / `SCOPE_REQUIRED_FIELDS` consts and
  `validate_scope_overwrite` (validates fields without enforcing scope sequence, so
  existing scopes can be edited/updated) in `src/application/markdown_governance.rs`.
* Added `AppliedChange { Written, Read, Archived }` and
  `apply_directive_change(&MarkdownGovernance, &DirectiveFileChange)` implementing the
  full CRUD surface through governance: Create/Edit/Update validate then write, Read
  returns content, Delete archives (recoverable, never a hard delete). Maestro persona
  and skill remain immutable. Added `guided_setup_actions(ProviderStatus)` (Option A
  steps, empty when a model is available) and `reassess_engine(ProviderStatus)` for
  auto-promotion A→B once a model responds, in `src/application/interview_bot.rs`.
* `cargo fmt --all` — clean.
* `cargo clippy --all-targets -- -D warnings` — no warnings.
* `cargo test --all-targets` — 177 passed; 0 failed. New tests:
  `apply_directive_change_creates_reads_and_archives_persona`,
  `apply_directive_change_rejects_immutable_maestro_persona`,
  `guided_setup_actions_present_only_when_model_unavailable`,
  `reassess_engine_auto_promotes_when_model_becomes_available`.
* `scripts/check-doc-links.sh` — link integrity passed.

### PR4 — TUI engine wiring + double-voice suppression (AC4, AC5)
* SENSE stage in `run_tui` (`src/presentation/tui/mod.rs`) now probes the configured
  provider (`probe_default_provider`) before the interview, selects the engine via
  `InterviewEngine::from_provider_status`, records it on the session, logs the chosen
  engine + model online/offline, and surfaces `guided_setup_actions` steps when offline
  (Option A).
* Added `engine: InterviewEngine` and `maestro_online: bool` to `InterviewSession`
  (Default impl updated; `InterviewEngine` now derives `Default` = `GuidedSetup`).
* `enqueue_interview_question` (`src/presentation/tui/interview.rs`) suppresses the
  scripted Maestro bus publish when `maestro_online` (Option B), leaving the live
  Maestro role as the single voice in shared history — removing the dual-voice
  double-post.
* `render_maestro_panel` (`src/presentation/tui/render.rs`) status line now shows the
  engine label and model online/offline state (Thinking vs Listening).
* `cargo fmt --all` — clean.
* `cargo clippy --all-targets -- -D warnings` — no warnings.
* `cargo test --all-targets` — 179 passed; 0 failed. New tests:
  `maestro_panel_shows_llm_driven_engine_when_model_online`,
  `maestro_panel_shows_guided_setup_engine_when_model_offline`.
* `scripts/check-doc-links.sh` — link integrity passed.

## 6. RESIDUAL RISKS
* Gemini model enumeration is awkward against a `generateContent` endpoint; PR1 verifies
  access-token acquisition (auth) rather than a full catalog listing and documents the
  gap. A future increment can adopt `models.list` when the endpoint shape allows.
* The full cross-cutting retrofit spans domain, infrastructure, application,
  presentation, and docs; it is delivered as the small PR sequence above to keep merges
  reviewable and CI-green, per the repository's small-PR governance.
* PR4 engine selection is evaluated once at interview launch. Auto-promotion A→B
  (`reassess_engine`) currently takes effect on the next interview start after the model
  comes online; live mid-session promotion is deferred to a later increment.

