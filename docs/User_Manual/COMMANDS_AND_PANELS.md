# Maestro Commands and Panels

## CLI Commands
- `maestro run [--config <path>] [--duration-ms <n>]`: Runs the multi-agent cycle for a bounded duration.
- `maestro tui [--config <path>]`: Opens the interactive TUI (Workspace Mode monitor).
- `maestro interview [--config <path>]`: Launches the guided onboarding interview.
- `maestro directives [--config <path>]`: Opens Architect Mode directly on the directive governance home (the directives picker select stage).
- `maestro onboarding [--config <path>] [--mode fast|detailed]`: Starts onboarding directly.
- `maestro onboarding --mode fast`: Starts with safe defaults for quicker setup.
- `maestro onboarding --mode detailed`: Starts the guided interview for full control.
- `maestro validate-config [--config <path>]`: Validates runtime configuration.
- `maestro list-agents`: Lists registered default personas.
- `maestro doctor [--config <path>]`: Checks environment, config, and governance structure.
- `maestro scaffold-markdown`: Creates initial markdown governance artifacts.
- `maestro init-config`: Generates local config template in `./maestro/config.yml`.
- `maestro init <project-name> [--no-tui]`: Bootstraps a new project with governance folders and config, then opens onboarding interview mode by default.
- `maestro init <project-name> --no-tui`: Bootstraps and exits without opening the TUI (recommended for scripts/CI).
- `maestro deps check --scope <harness|project|all>`: Validates dependency zones independently.
- `maestro rag <subcommand>`: Runs RAG ingestion/query operations.
- `maestro logout`: Clears provider credentials from local secure storage.

## Modes
Maestro has two intent-driven modes; there is no default landing mode.
- **Workspace Mode** is a lean runtime monitor. Submitting a prompt makes the Maestro agent orchestrate the demand end to end: it **plans** a brief, **delegates** the demand to each worker persona in turn, **audits** every contribution (approved or rejected), and **delivers** a synthesized result with an audit trail. Workers run one at a time, Maestro narrates each phase in real time, and a heartbeat fires at least every 5 seconds while a worker runs longer than 5 seconds. Concurrent prompts are serialized (single-flight), and per-worker failure isolation is preserved (a failing worker is audited `rejected` and the workflow continues).
- **Architect Mode** is the single directive governance home (open it with `/architect`, or the `maestro directives` CLI). It opens on a select stage (the directives picker, grouped by type with the Maestro persona shown read-only), then hands off to a guided authoring interview for Create / Edit / Update / Delete of personas, persona skills, and project scopes. The Project Manager agent writes scope files first; Maestro then reads the written scope and derives the additions each non-Maestro persona needs. The Maestro persona is immutable and can never be a directive target.
- **Persona source of truth.** Personas live as canonical markdown under `maestro/personas/` (`# Name`, `## Purpose`, `## Responsibilities`, `## Deliverables`, `## Operational Instructions`, `## Interaction Matrix` as `Target | Contract | Handoff`, `## Quality Criteria`). Workspace Mode loads its agents from these governed files, so editing a persona in Architect Mode changes the live agent set. If the files are missing or invalid, Maestro safely falls back to the built-in default catalog. `maestro scaffold-markdown` emits this exact schema, and the immutable Maestro orchestrator is always present.
- **Default skills and scope.** The bundled workspace ships a canonical skill for every persona under `maestro/skills/<persona-slug>/` (`## Objective`, `## Triggers`, `## Inputs`, `## Outputs`, `## Constraints`) and a starter project scope under `maestro/scopes/` (`## Objective`, `## Business Scope`, `## Deliverables`, `## Acceptance Criteria`, `## Dependencies`). `maestro scaffold-markdown` regenerates the same defaults idempotently without overwriting existing files. Maestro's own skill is shipped read-only because the orchestrator persona is immutable.

## Workspace Mode Panels
The monitor has four panels with explicit roles and a defined flow (input → orchestration → agent activity → readiness/actions). Press `Tab` to cycle focus deterministically.
- **① Input**: receives slash commands and user prompts.
- **② Orchestration**: chronological runtime narration. Maestro orchestration events appear here, including the `plan`, `delegate`, `audit`, and `deliver` phases (🎼 per transition) and heartbeats (💓 while a worker runs longer than the threshold).
- **③ Agent Activity**: agent names and lifecycle status (`idle`, `observe`, `think`, `act`, `error`).
- **④ Readiness**: readiness checks and recommended next actions.

## TUI Command-Center Commands
- `/help`
- `/architect` — open Architect Mode directive governance (select stage). `/core` is a back-compat alias.
- `/monitor` — return to Workspace Mode.
- `/onboarding status`
- `/onboarding restart user`
- `/onboarding restart project`
- `/onboarding skip`
- `/onboarding continue`
- `/new scope`
- `/new persona`
- `/new skill`

## Panel Content Proposals
- Agent Panel should include provider/model badges and health heartbeat.
- Logs Panel should support filter presets (`errors`, `handoffs`, `governance`).
- Command Input should surface command autocomplete and onboarding context hints.
- Help Panel should show first-run shortcuts and last failed action recovery guidance.
