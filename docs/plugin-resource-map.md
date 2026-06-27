# Harness Ultragoal Plugin Resource Map

This map is the product cohesion layer for plugin usage. It tells agents which
plugin surface to use and why, so users do not have to manually orchestrate the
skills.

## Core Flow

1. Use `harness-ultragoal:fit-repo` first. It classifies the repo, chooses init
   or retrofit follow-on paths, installs or verifies required setup surfaces,
   emits the fit-repo receipt, and records the claim ceiling.
2. Use `ultragoal` after fit-repo when the work needs a binding goal contract, lane registry,
   verification backlog, completion manifest, amendments, and red fixtures.
3. Use `harness-engineering` as the umbrella for agent-first repo setup.
4. Use `agent-first-repo-init` for a new repo and `agent-first-repo-retrofit`
   for an existing repo.
5. Use `execplan-lane` to create each macro-lane contract.
6. Use `orchestrator-reconciler` in the parent session to launch, monitor,
   merge, advance dependencies, and tear down lane worktrees.
7. Use `proof-gate` before accepting lane completion, final completion, or any
   ready/install/publish claim.
8. Use `standards-gardener` when friction, repeated review findings, stale
   docs, stale worktrees, or agent-legibility entropy appear.

## Conditional Gates

- Use `agent-runtime-legibility` when the target repo has an app, CLI, service,
  workflow engine, or runtime surface that agents must run or prove.
- Use `agent-observability-stack` only when the user requests observability,
  the goal contract requires it, or a runtime claim cites logs, metrics,
  traces, spans, or agent context summaries.
- Use `product-cohesion-gate` when the work changes how a user starts,
  understands, monitors, trusts, or completes work through a product surface.

## Supporting Agents

- Use `agents/plugin-scout.md` when plugin or connector capability is uncertain,
  when installing a plugin could change the implementation path, or when a
  claim depends on app/plugin availability. Evidence lives in
  `docs/plugin-scout-receipt.json` and `artifacts/plugin-scout/`.
- Use `agents/standards-extractor.md` when harvesting reusable operating laws
  from a successful repo, decomposing mega-doc guidance, or updating templates
  after repeated friction. Evidence routes through `docs/repo-patterns-extracted.md`
  and `templates/agent-standards/`.
- Use `custom-agents/harness-repo-initializer.toml` only for fresh repo setup
  lanes; use `custom-agents/harness-retrofit-planner.toml` only for existing
  repo retrofit lanes.

## Setup And Proof Roots

- Codex app worktree setup is documented in
  `docs/codex-worktree-environment.md`; generated state belongs in
  `.codex-worktree/` and agents source `.codex-worktree/env.sh` before
  validation.
- Standards enforcement is a first-class setup surface. Fresh and retrofitted
  repos install `agent-standards/enforcement.json`,
  `agent-standards/enforcement.tsv`, `agent-standards/enforcement-audit.tsv`,
  and `scripts/check-agent-standards`; the validator check
  `agent-standards-enforcement` fails missing, unclassified, stale, or
  overclaimed standards rows.
- Coverage proof is a first-class setup surface. Generated repos receive
  `templates/COVERAGE_RECEIPT.json` and `templates/scripts/check`; the check
  runs standards enforcement first and blocks when no real coverage command is
  configured.
- Optional observability setup is documented in `docs/observability-stack.md`;
  use it only when the goal or claim surface requires observable runtime proof.
- Current review authority is typed, not narrative:
  `fixtures/review-round/valid/review-round-receipt.json` plus anchors in
  `fixtures/review-round/anchors/`.
- Active custom-agent registry proof is separate from disk sync and is
  documented in `docs/codex-custom-agent-registry-preflight.md`.
- `docs/review-loop-record.md` is a compact index. Raw historical Markdown is
  archive context, not approval or claim-ceiling authority.

## Review Team

Material review uses the four merged canonical personas:

1. Contract and Claim Falsifier.
2. Orchestration and Recovery Falsifier.
3. Security Trust-Boundary Falsifier.
4. Product and Simplicity Falsifier.

Every material review round is a sign-off attempt using all four installed
personas with `gpt-5.5`, `high`, fresh context, full scope, current anchors,
and the current claim ceiling. Any `REVISE_BEFORE_NEXT_PHASE` or `BLOCKED`
invalidates the round; repair, regenerate anchors, close reviewers, and launch
a fresh full-scope round.

Earlier separated reviewer prompts are historical compatibility resources only.
Current review authority is the typed four-persona review receipt plus the
current validator, review-target, and archive anchors.

Before any reviewer launch, run the Material Review Scope Gate. The gatekeeper
agent is `harness_material_review_scope_gatekeeper`, backed by
`agents/material-review-scope-gatekeeper.md` and
`schemas/review-materiality-gate.schema.json`. Its output classifies the launch
as full-scope material review, delta review, advisory review, or blocked before
review. Delta and advisory review outputs are never material `SIGN_OFF`.

## Parent And Lane Signals

Lane agents report completion to the parent with:

- lane id, branch, worktree, and current commit;
- owned paths changed and forbidden/shared paths untouched;
- commands run with exit codes and artifact paths;
- ready receipt path and claim ceiling;
- dirty worktree status or explicit preserved uncommitted paths;
- blockers, withheld claims, and next recommended parent action.

The parent then decides whether to steer, request repair, merge, run root
verification, advance dependent lanes, or tear down the lane.

Future Codex app worktree lane owners default to `gpt-5.5` with `low`
reasoning. Review agents are a different surface: material sign-off reviewers
use `gpt-5.5` with `high`.

Active repo files must justify their current function. Files kept only as
archives, old versions, or maybe-useful references should be removed from the
active repo instead of moved into in-repo archive folders.
