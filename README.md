# ⚡ MAESTRO HARNESS FOR AI

**You are the orchestrator. This harness executes your vision.**

Maestro is a **relentless AI command deck**—built in Rust for blazing speed and rock-solid safety. Instead of coordinating human developers, you coordinate, test, and manage a **team of AI agents** to architect and build software on your command.

Fire up the TUI. Define your personas, scopes, and skills. Watch your AI team synthesize, execute, and iterate. No memorized commands. No friction. Just pure orchestration.

<p align="center">
  <img src="https://raw.githubusercontent.com/invector-tecnologia/maestro-ai-harness/main/docs/assets/dream-tui.png" alt="Maestro Dream TUI" width="800">
</p>

**▓▒░ SYNTH PROFILE ░▒▓**

## 🌟 Core Capabilities

Maestro is **Rust-native**. Fast. Uncompromising. It delivers a rich **TUI** (Terminal User Interface) with menus, tables, real-time logs, and keyboard shortcuts—all without terminal bloat or command memorization.

### ⚡ What You Control
* **AI Synthesis:** Connect **Google Gemini**, **OpenAI**, **Ollama** (run models locally, free), or any LLM provider. Full provider registry. Full model parity.
* **Governance Codex:** Define *Personas* (AI profiles), *Scopes* (execution domains), *Skills* (tool capabilities). Maestro enforces rules, validates boundaries, tracks compliance.
* **Secure Credentials:** OAuth2 browser login to Google Gemini. Credentials stored in OS keychain—never exposed to logs or configs.
* **Agent Observability:** Full tracing of agent decisions, token usage, cost tracking, and audit logs. Know what your AI team is doing, always.

### ⚡ Dependency Matrix
Maestro partitions the dependency graph into **two isolation zones**:

**Zone 1: Harness Domain** — Maestro runtime readiness. LLM provider config, model catalog, connection health.

**Zone 2: Project Domain** — Your repo's AI companion. Toolchain checks, command availability, framework validation (defined in `maestro/project-deps.yaml`).

Validate each zone independently:

```bash
maestro deps check --scope harness      # Check Maestro runtime only
maestro deps check --scope project      # Check project toolchain only
maestro deps check --scope all          # Full validation
```

---

## ⚡ BOOT SEQUENCE

**Open your terminal.** On macOS and Linux: search for "Terminal". On Windows: open "PowerShell" or "Command Prompt".

### 🪄 AUTO-DEPLOY (macOS & Linux)
Run this one-liner to synthesize and install:

**Copy and paste this command, then press `Enter`:**
```bash
curl -sSL https://raw.githubusercontent.com/invector-tecnologia/maestro-multi-agents/main/scripts/install.sh | bash
```
*Note: You may need to enter your system password to install. Characters won't show as you type—this is normal. Just type and press Enter.*

---

### 🔧 MANUAL OVERRIDE (Build from Source)
If auto-deploy fails, or you prefer direct control, follow your OS track:

#### 🍎 MACOS
Generate the native `.pkg` installer from source:
1. Open the terminal in the project folder.
2. Generate the package: `./scripts/build-macos-pkg.sh 0.1.0`
3. Install by double-clicking the generated file or run in terminal:
```bash
sudo installer -pkg target/macos/build/maestro-ai-0.1.0-macos-$(uname -m).pkg -target /
```

#### 🐧 DEBIAN / UBUNTU / LINUX MINT
1. Navigate to project folder.
2. Build: `./scripts/build-deb.sh 0.1.0`
3. Deploy:
```bash
sudo dpkg -i target/deb/maestro-ai_0.1.0_amd64/maestro-ai.deb
```

#### 🎩 ARCH LINUX / OMARCHY
1. Navigate to project folder.
2. Build: `./scripts/build-omarchy-pkg.sh 0.1.0`
3. Deploy:
```bash
sudo pacman -U --noconfirm target/omarchy/build/maestro-ai-0.1.0-1-$(uname -m).pkg.tar.zst
```

> **⚡ Validation Override:** Run the smoke test to verify installation integrity:
> `./scripts/smoke-test-omarchy.sh target/omarchy/build/maestro-ai-0.1.0-1-$(uname -m).pkg.tar.zst`

---

## ⚡ CONTROL DECK INITIALIZATION

All governance, TUI state, and project configurations live inside the `maestro/` folder in your project root. Maestro reads `./maestro/config.yaml` first; if not found, it scans the global system config path.

**This is your control deck schema.** Define providers, models, concurrency limits, rate limits, retry logic. Example: orchestrating Ollama locally:

```yaml
system:
  default_provider: "ollama"
  default_model: "mistral"
  max_concurrency: 4
  rate_limit_per_minute: 120
  retry_max_attempts: 3

providers:
  ollama:
    kind: "ollama"
    endpoint: "http://127.0.0.1:11434/v1"
    auth_mode: "none"
    timeout_ms: 60000
    models:
      - name: "mistral"
        context_window: 32000
    capabilities:
      supports_tools: false
      supports_streaming: true
      supports_json_mode: false
      supports_reasoning_controls: false
      max_context_tokens: 32000
```

**Auth Override:** For Bearer token authentication, adjust `auth_mode` to `"bearer"` and export the token as an environment variable before launching Maestro.

---

## ⚡ COMMAND EXECUTION

**Execution Protocol.** Boot your orchestration workflow:

1. **INIT** — `maestro init <project-name>` synthesizes project folder, default config, and mandatory directories. Opens interactive onboarding by default.
2. **SCRIPTED INIT** — `maestro init <project-name> --no-tui` for CI/CD and automation (no interactive prompts).
3. **VALIDATE** — `maestro validate-config` checks configuration integrity and dependency health.
4. **LAUNCH** — `maestro tui` fires up your interactive command deck.
5. **ARCHITECT** — Inside TUI, execute `/new scope`, `/new persona`, `/new skill` to map execution domains and AI profiles.
6. **EXECUTE** — `maestro run` triggers automated work cycles. Monitor logs. Watch your AI team synthesize.

### ⚡ Utility Commands

* **`maestro doctor`** — Health scan. Validates environment, mandatory markdowns, and config readiness.
* **`maestro init-config`** — Generates only `maestro/config.yaml`.
* **`maestro scaffold-markdown`** — Generates only Markdown folder structure.
* **`maestro deps check --scope <harness|project|all>`** — Validates dependency zones independently.
* **`maestro list-agents`** — Catalogs all registered personas.
* **`maestro onboarding --mode fast`** — Rapid onboarding with safe defaults.
* **`maestro onboarding --mode detailed`** — Full guided interview with advanced options.

### 🐞 DEBUG OVERRIDE

Enable full tracing and write debug logs:

```bash
MAESTRO_DEBUG=1 maestro tui
```

Logs stream to `maestro.log` in the current directory.

---

## ⚡ GOVERNANCE OVERRIDE

Every release passes through a **Quality Gate**. Validate locally:

```bash
./scripts/quality-gate.sh              # Run full quality validation
./scripts/publish-github.sh v0.1.0    # Build and publish release to GitHub
```

### ⚡ PR Governance Protocol

This repository enforces **CI-gated governance** through `.github/workflows/ai-governance-gate.yml`.

**Required PR Structure:**
1. `## Linked Plan Task` — exactly one line:
  - `- Path: docs/Maestro_Execution_Plans/tasks/<task>.md`
2. `## Acceptance Criteria` — IDs like `AC1`, `AC2`, `AC3`.
3. `## Validation Evidence` — one evidence line per AC ID.

**Acceptance Criteria Floor:** Configurable via repository variable `MIN_ACCEPTANCE_ITEMS`. Defaults to `3` if not set.

**Configure in GitHub:**
1. Repo Settings → `Secrets and variables` → `Actions` → `Variables`
2. Create `MIN_ACCEPTANCE_ITEMS` with numeric value (e.g., `4`)

**License:** MIT

---

## ⚡ REFERENCE GRID

The `docs/` folder is your knowledge base, organized by execution domain:

* **`docs/Maestro_Execution_Plans/`** — Roadmap: execution plans, release candidates, milestone specs.
* **`docs/Practical_Guides/`** — Tutorials: onboarding, smoke tests, adoption playbooks.
* **`docs/User_Manual/`** — Runtime reference: commands, panels, day-to-day operations.
* **`docs/Maestro_Manifesto/`** — Architecture truth: design philosophy, conventions, feature matrix, value streams.
