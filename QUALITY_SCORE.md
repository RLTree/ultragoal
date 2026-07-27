# QUALITY_SCORE

Use this scorecard for agent-authored work. Scores come from current
observations on the named surface, not artifact volume or reviewer confidence.

| Category | Pass signal |
| --- | --- |
| Boundaries/types | Untrusted inputs parse before action; boundary checks pass. |
| Tests/coverage | Focused tests pass; any coverage claim has current measurement. |
| Code shape | Namespace, architecture, and size checks pass. |
| Security | Security hygiene passes or a blocker names the risk. |
| Latency/efficiency | Timing proof or a performance blocker is recorded when material. |
| Observability | Claim-relevant diagnostics or observations are named; unused channels are not manufactured. |
| Product cohesion | Journey, same-surface evidence, and attention exceptions are joined for product claims. |
| Docs/architecture fit | Agent docs route correctly and the active ExecPlan is current. |
| Evidence economy | Proof matches the claim and has finite invalidation/deletion rules. |
| Residual gaps | Gaps are explicit blockers or debt, never hidden. |

No category passes on “looks good” or probabilistic review alone.
