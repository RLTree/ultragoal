# SECURITY

Local does not mean trusted. Treat user input, CLI args, environment variables,
file imports, browser or IPC payloads, database or state reads, model output,
tool/MCP output, memory retrieval, logs, search results, and generated tests as
untrusted until parsed and authorized.

## Required Security Gates

Replace these with the repo's real commands:

- Security hygiene check.
- Parse-boundary or input-contract check.
- Test/coverage gate for sensitive behavior.

## Hard Rules

- Parse and authorize before side effects.
- Keep local-only servers loopback-only unless a documented requirement,
  explicit approval, and regression tests say otherwise.
- Do not commit raw secrets, prompt bodies, transcripts, message bodies,
  credentials, personal data, or unredacted production logs.
- Secrets are referenced by name and resolved only at dispatch or runtime
  boundaries.
- Shell execution paths need narrow arguments, parser/authorization proof, and
  tests or blocker notes.
- External network, destructive, credential, financial, permission, or
  production-impacting actions require explicit human approval.
- Dependency additions require a receipt covering alternatives, license, attack
  surface, maintenance, runtime cost, and build cost.

When a sensitive surface changes, add negative tests or record a blocker in the
active ExecPlan. Do not rely on review prose alone.
