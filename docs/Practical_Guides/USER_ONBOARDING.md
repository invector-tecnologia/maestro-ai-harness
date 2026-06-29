# User Onboarding Guide

## Goal
This flow introduces Maestro to first-time users and explains the essential commands to operate the TUI.

## Prerequisites
- Valid config at `~/.config/maestro/config.yml` (or `XDG_CONFIG_HOME/maestro/config.yml`). A legacy `config.yaml` is still accepted with a deprecation warning.
- Default provider reachable according to your configuration.

## How To Start
```bash
maestro onboarding --mode detailed --config ~/.config/maestro/config.yml
```

To start faster with safe defaults:
```bash
maestro onboarding --mode fast --config ~/.config/maestro/config.yml
```

You can also start in normal mode and let Maestro resume saved onboarding state:
```bash
maestro tui --config ~/.config/maestro/config.yml
```

## Flow
1. Fast mode starts with safe defaults and minimal prompts.
2. Detailed mode opens the guided interview and captures more setup decisions.
3. `continue` advances onboarding steps.
4. `skip` redirects immediately to project onboarding.
5. Onboarding state is saved locally for future sessions.
6. After project onboarding is completed, Maestro returns automatically to standard TUI mode.

## Onboarding Engines (Model-Availability SENSE)
Before the interview starts, Maestro probes the configured default provider to sense
whether a model is actually available, then selects an engine automatically:
- **LLM-driven interview (model online):** Maestro conducts the onboarding as a single
  cognitive voice and can author your configuration files directly. It drives the
  conversation live — guiding, processing each answer, and confirming — and when it has
  enough context it proposes governed file changes (project scopes plus the persona/skill
  additions they imply). Proposals are staged for your approval, never written silently:
  press `y` to write every change through markdown governance, or `n` to keep talking and
  let Maestro refine. The immutable Maestro persona is never a proposal target. Once
  changes are written, Maestro installs the full agent team and switches you to Workspace
  Mode so your next instruction starts the build.
- **Guided setup (model offline):** when no model is reachable, Maestro switches to a
  scripted guided setup that lists the steps to get a provider running, then
  auto-promotes to the LLM-driven interview the next time onboarding starts once a model
  becomes available.

The Maestro panel status line shows the active engine and whether the model is online or
offline so you always know which mode you are in.

## Useful TUI Commands
- `/help`
- `/onboarding status`
- `/onboarding restart user`
- `/onboarding restart project`
- `/onboarding skip`
- `/onboarding continue`
- `/architect` (`/core` alias) — open Architect Mode directive governance to Create/Edit/Update/Delete personas, persona skills, and scopes.
- `/monitor` — return to Workspace Mode (the runtime monitor).

## Modes
Maestro operates in two intent-driven modes:
- **Workspace Mode** is the runtime monitor. The Maestro agent orchestrates the demand end to end — plan, delegate to each worker persona, audit every contribution, then deliver a synthesized result — with live narration and a heartbeat while any worker runs longer than 5 seconds.
- **Architect Mode** is the directive governance home. Launch it directly with `maestro directives`, or open it from the TUI with `/architect` (`/core` alias). The Maestro persona is immutable and can never be a directive target.

Note:
- If onboarding is inactive, `/onboarding skip` and `/onboarding continue` are not routed to agents; the TUI suggests `/onboarding restart user|project`.

## Troubleshooting
- If the provider is unavailable, validate configuration and endpoint before starting project onboarding.
- For ASCII-only rendering:
```bash
MAESTRO_ASCII_ONLY=1 maestro tui --config ~/.config/maestro/config.yml
```
- To enable local opt-in telemetry:
```bash
MAESTRO_TELEMETRY=1 maestro tui --config ~/.config/maestro/config.yml
```
