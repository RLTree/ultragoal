---
name: fit-repo
description: Deprecated compatibility alias for explicit `$harness-ultragoal:fit-repo` requests. Preserve the request and route it to `$harness-ultragoal:repository-fit`; do not use this alias as independent workflow authority.
---

# Deprecated Compatibility Route

> Compatibility warning: this legacy alias is not an independent workflow or authority. Its canonical target is `$harness-ultragoal:repository-fit`.

1. Preserve the user's full request, context, constraints, and authorized effects unchanged.
2. Before routing, inspect the request and supplied context for Harness Ultragoal skill tokens. If it contains another distinct explicit Harness Ultragoal skill token or selects multiple compatibility routes, report a causal compatibility-route conflict and perform no routing or effect.
3. Invoke `$harness-ultragoal:repository-fit` with that preserved input.
4. Follow only the canonical target's current contract. Do not restore or apply legacy lane, gate, receipt, finalizer, command, tool, helper, schema, state-store, or generated authority from this wrapper.
5. Perform no hidden writes or external effects while resolving the route. Any later effect must remain authorized by the original request and the canonical target.
6. Fail closed if the canonical target is unavailable: report the exact blocker and do not fall back, infer semantic equivalence, or claim adoption, discovery, runtime behavior, retirement, readiness, release, or completion.
7. Never substitute documentation, tests, receipts, generated rows, telemetry, signatures, or provenance for the requested product behavior.
