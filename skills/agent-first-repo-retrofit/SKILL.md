---
name: agent-first-repo-retrofit
description: Use when converting an existing repo into an agent-first repo without losing current conventions, user changes, build truth, or production constraints.
---

# Agent-First Repo Retrofit

Use this when the repo already exists. Retrofit by discovering current truth,
then installing the smallest durable harness that improves agent work without
rewriting the project around the harness.

This skill is subordinate to `harness-ultragoal:fit-repo` for Harness
Ultragoal retrofits. A repo is not retrofitted, harnessed, ultragoal-ready, or
product-ready until `fit-repo` emits a schema-valid receipt or records a
withheld-claim blocker naming the missing setup surface.

## Retrofit Rules

- Preserve user changes and current conventions.
- Establish a baseline before refactoring guidance.
- Do not invent standards the repo cannot enforce.
- Record blockers instead of pretending the retrofit is complete.
- Keep the first gate focused; expand it only after it runs.

## Procedure

1. Inspect existing instructions, build files, tests, CI, docs, and runtime
   entrypoints.
2. Map current proof surfaces: static checks, tests, coverage, UI, API,
   package/install, production logs, and live-use evidence.
3. Create a retrofit backlog with rows classified as `now`, `later`,
   `blocked`, or `withheld-claim`.
4. Add a short `AGENTS.md` router if none exists, or shrink a bloated one.
5. Add or update compact `AGENT_STANDARDS.md`, the `agent-standards/`
   modules, `ARCHITECTURE.md`, `PLANS.md`, `DESIGN.md`, `FRONTEND.md`,
   `PRODUCT_SENSE.md`, `QUALITY_SCORE.md`, `RELIABILITY.md`, and
   `SECURITY.md`. Treat `PLANS.md` as stable ExecPlan law, not project state;
   active status, worker/thread ids, phase progress, backlog items, receipt
   state, and completion claims move to active ExecPlans, lane registry,
   verification backlog, completion manifest, receipts, or amendments.
6. Add or update `agent-standards/enforcement.json`,
   `agent-standards/enforcement.tsv`, `agent-standards/enforcement-audit.tsv`,
   and `scripts/check-agent-standards` from the plugin templates.
7. Add `scripts/check-agent-standards` to the fast/local gate, or record the
   exact blocker and withheld claims in `VERIFICATION_BACKLOG.json`.
8. Add or bridge `docs/design-docs/`, `docs/exec-plans/`, `docs/generated/`,
   `docs/product-specs/`, and `docs/references/` with index files and a
   tech-debt tracker. Existing historical docs can stay where they are if the
   index names the bridge and migration debt.
9. Wrap existing checks behind `scripts/check` without changing behavior first.
10. Add or update `.codex/setup-worktree-env.sh`,
   `.codex/environments/environment.toml`, and `.gitignore` without hardcoding
   a worktree path.
11. Mirror `.codex/setup-worktree-env.sh` to
   `~/.codex/bin/codex-worktree-env` when installing locally.
12. Add `validation_artifacts/` and receipt templates.
13. If product-impacting claims or surfaces exist, install or block Product
    Fitness surfaces: `PRODUCT_FITNESS.md`, `PRODUCT_FITNESS_RECEIPT.json`,
    standards row `STD-PRODUCT-FITNESS-001`, and the product-fitness receipt
    path. Withhold product readiness, release, daily-driver, and material
    product sign-off claims until Product Fitness proof exists.
14. Expose `.codex/automations/ultragoal-orchestrator/` when the repo will run
    multi-lane Ultragoals. Do not enable it until active setup, first-wave lane
    launch, and transition receipt exist.
15. Emit or refresh the fit-repo receipt. If the receipt cannot be emitted,
    record the blocker and withhold retrofitted, harnessed, ultragoal-ready,
    product-ready, release, and material sign-off claims.
16. Only then propose structural refactors or stricter gates.

## Anti-Laziness Checks

- "Existing CI passes" is not proof unless the command and run are named.
- "No tests" requires a backlog row with the first useful gate to add.
- "Legacy structure" is not a license to skip codemap boundaries.
- "Too big to retrofit" means sequence the retrofit, not abandon it.
- "Docs updated" is insufficient without a fresh-agent discovery check.
- A monolithic `AGENT_STANDARDS.md` is retrofit debt. Preserve its content by
  routing it into semantically named standards modules before adding more law.
- Standards compliance is retrofit debt until `agent-standards/enforcement.*`
  exists, each row is mechanized/backlogged/blocked/informational, and the
  checker has run or produced an explicit blocker.
- Archive-only files are retrofit debt. If a file has no current operational
  purpose, remove it instead of moving it to an in-repo archive.
- Codex environment setup is invalid if it writes generated state outside
  `.codex-worktree/` or uses a hardcoded worktree path.
- Future Codex app worktree lane owners default to `gpt-5.5` with `low`
  reasoning. Record any higher reasoning choice in the ExecPlan with the risk
  that justifies it.

## Acceptance

Accepted only when:

- current repo truth is documented with fresh commands or exact blockers;
- a new agent can find guidance from the repo root;
- a new agent can route from `AGENT_STANDARDS.md` to the relevant
  `agent-standards/` module without reading a mega document;
- routed docs distinguish operating law, architecture, design, product sense,
  references, generated docs, security, reliability, and debt;
- at least one real check runs through the new entrypoint;
- the standards enforcement checker runs or affected claims are withheld;
- missing proof is explicit in a verification backlog;
- no existing workflow is broken by the harness layer.
- Codex App-created worktrees have isolated scratch, state, home, temp, Cargo
  target, and port environment variables after sourcing
  `.codex-worktree/env.sh`.
- archive-only or purposeless files are removed or explicitly justified by an
  active operational function.
