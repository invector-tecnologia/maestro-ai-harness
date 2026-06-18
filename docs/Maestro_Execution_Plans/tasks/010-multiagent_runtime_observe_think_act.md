# TASK 010: Multi-Agent Runtime with Observe-Think-Act Cycle

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** #file:src/application/environment.rs, #file:src/domain/ports/role.rs, #file:src/domain/ports/llm_provider.rs
* **Context Anchors:** #file:docs/Maestro_Manifesto/ARCHITECTURE.md, #file:docs/Maestro_Manifesto/CONVENTIONS.md
* **Expected Output:** Concurrent orchestration across multiple agents.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* No synchronous blocking.
* Failure isolation per agent.
* Explicit session lifecycle.

## 3. EXECUTION PROMPT (Paste into Copilot Chat)
"""
Act as a Principal Distributed Systems Engineer.
Goal: Run agents in parallel with lifecycle governance.

Before generating code, open a `<reasoning>` block and model concurrency, cancellation, and failure isolation.

Execute:
1. Implement an agent spawner and supervisor.
2. Implement the Observe-Think-Act cycle per agent task.
3. Integrate the broadcast bus for message distribution and consumption.
4. Implement start, stop, and health controls.
5. Test integration with multiple agents running simultaneously.

[Cohesion Mechanism]:
- Confirm that a single agent failure does not bring down the entire session.

Return ONLY the modified code blocks in Markdown. No introduction.
"""
