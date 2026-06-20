---
applyTo: "**/*"
description: "Use when planning or implementing features with specification-first delivery, acceptance criteria, milestone tracking, or execution plans in docs/Maestro_Execution_Plans."
---

# Spec-Driven Delivery

## Required Sequence
1. Define or update the execution plan task document before major code changes.
2. Encode acceptance criteria that can be validated by tests or scripted checks.
3. Implement in small increments mapped to the spec.
4. Record validation evidence and residual risks.

## Review Rules
- Reject features without explicit acceptance criteria.
- Reject merges when implementation diverges from documented scope without rationale.
- Prefer small PRs linked to one plan task whenever possible.

## Documentation Targets
- Task specs: `docs/Maestro_Execution_Plans/tasks/`
- Product doctrine: `docs/Maestro_Manifesto/`
