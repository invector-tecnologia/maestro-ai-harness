# Maestro Commands and Panels

## CLI Commands
- `maestro run [--config <path>] [--duration-ms <n>]`: Runs the multi-agent cycle for a bounded duration.
- `maestro tui [--config <path>]`: Opens the interactive TUI.
- `maestro onboarding [--config <path>] [--mode fast|detailed]`: Starts onboarding directly.
- `maestro onboarding --mode fast`: Starts with safe defaults for quicker setup.
- `maestro onboarding --mode detailed`: Starts the guided interview for full control.
- `maestro validate-config [--config <path>]`: Validates runtime configuration.
- `maestro list-agents`: Lists registered default personas.
- `maestro doctor [--config <path>]`: Checks environment, config, and governance structure.
- `maestro scaffold-markdown`: Creates initial markdown governance artifacts.
- `maestro init-config`: Generates local config template in `./maestro/config.toml`.
- `maestro init <project-name>`: Bootstraps a new project with governance folders and config.
- `maestro logout`: Clears provider credentials from local secure storage.

## TUI Panels
- Agent Panel: shows agent names and lifecycle status (`idle`, `observe`, `think`, `act`, `error`).
- Logs Panel: shows chronological runtime events and diagnostics.
- Command Input: receives slash commands and user prompts.
- Command Center: `/help` and `/onboarding` actions with guided hints.

## TUI Command-Center Commands
- `/help`
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
