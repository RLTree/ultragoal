# Schema Resolver Contract

`schemas/schema-catalog.json` is part of the package contract. Validators must preload the catalog before validating package artifacts.

Rules:

- Resolve every `https://harness-ultragoal.local/schemas/...` id to the package-relative file listed in the catalog.
- Resolve relative `$ref` values against the same catalog.
- Do not fetch schema ids over the network. A remote schema fetch attempt is a validation failure.
- Offline validation of the canonical templates is a Phase 1 acceptance gate.
- If a schema is added, removed, or renamed, update the catalog in the same change.

This keeps the plugin portable and prevents fresh installs from depending on undocumented local resolver state.
