# Connector Strategy

The plugin should not make connectors core to correctness. Core correctness lives in file artifacts, schemas, and validator receipts.

## Core Runtime Integration

### Codex Goal Tools

Use when available for true goal binding:

- `get_goal`
- `create_goal`
- `update_goal`

If unavailable, record the gap and remove `goal_bound` claims.

### Codex App Thread Tools

Useful for real thread lanes. The connector should support:

- create thread;
- send to existing thread;
- archive/supersede stale thread;
- inspect session receipts.

Absence should not break file-contract validation.

### Automation Tools

Useful for periodic orchestration ticks. The connector must expose real automation artifacts. Chat-only automation claims are insufficient.

## Optional Work Management

GitHub and Linear are useful when the repo uses issues or PRs as the control plane. The plugin should keep them optional and parse their data into typed records.

## Human Review

Proof or similar document review tools are useful for human review of contracts. They do not prove runtime behavior.

## Runtime And UI Proof

Browser, Chrome, Playwright, and Computer Use are required only when the claim surface is UI/browser/desktop interaction.

## Security And API Guidance

Codex Security and OpenAI Developers are useful reference plugins for threat modeling, validation, and current platform integration. They should inform the implementation but not own the goal contract.

## Automation Tick Receipts

Connector discovery is not enough for automation-driven orchestration claims. Such claims require `schemas/automation-tick-receipt.schema.json`: enabled state, source artifact digest, last tick, next due, last success, drift verdict, and checked_at.
