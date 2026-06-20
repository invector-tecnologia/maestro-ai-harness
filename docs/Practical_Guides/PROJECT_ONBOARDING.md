# Project Onboarding Guide

## Goal
Set up a new Maestro project using the guided Ratatui flow.

## Recommended Start
```bash
maestro init my-project
```

This command scaffolds the project and opens the onboarding interview mode automatically.

For non-interactive automation:
```bash
maestro init my-project --no-tui
```

## How To Start Directly
```bash
maestro onboarding --mode detailed --config ~/.config/maestro/config.toml
```

For a quicker path with safe defaults:
```bash
maestro onboarding --mode fast --config ~/.config/maestro/config.toml
```

## Guided Flow
1. Setup welcome screen.
2. Scope wizard (step 1/3).
3. Persona wizard (step 2/3).
4. Skill wizard (step 3/3).
5. Completion screen with project ready for use.
6. Automatic return to normal TUI mode (agent panel).

Fast mode skips the guided interview and uses defaults when the workspace is already close to ready.

## Validations
- Required fields block progress.
- Scope requires a 3-digit numeric prefix (for example: `001`).
- Persona/scope/skill creation follows markdown governance validation.

## Expected Result
After completion, the project has initial artifacts in:
- `maestro_scopes`
- `maestro_personas`
- `maestro_skills`

## Recommended Commands After Completion
```bash
maestro list-agents
maestro doctor --config ~/.config/maestro/config.toml
maestro run --config ~/.config/maestro/config.toml --duration-ms 500
```

## Troubleshooting
- If the flow is in an unexpected stage, use:
```bash
/onboarding status
/onboarding restart project
```
- To restart user onboarding and return to project onboarding:
```bash
/onboarding restart user
```
