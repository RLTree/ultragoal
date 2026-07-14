# FRONTEND

This repository has no repo-owned browser, desktop, mobile, or terminal UI.
Its user-facing surfaces are the Codex host's plugin discovery UI, installed
skills and agents, and the `ultragoal` command-line interface. The host Plugins
UI is an external proof surface; source files and CLI tests cannot prove that
the host discovered or activated the plugin.

## Surface Routing

- Plugin identity and resources: `.codex-plugin/plugin.json`,
  `plugin-manifest-draft.json`, `skills/`, `agents/`, and `install/`.
- CLI grammar, help, and diagnostics: `validator/src/cli/successor/`.
- Public CLI behavior: `validator/src/cli/successor_public/` and the owning
  semantic domain named in `ARCHITECTURE.md`.
- Host-visible installation, cache, discovery, runtime identity, and Plugins UI
  proof: representative supported-host journeys, kept separate from source and
  package validation.

Do not add a repo-local web server or UI framework to simulate host behavior.
If a future requirement adds a repo-owned visual surface, update this file and
`ARCHITECTURE.md` before implementation with its entry point, routes, bind
address, state model, accessibility contract, and interaction proof.

## Current Proof Spine

- `ultragoal --json --help` proves only the built binary's public grammar.
- `ultragoal --json inspect capabilities` proves only the invoked runtime's
  reported capabilities.
- Package tests prove package contents, not installation or discovery.
- Supported-host lifecycle tests prove typed adapter behavior, not a live host
  session.
- Plugins UI or runtime claims require fresh app-visible evidence bound to the
  installed package identity and current candidate.

Read, help, parse, inspect, query, diagnose, and next-action routes must remain
recursively zero-write. Diagnostics must be stable, redact untrusted input, and
identify the smallest causal repair. CLI evidence never substitutes for visual
host evidence, and host screenshots never substitute for CLI behavior tests.
