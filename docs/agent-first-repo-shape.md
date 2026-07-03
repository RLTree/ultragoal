# Agent-First Repo Shape

This is the portable repo shape the plugin should create for fresh repos and
retrofit into existing repos.

## Fresh Repo

```text
AGENTS.md
AGENT_STANDARDS.md
agent-standards/README.md                  # directory index, not always-loaded law
agent-standards/01-namespace-and-progressive-disclosure.md
agent-standards/02-boundaries-validation-and-enforcement.md
agent-standards/03-execplans-worktrees-and-orchestration.md
agent-standards/04-security-reliability-and-product-cohesion.md
agent-standards/05-review-and-completion.md
agent-standards/06-standards-gardening.md
agent-standards/07-cli-authority-and-proof-surfaces.md
agent-standards/08-observability-and-repair-loop.md
agent-standards/09-product-success-and-quality-in-use.md
agent-standards/10-plugin-activation-and-distribution-surfaces.md
agent-standards/11-research-improvement-and-quality-gates.md
agent-standards/12-tool-risk-and-runtime-substrates.md
agent-standards/enforcement.json
agent-standards/enforcement.tsv
agent-standards/enforcement-audit.tsv
ARCHITECTURE.md
PLANS.md
DESIGN.md
FRONTEND.md
PRODUCT_SENSE.md
QUALITY_SCORE.md
RELIABILITY.md
SECURITY.md
scripts/check
scripts/check-agent-standards
COVERAGE_RECEIPT.json
docs/design-docs/index.md
docs/design-docs/core-beliefs.md
docs/exec-plans/active/
docs/exec-plans/completed/
docs/exec-plans/tech-debt-tracker.md
docs/generated/index.md
docs/product-specs/index.md
docs/references/index.md
validation_artifacts/
.agents/skills/
.agents/plugins/marketplace.json   # only when repo-local plugins are used
.codex/setup-worktree-env.sh
.codex/environments/environment.toml
.gitignore                         # ignores .codex-worktree/
```

The specialized root docs are required even when a surface is initially absent.
For example, a repo without a browser UI still keeps `FRONTEND.md` as the
routing point that says no frontend exists yet and names the trigger for adding
frontend proof. Do not create fake reference packs, generated DB schemas, or
product specs just to fill the tree; add only the index files until the repo
has that surface.

`AGENT_STANDARDS.md` is a compact router, not the full law. Detailed operating
standards live under `agent-standards/` in semantically named modules so agents
load the guidance that matches the task instead of carrying the whole standards
manual in every context.
`agent-standards/enforcement.*` records which standards are mechanized,
backlogged, blocked, or informational. Completion claims must update affected
repo-owned docs or record the stale/owed documentation update in the backlog,
active ExecPlan, standards enforcement rows, or claim ceiling.

`scripts/check` runs `scripts/check-agent-standards` first. If the repo has no
configured coverage command, it exits with a setup blocker instead of letting
test pass counts masquerade as coverage proof. Coverage claims use
`COVERAGE_RECEIPT.json` as the authoring template and a generated receipt under
`validation_artifacts/coverage/`.

When the repo owns an app, service, CLI, workflow engine, or UI, apply
`agent-runtime-legibility` and emit its receipt. Apply `agent-observability-stack` only when the user requests the observability setup, when the goal contract requires it, or when runtime/performance/UI claims cite observability evidence. Typical directories are:

```text
docs/runtime.md
validation_artifacts/runtime/
e2e/ or tests/e2e/
```

## Codex App Worktree Environment

Fresh init and retrofit should install the repo-managed Codex App worktree
environment described in `docs/codex-worktree-environment.md`. The setup must
derive the active root from `${CODEX_WORKTREE_PATH:-$PWD}`, write generated
state only under `.codex-worktree/`, and provide reusable setup, cleanup, env,
agent-docs, fast-check, and git-state actions in
`.codex/environments/environment.toml`.

Agents should source `.codex-worktree/env.sh` before validation commands so
temp files, Cargo target output, scratch space, and worktree-local ports are
isolated per Codex-created worktree.

## Existing Repo

Retrofit in this order:

1. Preserve current behavior and user changes.
2. Record current commands and blockers.
3. Add `AGENTS.md` as a short router.
4. Add compact standards router, standards modules, codemap, ExecPlan law,
   specialized root docs, routed docs indexes, proof root, and check wrapper.
5. Run the first gate.
6. Track stricter laws as backlog until the repo can enforce them.

## Hard Boundaries

- The harness cannot claim what its gate has not run.
- Fixture proof cannot support live-use claims.
- UI proof, CLI proof, API proof, package proof, and static proof are separate.
- Missing install, setup, auth, runtime, logging, metric, or trace requirements become explicit blockers.
- Repeated review feedback becomes standards or validators.
- Repeated plugin-flow confusion becomes a skill, resource-map, standards,
  validator, or persona repair through `standards-gardener`.

## Optional Observability Setup

`agent-observability-stack` is a separate setup skill. It is not installed into target repos by default. When requested, it installs the article-inspired observability surface throughout the repo: `docs/observability.md`, a local observe command or documented backend adapter, append-only events under `validation_artifacts/observability/`, an agent context summary, and a gate marker proving the setup is wired.
