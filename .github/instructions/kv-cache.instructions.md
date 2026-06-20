---
applyTo: "src/**/*.rs"
description: "Use when implementing caching, prompt reuse, token reduction, response memoization, provider request deduplication, or KV cache related runtime behavior."
---

# KV Cache Policy

## Design Rules
1. Cache keys must be deterministic and include model/provider identity.
2. Cache entries must carry explicit freshness policy (TTL or invalidation signal).
3. Fail open on cache read errors unless user safety or correctness requires fail closed.
4. Never let cache bypass authorization or tenant boundaries.

## Safety Rules
- Avoid stale-cache hallucination by tying cache scope to prompt and context hash.
- Log cache hit, miss, and invalidation events with `tracing` for auditability.
- Provide a clear bypass path for debugging and incident response.

## Verification
- Add tests for hit/miss, invalidation, and stale-read prevention.
- Document measurable impact (latency, token reduction, cost proxy) when cache policy changes.
