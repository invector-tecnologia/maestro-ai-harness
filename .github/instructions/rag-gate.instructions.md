---
applyTo: "src/**/*.rs"
description: "Use when implementing or reviewing RAG ingestion, retrieval, reranking, evaluation, dataset versioning, embeddings, citations, or grounding quality in Maestro."
---

# RAG Governance Gate

## Required Flow
1. Preserve separation between domain ports, application orchestration, and infrastructure adapters.
2. Keep lexical fallback available when embeddings are absent.
3. Ensure query outputs include traceable citations or provenance.
4. Keep evaluation datasets versioned under docs and reports persisted for later comparison.

## Change Rules
- Any RAG logic update must include at least one test covering regression risk.
- If scoring or ranking logic changes, compare baseline vs enhanced behavior.
- Reject hidden magic constants; document thresholds in code or config.

## Evidence Expectations
- Include command evidence for local validation (for example: `cargo test`).
- When possible, include a short before/after metric delta for relevance or hit-rate.
