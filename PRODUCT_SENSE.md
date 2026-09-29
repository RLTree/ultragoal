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

## Invisible Advisory, Inspectable Decisions

An operator should receive the right design or decision help without knowing
that Agentic Engineering exists. The default Harness response therefore names
the intended outcome, current result, material tradeoff, and next action in
plain language. It does not present a catalog of disciplines or ask the user to
choose an internal lifecycle.

Small, already-specified, and already-correct tasks should feel smaller, not
more ceremonial. The product selects no change or one smallest sufficient
advisory lens when that is enough. A supporting lens appears only when a real
cross-layer dependency requires it.

Operators who want more control can inspect the activated disciplines and
rationale. Advanced users can inspect exact skill paths, evidence, alternatives,
root adoption decisions, invalidation conditions, and claim ceilings or
explicitly request a lens. Explicit preference never hides a material decision
or effect and cannot bypass protected authority.

Reactivation should be visible only when it changes the useful result: a new
candidate, lifecycle boundary, evidence item, assumption, risk, or failure
mechanism. Unchanged evidence must not create another advisory turn.
