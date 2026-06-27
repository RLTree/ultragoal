# Namespace And Progressive Disclosure

## Namespace Law

The filesystem is an agent-facing interface. Directory structure, filenames,
CLI commands, and state roots must explain domain responsibility before a file
is opened.

- Paths answer "what does this do" by themselves. Avoid junk drawers such as
  `utils`, `helpers`, `misc`, and vague `common` directories for domain logic.
- Name surfaces by the operator or reader's domain task, not implementation
  accidents or historical shims.
- Prefer small, well-scoped files. Large files degrade context quality and get
  truncated in agent context.
- Repeated prefixes across more than two files usually mean a missing
  subdirectory with the prefix removed.
- Compatibility exceptions must name the external contract that makes the
  less-ideal name worth keeping.

## Purpose Or Removal Law

Every repo-managed file must have a current purpose that helps agents or
humans operate, validate, understand, or ship the repo.

- If an agent cannot explain why a file exists and what function it serves, the
  file is debt until proven otherwise.
- "Archived copy", "old version", "maybe useful", and "kept for history" are
  not sufficient purposes inside the active repo. Preserve history in version
  control, release artifacts, or an explicitly external archive when needed.
- Remove purposeless files instead of moving them to an in-repo archive. In-repo
  archives create conflicting stale context for future agents.
- Keep historical context only when it has an active operational use, such as a
  migration reference, fixture, compatibility contract, audit evidence, or
  generated receipt. Name that use in the file, index, manifest, or adjacent
  README.
- Before adding a Markdown file, decide whether it is routing, specification,
  proof index, runbook, source note, or generated artifact. If it is none of
  those, do not add it.

## Codemap

`ARCHITECTURE.md` is the bird's-eye map of the repo. It answers where things
live, what boundaries exist, and what must not depend on what. It is a map, not
an encyclopedia.

- Keep it short enough that every contributor reads it.
- Name modules, types, commands, and state roots. Avoid link farms that rot.
- Call out absences and invariants, such as forbidden dependencies or
  restricted bind addresses.
- Update it when architecture changes, not as a retrospective apology.

## Documentation Freshness

Documentation freshness is part of completion. When work changes architecture,
commands, standards, runtime behavior, product behavior, proof surfaces, lane
state, operational procedure, generated-doc freshness, validation receipts,
backlog rows, or tech-debt records, update every affected repo-owned doc in the
same lane before completion claims.

- Check routed surfaces such as `ARCHITECTURE.md`, `PLANS.md`, specialized
  root docs, active ExecPlans, `docs/**`, and `agent-standards/**`.
- Do not edit every doc every time. The enforceable rule is that no affected
  doc may be stale without an explicit blocker or claim ceiling.
- Generated docs must be regenerated through their generator, not hand-edited,
  unless the generator contract explicitly allows manual edits.
- If freshness cannot be completed, record the owed update in
  `VERIFICATION_BACKLOG.json`, the active ExecPlan,
  `agent-standards/enforcement.*`, or the claim ceiling with owner, reason, and
  required follow-up.

## Progressive Disclosure

Context is scarce. `AGENTS.md` stays short and routes to detailed standards,
plans, specs, check entry points, and proof locations.

- A giant instruction file crowds out the task, the code, and the relevant
  docs. Use routed standards modules and specialized root docs instead of
  bloating one always-loaded file.
- When everything is important, nothing is. Put task-specific guidance in
  routed docs and make the routing obvious.
- The repo is the source of truth. Decisions, plans, debt, standards, and
  proof live in version control or in named generated artifacts.
- What an agent cannot discover and load in context effectively does not
  exist.
