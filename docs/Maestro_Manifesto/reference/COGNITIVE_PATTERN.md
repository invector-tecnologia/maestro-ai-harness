# Canonical Cognitive Pattern

Maestro models **every** agent — personas, onboarding, retrieval, and the
orchestrator itself — on one shared cognitive cycle. This document is the
canonical reference for that pattern and where it is implemented.

## The Cycle

```text
SENSE → OBSERVE → THINK → ACT → AUDIT → DELIVER
```

| Stage    | Intent                                                              |
| -------- | ------------------------------------------------------------------ |
| SENSE    | Probe the environment for readiness before committing to work.      |
| OBSERVE  | Register an incoming message or demand as the unit of work.         |
| THINK    | Reason over the observation without side effects.                   |
| ACT      | Produce an output or perform the delegated work.                    |
| AUDIT    | Validate the contribution against acceptance criteria.              |
| DELIVER  | Synthesize and publish the approved result.                         |

The innermost loop is the `Role` trait's `observe → think → act`. `SENSE`,
`AUDIT`, and `DELIVER` are orchestration-level stages that wrap that loop when
agents collaborate.

## Where It Lives

### Per-agent loop (`OBSERVE → THINK → ACT`)

- The `Role` trait (`src/domain/ports/`) defines
  `observe(&[Message]) → think() → act() -> Option<Message>`.
- `RuntimeEvent::{AgentObserving, AgentThinking, AgentActing, AgentActed}`
  (`src/application/agent_observability.rs`) narrate each phase for observability.

### SENSE stage

- `LlmProvider::probe` (`src/domain/ports/llm_provider.rs`) returns
  `ProviderStatus { Available, Unreachable, Unauthorized, ModelMissing }`.
- `readiness::probe_default_provider` / `run_checks_with_probe`
  (`src/application/readiness.rs`) lift the probe into the readiness signal
  `model_loaded`.
- Onboarding consumes the probe to choose an interview engine:
  `InterviewEngine::from_provider_status` (`src/application/interview_bot.rs`)
  selects the LLM-driven interview (Option B) when a model is serving and the
  deterministic guided setup (Option A) otherwise, auto-promoting once a model
  becomes available.

### Orchestration stages (`SENSE → … → AUDIT → DELIVER`)

- `AgentRuntime::orchestrate_as_maestro` (`src/application/agent_runtime.rs`)
  narrates the full cycle as `MaestroNarration` phases:
  `sense → plan → delegate → audit → deliver`. The `sense` phase observes the
  incoming demand before any planning begins.

### Retrieval

- `RagCognitiveAgent` (`src/application/rag_cognitive.rs`) wraps `RagService`
  and narrates a query as `AgentObserving → AgentThinking → AgentActing →
  AgentActed`, delegating retrieval unchanged to the service.

## Rules

1. New collaborating agents must narrate their lifecycle through `RuntimeEvent`
   so the TUI and observers can render the cycle uniformly.
2. SENSE must never block the loop indefinitely; probes use provider timeouts and
   degrade to a safe engine (guided setup) on failure.
3. THINK must be side-effect free; all writes happen in ACT or DELIVER, through
   governance where files are involved.
4. AUDIT is mandatory for delegated work: a failing contribution is isolated and
   recorded, never silently dropped.
