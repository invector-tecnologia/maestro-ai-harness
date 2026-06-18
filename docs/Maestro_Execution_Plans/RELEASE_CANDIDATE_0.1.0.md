# Maestro MLP 0.1.0 - Release Candidate Checklist

## Scope
This checklist consolidates evidence from Task 016 for MLP 0.1.0 validation.

## Required gates
- [x] `cargo fmt --all --check`
- [x] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [x] `cargo test --workspace -- --nocapture`

### Consolidated gate evidence
- Script executed: `./scripts/quality-gate.sh`
- Result: `Quality gate passed`
- Current suite: `45 passed; 0 failed`

## MLP requirements coverage
- [x] Complete multi-agent flow (runtime + failure isolation)
  - Evidence: tests in `application::agent_runtime::tests::*`
- [x] Default operational personas for Product/Engineering/UX/DevOps
  - Evidence: test `application::persona_operations::tests::default_personas_collaborate_on_user_message`
- [x] TUI with agent panel, logs, and command input
  - Evidence: tests `presentation::tui::tests::renders_agents_logs_and_input_panels` and `handles_basic_input_flow_and_submit`
- [x] Required persona/scope/skill creation wizards with required-field blocking
  - Evidence: unit tests `presentation::tui::tests::wizard_*`
- [x] External configuration validated (schema, type, and cross references)
  - Evidence: unit tests `application::config::tests::*`
- [x] Provider registry + reference Ollama adapter
  - Evidence: unit tests `infrastructure::llm::provider_registry::tests::*` e `infrastructure::llm::ollama_adapter::tests::*`
- [x] Operational CLI (`run`, `tui`, `validate-config`, `list-agents`, `doctor`, `scaffold-markdown`)
  - Evidence: unit tests `presentation::cli::tests::*`
- [x] Debian packaging prepared (`.deb`) with remove/purge lifecycle
  - Evidence: `packaging/debian/*`, `scripts/build-deb.sh`, `scripts/smoke-test-debian.sh`, `docs/Practical_Guides/SMOKE_TEST_DEBIAN.md`

## Debian validation status
- [ ] Debian smoke test executed in a clean environment with `dpkg`/`dpkg-deb`
  - Expected command:
    - `./scripts/build-deb.sh 0.1.0`
    - `./scripts/smoke-test-debian.sh target/deb/maestro-ai_0.1.0_$(dpkg --print-architecture).deb`
  - Note: in the current automation environment, `dpkg` was not available.

## Release readiness criterion
Release Candidate 0.1.0 is approved when:
1. All required gates pass.
2. Coverage checklist remains 100% checked.
3. Debian smoke test is successfully executed in a clean environment.

## Current RC status
- Quality gates: APPROVED
- MLP functional coverage: APPROVED
- Debian smoke test in clean environment: PENDING (depends on host with `dpkg`/`dpkg-deb`)

## Single gate command
```bash
./scripts/quality-gate.sh
```
