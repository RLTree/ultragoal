# QUALITY_SCORE

Use this deterministic scorecard for agent-authored work. Final reports may
summarize the categories, but the score comes from commands and receipts, not
vibes.

| Category | Pass signal |
| --- | --- |
| Boundaries/types | Untrusted inputs parse before action; boundary checks pass. |
| Tests/coverage | The repo's test and coverage gates pass or name blockers. |
| Code shape | Namespace, architecture, and size checks pass. |
| Security | Security hygiene passes or a blocker names the risk. |
| Latency/efficiency | Timing proof or performance blocker is recorded. |
| Observability | Claim-relevant diagnostics, logs, metrics, traces, evals, or screenshots are named; unused channels are not manufactured. |
| Product cohesion | User journey, UI/runtime evidence, and human-attention exceptions are joined. |
| Docs/architecture fit | Agent docs route correctly and active ExecPlans are current. |
| Receipts | The smallest current claim-consuming receipt names exact commands, outcomes, artifacts, and digests; routine diagnostics remain ephemeral. |
| Residual gaps | Gaps are listed as blockers or tech debt, never hidden. |

No category can pass on "looks good" or probabilistic review alone.
