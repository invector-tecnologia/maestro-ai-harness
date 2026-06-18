# User Onboarding Guide

## Goal
This flow introduces Maestro to first-time users and explains the essential commands to operate the TUI.

## Prerequisites
- Valid config at `~/.config/maestro/config.toml` (or `XDG_CONFIG_HOME/maestro/config.toml`).
- Default provider reachable according to your configuration.

## How To Start
```bash
maestro onboarding --mode user --config ~/.config/maestro/config.toml
```

You can also start in normal mode and let Maestro resume saved onboarding state:
```bash
maestro tui --config ~/.config/maestro/config.toml
```

## Flow
1. Welcome screen with a feature overview.
2. `continue` advances onboarding steps.
3. `skip` redirects immediately to project onboarding.
4. Onboarding state is saved locally for future sessions.
5. After project onboarding is completed, Maestro returns automatically to standard TUI mode.

## Useful TUI Commands
- `/help`
- `/onboarding status`
- `/onboarding restart user`
- `/onboarding restart project`
- `/onboarding skip`
- `/onboarding continue`

Note:
- If onboarding is inactive, `/onboarding skip` and `/onboarding continue` are not routed to agents; the TUI suggests `/onboarding restart user|project`.

## Troubleshooting
- If the provider is unavailable, validate configuration and endpoint before starting project onboarding.
- For ASCII-only rendering:
```bash
MAESTRO_ASCII_ONLY=1 maestro tui --config ~/.config/maestro/config.toml
```
- To enable local opt-in telemetry:
```bash
MAESTRO_TELEMETRY=1 maestro tui --config ~/.config/maestro/config.toml
```
