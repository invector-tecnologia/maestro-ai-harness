# User Onboarding Guide

## Goal
This flow introduces Maestro to first-time users and explains the essential commands to operate the TUI.

## Prerequisites
- Valid config at `~/.config/maestro/config.yaml` (or `XDG_CONFIG_HOME/maestro/config.yaml`).
- Default provider reachable according to your configuration.

## How To Start
```bash
maestro onboarding --mode detailed --config ~/.config/maestro/config.yaml
```

To start faster with safe defaults:
```bash
maestro onboarding --mode fast --config ~/.config/maestro/config.yaml
```

You can also start in normal mode and let Maestro resume saved onboarding state:
```bash
maestro tui --config ~/.config/maestro/config.yaml
```

## Flow
1. Fast mode starts with safe defaults and minimal prompts.
2. Detailed mode opens the guided interview and captures more setup decisions.
3. `continue` advances onboarding steps.
4. `skip` redirects immediately to project onboarding.
5. Onboarding state is saved locally for future sessions.
6. After project onboarding is completed, Maestro returns automatically to standard TUI mode.

## Useful TUI Commands
- `/help`
- `/onboarding status`
- `/onboarding restart user`
- `/onboarding restart project`
- `/onboarding skip`
- `/onboarding continue`
- `/edit` (or `/core`) — open Interview Mode directive governance to Create/Edit/Update/Delete personas, persona skills, and scopes.
- `/monitor` — return to Workspace Mode (the runtime monitor).

## Modes
Maestro operates in two intent-driven modes:
- **Workspace Mode** is the runtime monitor. The Maestro agent orchestrates available agents sequentially with live narration and a heartbeat while any agent runs longer than 5 seconds.
- **Interview Mode** is the directive governance home. Launch it directly with `maestro directives`, or open it from the TUI with `/edit`. The Maestro persona is immutable and can never be a directive target.

Note:
- If onboarding is inactive, `/onboarding skip` and `/onboarding continue` are not routed to agents; the TUI suggests `/onboarding restart user|project`.

## Troubleshooting
- If the provider is unavailable, validate configuration and endpoint before starting project onboarding.
- For ASCII-only rendering:
```bash
MAESTRO_ASCII_ONLY=1 maestro tui --config ~/.config/maestro/config.yaml
```
- To enable local opt-in telemetry:
```bash
MAESTRO_TELEMETRY=1 maestro tui --config ~/.config/maestro/config.yaml
```
