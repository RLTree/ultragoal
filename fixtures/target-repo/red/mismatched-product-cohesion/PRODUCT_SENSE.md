# PRODUCT_SENSE

Agents convert vague product asks into explicit operating facts before
building. Code can be correct while the product is confusing; this file keeps
the user's job visible.

For every non-trivial product change, state:

- User: who benefits or is protected.
- Job: what they can do after the change.
- Constraints: security, latency, correctness, local-only behavior,
  compatibility, and claim ceiling.
- Non-goals: what is intentionally out of scope.
- Risks: what could regress, leak, confuse, or over-escalate to a human.
- Acceptance: exact command, browser path, fixture, receipt, screenshot, trace,
  or runtime artifact that proves the job.

## Product Cohesion Gate

For user-facing products, dashboards, workflow launchers, run consoles,
settings surfaces, local apps, or agentic control surfaces, add Product
Cohesion evidence. The journey must show how the surfaces fit together and
where the harness resolves routine ambiguity without dumping work into a
"needs human" bucket.

If the work touches model/tool behavior, add deterministic eval cases first or
record why this run is policy-only.
