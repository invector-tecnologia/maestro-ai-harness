---
name: rag-operations
description: "Use when you need end-to-end RAG operations in Maestro: corpus ingestion, chunking policy, retrieval tuning, reranking checks, eval dataset execution, and report persistence."
---

# RAG Operations Skill

## Use When
- You need to ingest or refresh the local corpus.
- You need to tune retrieval behavior or reranking.
- You need to run baseline vs enhanced evaluation and compare outcomes.

## Workflow
1. Confirm index location and corpus scope.
2. Ingest documents with explicit chunking parameters.
3. Run query smoke tests for citation quality.
4. Run evaluation dataset and persist the report.
5. Summarize regressions and recommended next action.

## Outputs
- Commands executed
- Key relevance observations
- Report path and pass-rate deltas
