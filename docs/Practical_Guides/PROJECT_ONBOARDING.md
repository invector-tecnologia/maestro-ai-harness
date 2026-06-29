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
maestro onboarding --mode detailed --config ~/.config/maestro/config.yml
```

For a quicker path with safe defaults:
```bash
maestro onboarding --mode fast --config ~/.config/maestro/config.yml
```

## Guided Flow
1. Setup welcome screen.
2. Scope wizard (step 1/3).
3. Persona wizard (step 2/3).
4. Skill wizard (step 3/3).
5. Completion screen with project ready for use.
6. Automatic return to Workspace Mode (the runtime monitor: ① Input, ② Orchestration, ③ Agent Activity, ④ Readiness).

Fast mode skips the guided interview and uses defaults when the workspace is already close to ready.

## Onboarding Engines (Model-Availability SENSE)
Maestro probes the configured default provider before the interview and picks an engine
automatically:
- **LLM-driven interview (model online):** a single cognitive voice runs the interview
  and authors scope, persona, and skill files through markdown governance.
- **Guided setup (model offline):** when no model is reachable, Maestro lists the steps
  to bring a provider online and re-senses on the next start, auto-promoting to the
  LLM-driven interview once a model is available.

The active engine and model online/offline state are shown in the Maestro panel status
line.

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
maestro doctor --config ~/.config/maestro/config.yml
maestro directives --config ~/.config/maestro/config.yml
maestro run --config ~/.config/maestro/config.yml --duration-ms 500
```

Use `maestro directives` (or `/architect` inside the TUI) to govern personas, persona skills, and scopes; return to the monitor with `/monitor`.

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
