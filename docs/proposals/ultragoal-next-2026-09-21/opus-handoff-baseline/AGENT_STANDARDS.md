# Agent Standards Router

Load only the module relevant to the current task. If several appear relevant,
start with the one owning the decision or failure boundary and expand only when
the source proves a dependency.

| Task | Module |
| --- | --- |
| Names, paths, context, documentation routing | `agent-standards/01-namespace-and-progressive-disclosure.md` |
| Tests, parsers, validators, feedback loops | `agent-standards/02-boundaries-validation-and-enforcement.md` |
| ExecPlans, ownership, worktrees, orchestration | `agent-standards/03-execplans-worktrees-and-orchestration.md` |
| Security, reliability, product cohesion | `agent-standards/04-security-reliability-and-product-cohesion.md` |
| Reviews, proof, claim ceilings, completion | `agent-standards/05-review-and-completion.md` |
| Repeated friction and standards gardening | `agent-standards/06-standards-gardening.md` |
| CLI authority and proof-surface separation | `agent-standards/07-cli-authority-and-proof-surfaces.md` |
| Observability, diagnosis, repair, evaluation | `agent-standards/08-observability-and-repair-loop.md` |
| Product success, fitness, cohesion, quality | `agent-standards/09-product-success-and-quality-in-use.md` |
| Plugin activation, package, installation, discovery | `agent-standards/10-plugin-activation-and-distribution-surfaces.md` |
| Research sources and decision updates | `agent-standards/11-research-improvement-and-quality-gates.md` |
| Tool risk, runtime, dependencies, privacy | `agent-standards/12-tool-risk-and-runtime-substrates.md` |

The nearest repository instruction wins within its authority. Deterministic
behavior belongs in code, tests, schemas, or tools; prose owns only semantics
that cannot be inferred mechanically.
