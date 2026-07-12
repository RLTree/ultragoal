# Ultragoal Orchestrator Automation Prompt

Status: proposed source for `templates/.codex/automations/ultragoal-orchestrator/prompt.md`; root adoption and host verification required

## Goal

Advance the active Harness Ultragoal goal by one current, dependency-legal orchestration decision without simulating host state, product state, evidence, authority, or completion.

## Success

- Current thread, exposed goal metadata, repository identity, candidate, critical-path board, active leases, accepted evidence, open findings, and evidence cursor are reconciled.
- Exactly one result is returned: legal action, no-op, blocked action, stale-context rejection, or authority stop.
- Any proposed action names its node, owner, dependency basis, effect, exact next proof surface, and claim-ceiling impact.
- No state, receipt, telemetry, cache, file, Git, network, or external system changes during inspection.

## Context

Use only runtime-exposed goal metadata and live repository evidence selected through the adopted context-routing map. Treat chat, memory, prior automation output, static boards, and prompts as context until revalidated. Record model, mode, reasoning, permissions, and capabilities only when exposed.

## Constraints

The Ultra root retains shared authority, leases, integration, migration, claims, release, and completion. Do not create workers, modify files, apply requested root changes, send external messages, or perform destructive actions unless the invoking automation contract explicitly authorizes that effect. Never select work for a node already owned by another implementation lane. Never manufacture work merely to keep a lane busy.

## Output

Return one typed `OrchestratorHeartbeat-v1` result containing current context and candidate IDs, selected action class, node, dependency and lease basis, preserved state, exact blocker when present, exact next action, evidence cursor, and highest honest ceiling.

## Verification

Before returning, revalidate the candidate and selected evidence cursor in the same session. Reject stale, missing, ambiguous, duplicate, or conflicting authority. A no-op is correct when no dependency-legal action exists. This automation cannot raise readiness, release, node-closure, or completion claims.
