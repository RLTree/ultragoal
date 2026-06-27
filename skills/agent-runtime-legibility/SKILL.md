---
name: agent-runtime-legibility
description: Use when a repo contains an app, service, workflow engine, CLI, or UI that agents must run, inspect, debug, or prove end-to-end.
---

# Agent Runtime Legibility

Use this when static files are not enough. Agents need observable runtime
truth: commands, health, logs, state, receipts, and UI proof that map to user
claims.

## Required Surfaces

- Local run command with expected readiness signal.
- Health or smoke command that exits nonzero on failure.
- Log location with stable names and scrubbed secrets.
- State root, cleanup rules, and isolation strategy.
- CLI help or API docs generated from the real entrypoint.
- UI proof path when users interact through UI.
- Receipt format for runtime runs and failures.

## Procedure

1. List each user-facing workflow and its strongest proof surface.
2. Add a cheap smoke path before adding broad e2e paths.
3. Make all runtime commands use local/loopback-safe defaults.
4. Ensure failures include actionable diagnostics, not generic "failed".
5. Capture receipts under `validation_artifacts/`.
6. Separate claims by surface: static, fixture, API, CLI, UI, package, live-use.
7. Add live beneficial e2e only when the feature itself is being claimed.

## Acceptance

Accepted only when a fresh agent can:

- start or intentionally skip the runtime with a named blocker;
- find logs and state without asking the user;
- prove one meaningful workflow through the strongest available surface;
- avoid using fixture or smoke proof for live-use claims.
