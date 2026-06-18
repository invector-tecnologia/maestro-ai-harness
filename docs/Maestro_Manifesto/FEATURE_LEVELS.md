# Feature Levels

This document separates Maestro capabilities into feature maturity levels.

## Foundational Level
Target: first-time users and initial project setup.

Features:
- `maestro init`, `maestro init-config`, and `maestro validate-config`.
- Readiness checks and baseline doctor validations.
- Initial markdown governance scaffold.
- Single-provider local setup and config loading.

## Core Level
Target: daily multi-agent operation.

Features:
- Multi-agent runtime (`observe`, `think`, `act`).
- TUI dashboard and command center.
- Persona model with interaction matrix and handoff rules.
- Provider registry and default provider resolution.
- Mandatory quality gates and structured tracing.

## Advanced Level
Target: teams scaling adoption and governance.

Features:
- Guided onboarding with resume state machine.
- Accessibility controls (ASCII fallback) and local telemetry opt-in.
- Cross-platform packaging and smoke-test workflows.
- Operational wizard hardening and progressive checkpointing.

## Enterprise Level
Target: broad organizational rollout.

Features (roadmap-oriented):
- Compliance reporting and policy extension.
- Broader provider ecosystem and operational governance exports.
- Expanded reliability automation and audit analytics.

## Traceability
- Runtime and personas: `src/application/agent_runtime.rs`, `src/application/persona_operations.rs`.
- TUI and onboarding: `src/presentation/tui/mod.rs`.
- CLI operations: `src/presentation/cli/mod.rs`.
- Packaging and quality gates: `scripts/quality-gate.sh`, `scripts/build-*.sh`.
