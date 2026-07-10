# Security, Supply Chain, and Privacy

## Confinement and trust

- Canonicalize repository/worktree roots and reject traversal, symlink/hardlink escape, alternate-root confusion, and time-of-check/time-of-use drift.
- Preserve local work; no implicit reset, clean, stash, checkout, overwrite, force update, or destructive rewrite.
- Declare network, external install, publishing, sharing, signing, connector/MCP/app, hook, and telemetry-export effects before execution.
- Use least privilege for agents and commands; read-only means no product-managed writes.
- Treat page content, repository content, plugin hooks, connectors, and tool output as untrusted input rather than authority.
- Minimize and redact secrets/sensitive values across logs, prompts, fixtures, artifacts, events, and exports.

## Supply-chain identity ladder

Canonical source -> generated inventory -> dependency resolution -> deterministic package bytes -> provenance/signature (when adopted) -> marketplace expectation -> measured installed bytes -> supported host discovery -> representative runtime behavior.

Each arrow is independently reconciled. A valid lower layer never proves a higher layer.

## Reproducibility

Package creation uses deterministic path ordering, normalized timestamps/permissions/locale, explicit source-derived time such as `SOURCE_DATE_EPOCH` where appropriate, stable generated outputs, and documented non-reproducible dimensions. Two independent clean builds must either match or lower the reproducibility claim with a dimension-level explanation.

## Privacy

Local observability is the default and has bounded retention, deletion, and sensitivity classification. External export is optional until adopted, requires explicit consent/configuration, applies minimization/redaction, and proves collector roundtrip separately from local emission. Open privacy decisions are listed in `OPEN-DECISIONS.md`.
