---
name: standards-gardener
description: Recurring cleanup for repo-law drift, stale docs, stale proof, and agent-legibility entropy.
---

# Standards Gardener

Use this skill for recurring maintenance after the plugin is installed.

## Scope

The gardener reduces entropy. It does not broaden product scope.

## Checks

- stale active ExecPlans;
- completed plans left active;
- stale worktrees;
- dirty abandoned worktrees;
- stale ready receipts;
- verification backlog rows with no owner or next action;
- old claim ceilings contradicted by current code;
- oversized files;
- purposeless or archive-only files kept in the active repo;
- junk-drawer namespaces;
- stale allowlist entries;
- docs that point to removed paths;
- generated artifacts that should be regenerated;
- named plugin/process/runtime authority falling back to a different authority
  source without a receipt and explicit claim ceiling;
- source/package checks being used to close installed plugin, cache package,
  app registry, launcher, marketplace, or runtime visibility claims;
- browser, UI, recording, replay, transcript, video-alignment, or runtime proof
  missing exact tool identity, artifact digests, coordination receipt, or
  parent-operation receipt;
- missing or disabled transcript finalization, cleanup, or alignment being
  treated as informational while downstream claims trust final words;
- raw private transcript, audio, prompt, message, or event payloads preserved
  in fixtures, logs, prompts, or review packets where category-only status,
  digests, redacted snippets, or leak summaries would suffice;
- proof-bearing generated artifacts whose behavior-affecting derived values
  are not recomputed from canonical current inputs or canonical-input digests;
- repeated review findings that should become mechanical gates.
- repeated wrong skill routing, missing plugin resource use, or user
  micromanagement that should become a plugin cohesion improvement.
- missing, stale, unclassified, or overclaimed `agent-standards/enforcement.*`
  rows.
- material review scope drift, especially attempts to turn material sign-off
  into delta-only review or to launch reviewers when deterministic preflight
  already blocks.
- `PLANS.md` drift where active project state, worker/thread ids, phase
  progress, backlog rows, receipt state, or completion claims are written into
  the stable ExecPlan law template.
- weak Ultragoal orchestrator automations that are generic reminders, miss
  thread/goal/workspace binding, omit tools/skills/evidence cursors, lack a
  `DONT_NOTIFY` path, or leave mutation authority unbounded.
- product-impacting success, readiness, daily-driver, release, or material
  product sign-off claims that lack Product Fitness receipts or substitute
  reviewer agreement, install success, smoke tests, test pass counts, fixture
  pass counts, package publication, first use, feature delivery, or Product
  Cohesion alone.

## Signal Threshold

Act immediately on a moderate or severe issue that is likely to recur:
false completion, wrong review team/model/reasoning, unsafe orchestration,
broken worktree initialization, product cohesion miss, security/trust-boundary
gap, purposeless repo files that can mislead future agents, or plugin flow
confusion that forced user micromanagement. Treat severe authority fallback,
source/installed/cache substitution, raw private artifact leakage, or transcript
quality reuse, or Product Fitness proof substitution as immediate promotion
candidates even if observed only once.

Wait for repeated evidence before changing standards for low-signal one-offs.
Do not add hooks unless they are cheap, deterministic, low-context, and prevent
a real recurring or severe failure at the moment it happens.

## Promotion Ladder

Prefer the smallest durable fix:

1. Validator, linter, schema, red fixture, or check for deterministic rules.
2. Standards enforcement row update when the obligation must be tracked across
   setup, resume, review, and completion.
3. Skill prompt update for workflow routing.
4. Persona prompt update for repeated review misses.
5. Routed standards module for semantic cross-repo law.
6. Plugin resource-map update for skill/resource cohesion.
7. Hook only when the trigger is cheap and clearly valuable.
8. Backlog row when the fix is real but too broad for the current pass.

When the finding is a file with no current function, prefer removal over
archival inside the repo. In-repo archive folders usually preserve the exact
stale context the gardener is meant to eliminate.

## Output

Produce small targeted diffs or a report with exact follow-up lanes. Do not create broad cleanup projects without a contract.

## Acceptance

A gardening pass is successful when it removes or classifies drift and leaves the repo easier for the next agent to navigate.
