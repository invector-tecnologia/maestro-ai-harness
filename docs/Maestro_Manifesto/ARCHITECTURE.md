# MAESTRO AI HARNESS: Architecture Guidelines

## 1. Overview and Paradigm
Maestro AI Harness is a complete multi-agent orchestration ecosystem for software engineering. The architecture is based on:
- **Actor Model (Event-Driven):** Agents are autonomous entities running in async tasks (`tokio::spawn`) and communicating only through asynchronous message exchange.
- **Hexagonal Architecture (Ports and Adapters):** Strict isolation between agent decision logic (Domain) and external AI/system APIs (Infrastructure).
- **AI Harness (Control and Evaluation):** A safe sandbox where AIs run with scoped context, token monitoring, and continuous validation (Quality Gates) before executing tasks.

## 2. Directory Topology (Strict DDD)
All generated code must respect the following segregation in `src/`:

```text
src/
├── domain/         # Core. Zero I/O dependencies, external APIs, or heavy frameworks.
│   ├── models/     # Entities and Value Objects (for example: Message, Role, Memory).
│   └── ports/      # Traits (interfaces) implemented by infrastructure (for example: LlmProvider).
├── application/    # Use cases and orchestration; environment and agent lifecycle live here.
│   └── sops/       # Standard Operating Procedures for agents.
├── infrastructure/ # Port implementations and external integrations.
│   ├── llm/        # Adapters for Ollama, Gemini, and future providers.
│   ├── bus/        # Event bus implementation (for example: tokio::sync::broadcast).
│   └── harness/    # Sandbox, token limits, and AI action safety auditing.
└── presentation/   # Entry points and UX surfaces.
    └── cli/        # CLI argument parsing (clap) and startup wiring.
```
