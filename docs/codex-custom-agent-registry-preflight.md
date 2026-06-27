# Codex Custom Agent Registry Preflight

Disk install is not active registry proof.

For material review rounds, the package must distinguish these surfaces:

1. Plugin cache synced: the plugin files exist under the installed plugin cache.
2. Global custom-agent TOML present: the agent files exist under
   `~/.codex/agents/`.
3. Active Codex registry exposed: the already-running Codex session can discover
   and spawn the current custom agent types through the multi-agent registry.

The first two are setup facts. They do not prove a running Codex session can
spawn the reviewers. Already-open sessions may keep an older registry until the
app, worker, or thread environment refreshes.

## Required Receipt

Before material sign-off review, capture a registry exposure receipt with
schema:

```text
harness-ultragoal.multi-agent-registry-exposure.v1
```

The receipt must be referenced by `live_registry_exposure` in the typed review
round receipt. Each current reviewer persona row must record:

- `agent_type`;
- `persona`;
- `custom_agent_path`;
- `disk_cache_synced`;
- `global_toml_present`;
- `exposed`.

Review is blocked unless all four current persona agent types are exposed by
the active registry:

- `harness_contract_claim_falsifier`;
- `harness_orchestration_recovery_falsifier`;
- `harness_security_trust_boundary_falsifier`;
- `harness_product_simplicity_falsifier`.

## Failure Policy

If files are present on disk but the active registry does not expose the current
agent types, material review is `BLOCKED`. Do not fall back to the old reviewer
team or generic reviewers for material sign-off.

The safe repair is to refresh the Codex app/session registry, recapture the
registry exposure receipt, and restart the review round with fresh reviewers.
