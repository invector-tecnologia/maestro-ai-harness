# Release Candidate 0.2.0

## Scope
- First-run user onboarding in Ratatui.
- Skip redirect to project onboarding.
- Guided project setup in staged wizards.
- Onboarding command center.
- Onboarding state persistence/resume.
- Local opt-in onboarding telemetry.
- ASCII visual fallback for limited terminals.

## Acceptance checklist
- [x] User onboarding starts automatically on first run.
- [x] `skip` redirects to project onboarding.
- [x] Project flow runs scope -> persona -> skill.
- [x] State resumes after TUI restart.
- [x] `/help` and `/onboarding ...` work as expected.
- [x] Local telemetry is written only when `MAESTRO_TELEMETRY=1`.
- [x] ASCII rendering works with `MAESTRO_ASCII_ONLY=1`.
- [x] `./scripts/quality-gate-onboarding.sh` passes with no failures.

## Evidence
- [x] TUI and CLI tests updated.
- [x] Onboarding documentation updated.
- [x] final checklist validated.

### Gate execution (2026-06-14)
- Command: `./scripts/quality-gate-onboarding.sh`
- Result:
	- `cargo fmt --all --check`: approved
	- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: approved
	- `cargo test --lib -- --nocapture`: approved (55 unit tests)

### Closing Notes
- Project onboarding flow now ends in normal TUI mode (onboarding state cleared to `None`).
- Control commands `/onboarding skip` and `/onboarding continue` are hardened for inactive state.
- Onboarding state persistence errors are now shown in the log panel and reported via local opt-in telemetry.
