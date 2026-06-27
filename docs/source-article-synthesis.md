# Source Article Synthesis

This document records how the cited foundation articles shape the plugin proposal.

## OpenAI Harness Engineering

Source: https://openai.com/index/harness-engineering/

Key takeaways:

- Humans steer; agents execute.
- The engineering job shifts toward environment design, intent specification, and feedback loops.
- Repository knowledge should be the system of record.
- A short `AGENTS.md` should route agents to deeper docs, not become a giant stale manual.
- Application UI, logs, metrics, traces, and worktree-local runtimes must become legible to agents.
- Invariants should be enforced mechanically through lints and structural tests.
- Agent autonomy introduces entropy, so recurring cleanup must encode golden principles into the repo.

Plugin implications:

- Ship `harness-engineering` as a repo setup skill.
- Split harness setup into fresh-repo initialization and existing-repo retrofit, because otherwise the user still has to know the target repo shape before the plugin can help.
- Keep `AGENTS.md` as a compact pointer, not the whole law.
- Include mechanical validators, not only documentation.
- Include standards-gardening as a first-class skill.
- Treat runtime proof surfaces as mandatory where claims are user-facing.

## OpenAI Symphony

Source: https://openai.com/index/open-source-codex-orchestration-symphony/

Key takeaways:

- The core orchestration idea can be expressed as a simple spec.
- The practical rule is: for every open task, guarantee an agent is running in its own workspace.
- Workflow steps that humans previously knew implicitly should be captured in a versioned workflow contract.
- Implementation in multiple languages was used to find ambiguity and simplify.
- Humanized event summaries must be observability only; orchestrator logic should not depend on prose strings.

Plugin implications:

- Represent lanes as durable records in a lane registry.
- Require workspace isolation for every lane.
- Make the workflow contract explicit and versioned.
- Include review agents that attack ambiguity.
- Treat human-readable summaries as evidence indexes only, not proof.

## Codex ExecPlans

Source: https://developers.openai.com/cookbook/articles/codex_exec_plans

Key takeaways:

- An ExecPlan must be self-contained.
- A future agent should be able to restart from the plan alone.
- The plan is a living document with progress, decisions, discoveries, outcomes, concrete steps, validation, and recovery.
- A plan must produce demonstrably working behavior, not only code changes.
- It is acceptable to include prototypes that de-risk a large implementation, but they must have promotion/discard criteria.

Plugin implications:

- Macro-lanes must receive full ExecPlan contracts.
- Launch prompts should orient to the ExecPlan, not replace it.
- Completion requires observable behavior and receipts.
- Every lane plan must maintain progress, discoveries, decision log, outcomes, validation, idempotence, and recovery.

## Parse, Don't Validate

Source: https://lexi-lambda.github.io/blog/2019/11/05/parse-don-t-validate/

Key takeaways:

- Validation throws away knowledge; parsing preserves the knowledge in the type.
- Parse at the boundary before acting on data.
- Typed representations reduce repeated checks and make illegal states less constructible.

Plugin implications:

- Use schemas for manifests, lane registry, ready receipts, and validator receipts.
- Represent claim status, evidence surface, blocker class, and lane state as typed enums.
- Do not let freeform prose become the authority for readiness.
- Treat model outputs and connector data as untrusted until parsed into contract objects.

## AI Is Forcing Us To Write Good Code

Source: https://bits.logic.inc/p/ai-is-forcing-us-to-write-good-code

Key takeaways:

- Strong tests, clear docs, scoped modules, static types, and reproducible dev environments become essential for agents.
- 100 percent coverage can act as an ambiguity-removal mechanism: uncovered lines become a concrete todo list.
- The filesystem is an interface for agents.
- Small, well-scoped files reduce context degradation.
- Fast, ephemeral, concurrent environments make multi-agent work natural.
- Isolation must cover ports, databases, caches, jobs, and any other shared resource.

Plugin implications:

- Include standards for namespacing, file-size budgets, and mechanical checks.
- Require fast feedback loops.
- Require per-lane isolation beyond just git branches.
- Include worktree lifecycle and teardown obligations.
- Require install/setup/runtime commands to be discoverable and runnable from a clean checkout, not inherited from the original author's memory.

## OpenAI Codex Customization, Skills, Plugins, And Subagents

Sources:

- https://developers.openai.com/codex/concepts/customization
- https://developers.openai.com/codex/skills
- https://developers.openai.com/codex/plugins/build
- https://developers.openai.com/codex/subagents

Key takeaways:

- Codex customization layers are complementary: `AGENTS.md`, memories, skills, MCP, and subagents.
- Skills are the authoring format; plugins are the installable distribution unit.
- Personal skills live under `$HOME/.agents/skills`; repo skills live under `.agents/skills`.
- A local plugin has `.codex-plugin/plugin.json` and can be exposed through a personal or repo marketplace.
- Custom agents are standalone TOML files under `~/.codex/agents/` or `.codex/agents/`.

Plugin implications:

- Ship `.codex-plugin/plugin.json` in the package, not only a proposal manifest.
- Include a personal marketplace example for local installation.
- Install reviewer personas as Codex custom-agent TOML files, not only markdown prompts.
- Keep plugin correctness independent of MCP or connector availability unless a claim explicitly depends on that connector.

## Synthesis

The plugin should be opinionated where false completion is likely, and flexible where local implementation details vary. It should enforce:

- durable contracts;
- typed evidence;
- restartability;
- isolation;
- mechanical validation;
- live beneficial proof;
- stale-state cleanup.

It should not enforce:

- one specific task tracker;
- one UI tool;
- one exact repo command set;
- one exact language stack;
- the `codex-workflow-rs` lane numbering.

## Source Cards

Source retrieval/provenance records live in `docs/source-cards.json`. This synthesis explains usage; it is not itself the provenance ledger.
Cards marked `not_refreshed` are allowed only as historical foundation context.
They lower the claim ceiling: they cannot support current, live, endorsed, or
source-backed implementation claims until refreshed or explicitly blocked with
a receipt.

## Observability Stack

The OpenAI harness-engineering article includes observability as an agent context surface, not merely operator monitoring. This package carries that forward with a portable baseline: isolated worktree runtime identity, append-only run events, queryable logs, basic metrics, trace/span receipts where the target supports them, and generated agent-context summaries. Stronger stacks such as LogQL, PromQL, TraceQL, OpenTelemetry, or Grafana are adapters, not mandatory scaffolding for static or non-runnable repos.

## Product Cohesion Gate

The foundation articles explain how to make agent work durable, isolated,
observable, and mechanically auditable. The Downbeat product surface showed the
remaining gap: a powerful engine can still feel fragmented if user journeys,
product promise, human-attention policy, proof surfaces, and claim ceilings do
not compose into one coherent experience.

The Product Design plugin shaped the gate around flow audit, screenshots,
accessibility, state coverage, and evidence tied to actual UI journeys. The
Creative Production plugin shaped the positioning side: audience, occasion,
offer/value promise, business outcome, and proof. Compound Engineering shaped
the strategy/pulse view: target user, product track, success signal, and
whether the product makes sense from the consumer's point of view. Superpowers
kept the addition in a compact, testable-plan frame instead of broad design
ceremony.

Plugin implications:

- Consumer-facing claims require product-cohesion receipts, not only engine
  receipts.
- Included `feature_completion` claims require explicit product-applicability
  classification. Consumer-facing, local-app, dashboard, workflow-launcher,
  run-console, and control-surface outcomes cannot be downgraded into
  runtime-only claims by changing structured surfaces while leaving the product
  promise in the title or description.
- Joined lowercase product text such as `webapp`, `runconsole`, and
  `controlsurface` still owes Product Cohesion proof. Generic proof/runtime
  surface wording does not trigger the gate by itself when it is explicitly
  classified as non-product.
- UI proof must connect intent, route decision, execution truth, receipts, and
  claim ceiling.
- Human handoff surfaces such as "Needs you" should be exceptional. If most work
  requires the user, the product is exposing harness failure instead of carrying
  the work autonomously.
- Human attention claims must cite evidence that harness-owned routes, retries,
  receipt lookup, blocker classification, or other autonomous recovery paths
  were exhausted before asking the user.
- Product cohesion remains conditional; backend-only work should not inherit a
  design-review tax unless the goal or claim depends on a product surface.
