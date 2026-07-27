# Agent Standards

This is the compact operating-law router for agent-first repositories.
Detailed standards live in `agent-standards/`.

## Module routing

| Task shape | Load |
| --- | --- |
| Names, paths, codemap, docs, generated-doc freshness | `agent-standards/01-namespace-and-progressive-disclosure.md` |
| Tests, parsing, validators, deterministic checks, coverage claims | `agent-standards/02-boundaries-validation-and-enforcement.md` |
| ExecPlans, lanes, worktrees, fan-in | `agent-standards/03-execplans-worktrees-and-orchestration.md` |
| Security, reliability, product surfaces, human attention | `agent-standards/04-security-reliability-and-product-cohesion.md` |
| Independent review, proof, claim ceilings, completion | `agent-standards/05-review-and-completion.md` |
| Recurring friction and standards gardening | `agent-standards/06-standards-gardening.md` |
| CLI authority and proof-surface separation | `agent-standards/07-cli-authority-and-proof-surfaces.md` |
| Observability, current state, next action, repair loops | `agent-standards/08-observability-and-repair-loop.md` |
| Product Success, Fitness, Cohesion, quality in use | `agent-standards/09-product-success-and-quality-in-use.md` |
| Plugin activation and distribution surfaces | `agent-standards/10-plugin-activation-and-distribution-surfaces.md` |
| Research, improvement loops, quality gates | `agent-standards/11-research-improvement-and-quality-gates.md` |
| Tool risk, runtime substrates, dependencies, privacy | `agent-standards/12-tool-risk-and-runtime-substrates.md` |

## Non-negotiable rules

- The repository is current truth. Memory, chat, and historical artifacts are
  context only.
- Preserve user changes and isolate concurrent work.
- Parse external inputs before product behavior and authorize before effects.
- No model output, worker, receipt, generated row, or reviewer mints root
  authority or raises a claim ceiling.
- Source, package, install, host discovery, runtime, journey, and release are
  separate proof surfaces.
- Documentation freshness follows affected current claims. A stale unrelated
  projection is a named gap, not a reason to refresh every artifact.
- Coverage proof is required only for a coverage or source-completeness claim.
  Ordinary implementation still runs focused changed-behavior checks.
- Parallel lanes require frozen shared interfaces, exclusive path and semantic
  ownership, independent local oracles, and a single root integration owner.
- Root owns shared schemas, dependencies, public grammar, migrations, effect
  authority, fan-in, and claim decisions.
- Ordinary work uses focused checks. A consequential authority, security,
  custody, concurrency, recovery, migration, install, host, or external-effect
  boundary uses one relevant independent falsifier. The full four-lens team is
  reserved for a current release-grade or repository-wide completion claim
  spanning all four lenses.
- A completed review closes while candidate bytes and consumed dependencies
  remain unchanged. Unrelated age or receipt drift does not reopen it.
- Proof artifacts are exceptional. Persist only the smallest canonical item a
  current claim, custody handoff, irreproducible observation, recovery need, or
  authorized release consumes.
- Do not claim done, ready, fixed, passing, complete, or production-ready
  without fresh evidence from the named surface.

## Independent review lenses

1. Contract and Claim.
2. Orchestration and Recovery.
3. Security Trust Boundary.
4. Product and Simplicity.

Select only the lenses needed to falsify the current claim. Before review,
bind the exact candidate and diff, claim, failure model or journey, cheapest
credible oracle, and claim ceiling.

## Completion report

For non-trivial work, report outcome, exact verification and result, residual
gaps, security/performance implications or `N/A`, and the supported and
unsupported claim ceiling.
