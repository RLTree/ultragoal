# Harness Ultragoal PLANS And Orchestrator Automation Hardening

This active contract records the implementation binding for
`harness-ultragoal-plans-and-orchestrator-automation-hardening`.

Runtime binding note: `set_goal()` was requested, but this Codex tool surface
does not expose `set_goal()`. Tool discovery exposed no callable `set_goal`;
the runtime-supported binding was `create_goal`, which was called before
implementation edits with a compressed version of this contract.

## Purpose

Close two recurring Harness Ultragoal enforcement gaps:

1. Downstream repos rewrite `PLANS.md` as active project state instead of
   treating it as stable ExecPlan law.
2. Orchestrator sessions create weak automations that are not self-contained,
   not bound to the correct thread/workspace, not explicit about tools, skills,
   evidence, checks, action policy, output shape, or claim ceiling.

## Core Thesis

Harness Ultragoal has two complementary contract surfaces:

- active setup and launch use goal binding;
- idle orchestration after first-wave lane launch uses a self-contained
  heartbeat automation contract.

The automation is not a generic reminder. It is the continuation contract for
the Ultragoal orchestrator after active setup, lane contracts, lane worktrees,
and first-wave lane owners are launched.

## Required Repairs

### PLANS.md

- Add a blunt banner making `PLANS.md` stable ExecPlan law.
- Forbid active project status, worker/thread ids, phase progress, backlog
  items, receipt state, and completion claims in `PLANS.md`.
- Route project state to active ExecPlans, `LANE_REGISTRY.json`,
  `VERIFICATION_BACKLOG.json`, `COMPLETION_MANIFEST.json`, receipts, or
  `AMENDMENTS.jsonl`.
- Add deterministic template-integrity validation and red fixture coverage.
- Add standards row `STD-PLANS-001`.
- Update init, retrofit, and standards-gardener guidance.

### Orchestrator Automation

- Add `templates/.codex/automations/ultragoal-orchestrator/automation.toml`
  plus a self-contained prompt template.
- Encode active goal setup, repo preparation, macro-lane ExecPlan validation,
  branch/worktree creation, first-wave lane launch, automation install,
  transition receipt, idle heartbeat monitoring, and next active actions.
- Require fields for automation id, thread id, goal id, repo root, lane
  registry, active ExecPlan directory, backlog, completion manifest, automation
  tick receipt, package/cache/install surfaces, source identity, and prompt
  digest/source.
- Require the prompt to name tools, skills, evidence cursors, privacy rules,
  mutation limits, output contract, action policy, cadence rules, and claim
  ceiling.
- Add deterministic template-integrity validation and red fixture coverage.
- Add standards row `STD-AUTOMATION-001`.
- Update ultragoal, orchestrator-reconciler, execplan-lane, init, retrofit,
  standards-gardener, and proof-gate guidance.

### Required External ExecPlans

This goal contract incorporates the following ExecPlans as mandatory scope. These
files are not background reading, discretionary guidance, or reviewer discretion. They
are binding implementation contracts. Removing, weakening, ignoring, or failing
to validate any referenced ExecPlan blocks this goal.

1. [`mandatory-coverage-authority-and-enforcement.md`](mandatory-coverage-authority-and-enforcement.md)
   - Enforces the definition of coverage proof, 100% policy, ratchet ceilings,
     coverage receipts, coverage substitution rejection, coverage fixtures, and
     coverage standards rows.
2. [`mandatory-coverage-scope-authority-and-anti-theater.md`](mandatory-coverage-scope-authority-and-anti-theater.md)
   - Enforces source-scope authority, `.harness/coverage-manifest.json`,
     changed-file coupling, behavior-class dimensions, freshness binding,
     tool-generated proof, fast/full gate separation, and anti-theater fixtures.
3. [`mandatory-plugin-product-cohesion-and-fit-repo-authority.md`](mandatory-plugin-product-cohesion-and-fit-repo-authority.md)
   - Enforces the plugin as a cohesive product: one mandatory `fit-repo` entry
     contract, strict init/retrofit receipts, setup dependency closure, plugin
     flow graph authority, source/install/cache/app-registry alignment, standards
     promotion, orchestrator lifecycle, and plugin-product journey receipts.
4. [`mandatory-product-fitness-quality-in-use-enforcement.md`](mandatory-product-fitness-quality-in-use-enforcement.md)
   - Enforces Product Fitness and Quality-In-Use: target audience, job,
     context, desired outcome, discovery evidence, user evidence,
     quality-in-use dimensions, accessibility, cognitive-load and recovery
     burden, continuance proof, substitution rejection, standards rows, and
     Product Fitness receipts.

Every implementation, validator, fixture, standards row, package/cache sync,
review packet, and final report for this goal must name which of these ExecPlans
it satisfies. A claim that omits a required ExecPlan is incomplete. A reviewer
round that does not receive all four ExecPlans as current scope is invalid.

## Verification Gates

- Full package audit must pass.
- All four required external ExecPlans must remain present, linked from this
  file, and supplied to every implementation worker, validator repair, review
  packet, and final report for this goal.
- `scripts/check-agent-standards` or the Rust `agent-standards-enforcement`
  gate must cover `STD-PLANS-001`, `STD-AUTOMATION-001`,
  `coverage-proof-accountability`, `coverage-scope-authority`, and
  `plugin-product-cohesion-authority`.
- Coverage schema, standards, script wiring, and validator gates must enforce
  `mandatory-coverage-authority-and-enforcement.md`.
- Coverage scope schema, standards, script wiring, and validator gates must
  enforce `mandatory-coverage-scope-authority-and-anti-theater.md`.
- Plugin product cohesion schema, standards, script wiring, package dependency
  closure, fit-repo receipts, flow graph validation, and validator gates must
  enforce `mandatory-plugin-product-cohesion-and-fit-repo-authority.md`.
- Product Fitness schema, standards, script wiring, receipt validation, claim
  classification, red fixtures, green fixtures, setup/retrofit routing, and
  validator gates must enforce
  `mandatory-product-fitness-quality-in-use-enforcement.md`.
- Red fixtures must prove active project state in `PLANS.md` fails.
- Red fixtures must prove weak automation contracts fail.
- Red fixtures must prove coverage-substitution and lower-than-100 completion
  claims fail.
- Red fixtures must prove missing fit-repo entrypoint, missing fit-repo receipt,
  source/install/cache/app-registry proof substitution, plugin flow graph gaps,
  unshipped setup dependencies, missing product journey receipt, missing
  standards-gardener promotion, and missing multi-lane transition receipt fail.
- Red fixtures must prove product success substitution, missing Product Fitness
  receipt, missing audience/job/context/outcome binding, opinion-only
  validation, missing accessibility proof, missing continuance proof for
  daily-driver claims, and engine-only false-positive protection fail or pass for
  the intended reason.
- Green fixtures must prove valid 100% coverage, valid ratchet floor, valid
  exclusions, no-coverage/no-completion, and configured-command receipt cases.
- Green fixtures must prove valid fit-repo receipts for fresh and retrofit
  targets, valid package dependency closure, valid source/install/cache alignment
  at the claimed proof surface, valid standards-gardener disposition, valid
  multi-lane transition receipt, and valid plugin product journey receipt.
- Green fixtures must prove valid Product Fitness receipts for product claims,
  valid withheld product claims, and valid engine-only runtime claims that do not
  require Product Fitness proof.
- Existing valid fixtures must continue to pass.
- Source, installed plugin, and cache package must point to the same candidate
  when synced.

## Claim Ceiling

Supported claims must not exceed package/static/fixture proof plus installed and
cache package proof when freshly regenerated. This work does not prove live
Codex app registry, marketplace, launcher, or runtime visibility.
