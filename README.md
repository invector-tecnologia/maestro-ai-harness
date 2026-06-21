# 🎼 Maestro Harness for AI

Welcome to **Maestro Harness for AI**! If you have never used a terminal tool before, do not worry: this manual was made to hold your hand and guide you step-by-step. 

The terminal might seem daunting, but with Maestro, it will transform into a friendly and powerful control panel!

<p align="center">
  <img src="https://raw.githubusercontent.com/invector-tecnologia/maestro-ai-harness/main/docs/assets/dream-tui.png" alt="Maestro Dream TUI" width="800">
</p>

---

## 🌟 Welcome and About Maestro

**Maestro Harness for AI** acts exactly as a relentless *Tech Lead*. Instead of coordinating human developers, **it coordinates, tests, and manages a team of Artificial Intelligences (AIs)** to plan and build software for you.

Built with **Rust** (making it incredibly fast and safe), it offers a "TUI" (Terminal User Interface). This means you will have menus, tables, logs, and visual shortcuts directly in your terminal, without needing to memorize dozens of commands.

### What does it do best?
* **AI Automation:** Works with providers like **Google Gemini** and **Ollama** (to run free models locally).
* **AI Harness & Governance:** Automatically organizes the rules of your project, defining *Personas* (AI profiles), *Scopes* (what needs to be done), and *Skills* (AI tools), while ensuring they run safely in a controlled environment.
* **Secure Login:** Allows you to log into your Google (Gemini) account directly via your web browser, saving credentials securely in your OS keychain.

### Dependency Domains (Important)
Maestro now separates dependencies into two domains:

1. Harness dependencies (Maestro runtime): provider config, default model availability, and core runtime readiness.
2. Project dependencies (AI companion for your repo): toolchain and command checks defined in `maestro/project-deps.yaml`.

Use dependency checks explicitly:

```bash
maestro deps check --scope harness
maestro deps check --scope project
maestro deps check --scope all
```

---

## 🚀 Installation Steps (Bringing Maestro to your machine)

To install Maestro, you need to open your **Terminal** application (on macOS and Linux, search for "Terminal"; on Windows, search for "Command Prompt" or "PowerShell").

### 🪄 Automatic Installation (Recommended for macOS and Linux)
If you use macOS or any Linux distribution, we have created a magic script that does all the hard work for you.

In your terminal, type (or copy and paste) the command below and press `Enter`:
```bash
curl -sSL https://raw.githubusercontent.com/invector-tecnologia/maestro-multi-agents/main/scripts/install.sh | bash
```
*Note: It might ask for your computer's password to place Maestro in the correct folder. When typing your password in the terminal, characters will not appear (this is normal system security!), just type and press Enter.*

---

### 🔧 Manual Installation by OS
If automatic installation fails, or if you prefer to install manually, follow your system's steps:

#### 🍎 macOS
Download or build the native Mac installer (`.pkg`). If you downloaded the source code, you can generate the package like this:
1. Open the terminal in the project folder.
2. Generate the package: `./scripts/build-macos-pkg.sh 0.1.0`
3. Install by double-clicking the generated file or run in terminal:
```bash
sudo installer -pkg target/macos/build/maestro-ai-0.1.0-macos-$(uname -m).pkg -target /
```

#### 🐧 Ubuntu / Debian / Linux Mint
1. Open the terminal in the project folder.
2. Generate the package: `./scripts/build-deb.sh 0.1.0`
3. Install the generated package:
```bash
sudo dpkg -i target/deb/maestro-ai_0.1.0_amd64/maestro-ai.deb
```

#### 🎩 Arch Linux / Omarchy
1. Open the terminal in the project folder.
2. Generate the package: `./scripts/build-omarchy-pkg.sh 0.1.0`
3. Install with pacman:
```bash
sudo pacman -U --noconfirm target/omarchy/build/maestro-ai-0.1.0-1-$(uname -m).pkg.tar.zst
```

> **Golden Tip:** also run the smoke test to validate your installation:
> `./scripts/smoke-test-omarchy.sh target/omarchy/build/maestro-ai-0.1.0-1-$(uname -m).pkg.tar.zst`

---

## ⚙️ Setting Up the Office (Configuration)

All governance, TUI state, and project configurations are now isolated locally inside a `maestro/` folder in your project directory. Maestro looks first for `./maestro/config.yaml` and, if not found, falls back to the global system config path.

In this file, you define the rules of the game. A minimal configuration example to use local models via Ollama:

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

(Note: If you need Bearer token authentication, just adjust the `auth_mode` parameters and export the corresponding environment variable before starting Maestro).

---

## 🎮 Mission Control (Usage Guide)

The recommended workflow to start your workday and orchestrate activities is the following:

1. **Initialize the project:** Run `maestro init <project-name>` to create the project folder, generate default config (`maestro/config.yaml`), and initial mandatory folders. The command opens the onboarding interview mode by default.
2. **Optional automation mode:** Use `maestro init <project-name> --no-tui` when you need non-interactive setup for scripts or CI.
3. **Validate the terrain:** Run `maestro validate-config` to ensure your settings and file dependencies are correct.
4. **Open the panel:** Type `maestro tui` to access your interactive dashboard.
5. **Plan and delegate:** Inside the TUI, use the commands `/new scope`, `/new persona`, and `/new skill` to map your architecture requirements.
6. **Get to work:** Run `maestro run` to execute the automated work cycles of your agent team and track the logs in the Harness.

### Other Useful Terminal Commands (CLI)

* **`maestro doctor`:** Performs a quick check-up to see if your environment structure and mandatory markdowns are healthy.
* **`maestro init-config`:** Generates only the default config file (`maestro/config.yaml`) in the current directory.
* **`maestro scaffold-markdown`:** Generates only the initial Markdown folders and files in the current directory.
* **`maestro deps check --scope <harness|project|all>`:** Validates dependency domains independently.
* **`maestro list-agents`:** Displays the list of all personas registered in your current catalog.
* **`maestro onboarding --mode fast`:** Starts onboarding with safe defaults and the shortest path to a working setup.
* **`maestro onboarding --mode detailed`:** Starts the guided interview with more controls and setup questions.

### 🐞 Debug Mode

To start with detailed logs (DEBUG level), use:

```bash
MAESTRO_DEBUG=1 maestro tui
```

Logs are written to the `maestro.log` file in the current directory.

---

## 🛡️ Quality and License

Before any release, quality is validated. There is a single Quality Gate that can be run via `./scripts/quality-gate.sh`. There is also a local script to package and publish releases on GitHub: `./scripts/publish-github.sh v0.1.0`.

### Pull Request Governance Gate

This repository enforces PR governance through `.github/workflows/ai-governance-gate.yml`.

Required PR body structure:
1. `## Linked Plan Task` with exactly one line in this format:
  - `- Path: docs/Maestro_Execution_Plans/tasks/<task>.md`
2. `## Acceptance Criteria` using IDs like `AC1`, `AC2`, `AC3`.
3. `## Validation Evidence` including one evidence line per AC ID.

The minimum number of acceptance criteria is configurable through the repository variable `MIN_ACCEPTANCE_ITEMS`.

If `MIN_ACCEPTANCE_ITEMS` is not set, CI uses the default value `3`.

How to set it in GitHub:
1. Open repository settings.
2. Go to `Secrets and variables` > `Actions` > `Variables`.
3. Create `MIN_ACCEPTANCE_ITEMS` with a numeric value (for example: `4`).

The project is distributed under the MIT License.

---

## 📚 Documentation Information Architecture

The `docs/` folder is organized by value stream and usage context:

* **`docs/Maestro_Execution_Plans/`:** Product and development execution plans, release candidates, and milestone tasks.
* **`docs/Practical_Guides/`:** Hands-on guides for onboarding, installation checks, and practical adoption.
* **`docs/User_Manual/`:** Command and panel reference for day-to-day operation.
* **`docs/Maestro_Manifesto/`:** Core architecture, conventions, philosophy, feature levels, and value streams.
