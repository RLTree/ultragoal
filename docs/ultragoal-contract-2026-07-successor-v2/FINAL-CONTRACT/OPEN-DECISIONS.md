# Open Decisions Requiring External Authority

These decisions are intentionally not resolved by the contract author or Ultra agent. Safe defaults preserve work and lower only dependent claims.

| Decision | Question | Authority | Safe default | Affected claims |
| --- | --- | --- | --- | --- |
| OD-001 | Public, workspace-private, repository-local, or mixed plugin distribution channels | Human product/release owner | Repository-local and unpublished | CL-INSTALL, CL-DISCOVERY, CL-RELEASE, CL-COMPLETION |
| OD-002 | License and third-party notice policy | Human legal/product owner | No public distribution claim | CL-PACKAGE, CL-RELEASE, CL-COMPLETION |
| OD-003 | Supported OS, architecture, Codex/ChatGPT host, and minimum-version matrix | Human product owner | Claim only environments independently exercised | CL-INSTALL, CL-DISCOVERY, CL-RUNTIME, CL-RELEASE, CL-COMPLETION |
| OD-004 | Whether OpenTelemetry export is a V1 requirement or an optional post-V1 adapter | Human product/privacy owner | Local-only observability | CL-OBSERVABILITY, CL-RELEASE, CL-COMPLETION |
| OD-005 | Release signing/provenance identity, issuer/key, transparency, and offline verification policy | Human security/release owner | Digest/provenance without signature claim | CL-PACKAGE, CL-RELEASE, CL-COMPLETION |
| OD-006 | Permitted plugin hooks, MCP/connectors, app actions, and trust prompts | Human security/product owner | No optional executable extension | CL-DISCOVERY, CL-RUNTIME, CL-FIT, CL-RELEASE |
| OD-007 | Telemetry consent, local retention, deletion, and external-export data policy | Human privacy/product owner | Local bounded events, no export | CL-OBSERVABILITY, CL-RELEASE, CL-COMPLETION |
| OD-008 | Compatibility window and removal version for lane/gate commands, skills, agents, schemas, and aliases | Human product/maintainer owner | Route non-destructively with warnings; do not claim retirement complete | CL-RUNTIME, CL-REAL-JOURNEY, CL-RELEASE, CL-COMPLETION |
| OD-009 | Approval for destructive deletion of historical/legacy files and generated artifacts after migration | Human repository owner | Archive or leave in place as non-authoritative | CL-RELEASE, CL-COMPLETION |
| OD-010 | Final package, binary, and marketplace naming where current names conflict or are externally published | Human product/release owner | Preserve `harness-ultragoal` product identity and avoid public rename | CL-PACKAGE, CL-INSTALL, CL-RELEASE |

The Ultra root may gather evidence and present alternatives, but it must not infer consent, licensing, publication, telemetry, signing identity, support promises, compatibility deadlines, or destructive authorization from this prompt.
