# TASK 042: Multi-Model Harness — Per-Agent Models as Maestro's Routing Responsibility

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** Every persona-agent currently shares a single `Arc<dyn LlmProvider>` cloned into each `PersonaRuntimeRole` (`src/application/persona_operations.rs`). Adapters bind to `provider.models.first()`, so only the first model per provider is ever used and `system.default_model` is validated then discarded (`src/infrastructure/llm/*_adapter.rs`, `provider_registry.rs::resolve_default`). Configuration is read from `maestro/config.yaml` (`src/application/config.rs`).
* **Context Anchors:** #file:src/application/persona_operations.rs, #file:src/infrastructure/llm/provider_registry.rs, #file:src/application/config.rs, #file:docs/Maestro_Manifesto/ARCHITECTURE.md
* **Expected Output:** A Maestro AI Harness where (1) each agent can run a different AI model, (2) any model in the catalog can be assigned to any agent, (3) configuration lives in `config.yml` (with backward-compatible `config.yaml` fallback), and (4) every available model is declared in `config.yml`. Model→agent routing becomes the orchestrator's responsibility via a new `ModelRouter` (Maestro's delegation-routing authority), and the `models.first()` binding bug is fixed at the root.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* The Maestro persona stays immutable and the product stays language/architecture-neutral: **no model/provider field is added to the `Persona` domain type**. Assignments live in `config.yml`, keyed by persona name.
* Routing is **deterministic** — config declares `persona → (provider, model)`; the `ModelRouter` applies it. No LLM call participates in routing, heartbeat, or narration.
* Per-agent failure isolation is preserved: an unresolvable route falls back to the system default; a single bad model never crashes the harness.
* No production `unwrap()`/`expect()`/`panic!()`; `thiserror` in domain/application, `anyhow` only at the CLI/presentation boundary; `tracing` for observability.
* Edition-2018 positional format arguments only (no inline `{var}` captures).
* Architecture boundaries respected: `ModelRouter` is an application type; the registry (infrastructure) constructs it from config; presentation wires it in.

## 3. ACCEPTANCE CRITERIA
* AC1: `config.yml` accepts an optional top-level `agents:` map of `persona-name -> { provider, model }`; `AppConfig::validate` hard-fails when an assignment references an unknown provider or a model not declared under that provider.
* AC2: Provider adapters bind to an **explicitly requested model** (not `models.first()`); `ProviderRegistry::resolve(provider, model, config)` returns an adapter bound to that model, and `resolve_default` honors `system.default_model`.
* AC3: A new `ModelRouter` (application) resolves a per-persona `Arc<dyn LlmProvider>`, falling back to the system default for unassigned personas; agents sharing a `(provider, model)` share one adapter instance.
* AC4: Registration functions (`registrations_from_default_personas`/`_governance`/`_selected_personas`) route each persona through the `ModelRouter`; the live pipeline logs the routing table (`persona -> provider:model`) via `tracing`.
* AC5: `ConfigLoader` prefers `config.yml` and falls back to `config.yaml` with a deprecation warning, for both the local `maestro/` and the global `~/.config/maestro/` paths; writers (`init-config`, `init`, readiness auto-config) emit `config.yml`.
* AC6: The shipped example is `maestro/config.yml.example` (with an `agents:` example); the dead `maestro/config.toml.example` is removed; `packaging/omarchy` uses `config.yml`; docs reflect the new file name and per-agent assignment.
* AC7: All quality gates pass with new tests covering per-agent routing, default fallback, shared-adapter dedup, `agents` validation hard-fail, `config.yml` preference + `.yaml` deprecation, and adapter model binding.

## 4. VALIDATION COMMANDS
* `cargo fmt --all -- --check`
* `cargo clippy --all-targets -- -D warnings`
* `cargo test --all-targets`
* `scripts/quality-gate.sh`
* `scripts/check-doc-links.sh`

## 5. INCREMENT MAP
* INC1: `config.rs` — add `agents: HashMap<String, AgentModelAssignment>` (`#[serde(default)]`) + `AgentModelAssignment { provider, model }`; extend `validate()` with the hard-fail rule. (AC1)
* INC2: Registry + 4 adapters — `ProviderFactory` carries an explicit `model`; adapters bind the requested model; add `resolve(provider, model, config)`; fix `resolve_default`; add `build_model_router(config)`. (AC2)
* INC3: `model_router.rs` (new application module) + registration functions take `&ModelRouter`. (AC3, AC4)
* INC4: Presentation wiring — `Commands::Run` and `run_tui_with_runtime` build and pass the router; trace the routing table. (AC4)
* INC5: `config.yml` migration — loader preference + fallback, writers, example rename, remove `config.toml.example`, packaging. (AC5, AC6)
* INC6: Docs sync (README, COMMANDS_AND_PANELS, onboarding guides). (AC6)
* INC7: Tests + quality gate + evidence. (AC7)

## 5b. VALIDATION EVIDENCE
Executed at completion (all green):
* `cargo fmt --all` — applied, tree clean.
* `cargo clippy --all-targets -- -D warnings` — `Finished` with no warnings.
* `cargo test --all-targets` — `157 passed; 0 failed` (baseline was 146; +11 from `ModelRouter` (2), config `agents`/`config.yml` resolution (5), and registry explicit-model/router dedup (4)).
* `bash scripts/quality-gate.sh` — `Quality gate passed` (fmt, clippy, build, test, doc-link integrity).
* `bash scripts/check-doc-links.sh` — `Documentation link integrity check passed.`

New/updated tests by acceptance criterion:
* AC1 — `config::tests::{accepts_agent_assignment_to_existing_provider_model, rejects_agent_referencing_unknown_provider, rejects_agent_referencing_unknown_model}`.
* AC2 — `provider_registry::tests::{resolve_binds_requested_model_not_first, resolve_rejects_model_absent_from_provider}` (proves the `models.first()` binding bug is fixed).
* AC3/AC4 — `model_router::tests::{routes_assigned_persona_to_its_model_and_others_to_default, uniform_router_sends_everyone_to_one_provider}` and `provider_registry::tests::{build_model_router_assigns_agent_models_and_defaults_others, build_model_router_shares_one_adapter_for_identical_assignments}`.
* AC5 — `config::tests::{existing_config_in_prefers_yml_over_legacy_yaml, existing_config_in_falls_back_to_legacy_yaml}`; `readiness::tests::auto_bootstrap_creates_valid_yaml_config` now asserts `config.yml`.
* AC6 — `maestro/config.yml.example` shipped with `agents:` example; `maestro/config.toml.example` and `packaging/omarchy/config.toml` removed; packaging (`PKGBUILD`, debian `postinst`, `build-deb.sh`) and smoke-test scripts/docs emit `config.yml`.

## 6. RESIDUAL RISKS
* Capabilities remain per-provider (not per-model) in this task; the `ModelSpec.context_window` vs `ProviderCapabilities.max_context_tokens` redundancy is documented but out of scope.
* Capability-aware automatic model selection (matching task needs to model capabilities) is a deliberate future extension; this task ships deterministic, config-declared routing only.
