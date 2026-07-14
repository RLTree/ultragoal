# Security, Reliability, And Product Cohesion

## Security And Privacy Law

Security rules apply to agent work even when the code path is "only local" or
"only a tool."

- Authorize before side effects, especially external, destructive, credential,
  permission, financial, or production-impacting actions.
- Do not commit raw prompts, transcripts, message bodies, secrets,
  credentials, or personal data into fixtures, logs, ledgers, or examples.
- Do not leak private local paths, raw traces, screenshots, videos, model
  prompts, session logs, API keys, tokens, cookies, Authorization headers,
  database URLs, or unbounded query output into package-visible artifacts.
- Validators, reviewers, and review prompts do not open, persist, or quote raw
  private transcript, audio, prompt, message, or event payloads as routine
  proof. They may use category-only status, counts, digests, redacted snippets,
  and leak summaries when those are enough to validate the claim.
- Dependency additions require a receipt: alternatives considered, license,
  attack surface, maintenance cost, runtime cost, build cost, and why an
  in-repo implementation is not better.
- Sensitive changes need negative tests or a named blocker explaining why a
  negative test is not feasible yet.

## Reliability Law

Reliability is part of correctness.

- Retry, idempotency, cancellation, timeout, concurrency, and recovery behavior
  need explicit tests or proof when touched.
- Feedback latency is a product and agent-throughput requirement. Measure
  before changing caches, parallelism, cold-start behavior, or gate scope.
- Do not lower correctness, coverage, or security to win timing.
- Preserve the artifacts needed to diagnose failures after the run ends.
- If downstream claims trust final words, missing or disabled post-stop batch
  transcription, cleanup, or final alignment is a blocker. It must emit a
  recovery action; it cannot be downgraded to an informational note.

## Product Cohesion Law

For user-facing products, dashboards, workflow launchers, run consoles,
settings surfaces, local apps, and agentic control surfaces, code correctness
is not enough.

- The product must make sense from the user's point of view.
- Powerful engines need coherent presentation, natural workflow, clear status,
  meaningful defaults, understandable exceptions, and recoverable errors.
- "Needs You" or equivalent human-attention states are last resorts. A harness
  should resolve routine ambiguity through tools, receipts, probes, and bounded
  decisions before escalating to the human.
- Route success, command success, and schema success do not prove the user
  journey. Product claims need journey proof, UI evidence, and inspection
  against the intended job to be done.
- Recording, replay, transcript, video-alignment, and live UI claims are
  product proof surfaces. CLI availability and fixture success may support
  setup confidence, but they do not prove those product surfaces.
- Product Cohesion proves journey coherence. It does not prove product success
  by itself. Product success claims need Product Success Contract lineage,
  Product Fitness, required quality-in-use evidence, and same-surface proof.

## Plugin Cohesion Law

A plugin is a product surface for agents. Skills, agents, templates, scripts,
receipts, and validators must compose into an obvious workflow.

- Every skill states when to use it, what it produces, and which skill or gate
  usually follows.
- Cross-skill routing belongs in a small resource map, not in a giant skill.
- A user should not have to remember the plugin's internal order for common
  flows such as init, retrofit, lane launch, review, proof, install, or
  standards gardening.
- If repeated friction shows agents choose the wrong skill, miss a required
  resource, or ask the user to orchestrate obvious plugin steps, improve the
  skill prompt, resource map, template, or validator.
