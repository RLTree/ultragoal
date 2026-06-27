# Install And Visibility

This package follows the Codex plugin shape:

```text
.codex-plugin/plugin.json
skills/
custom-agents/
install/personal-marketplace.example.json
```

## Personal Plugin Install

The official local plugin route is a personal marketplace:

1. Copy this package to `~/.codex/plugins/harness-ultragoal`.
2. Add or update `~/.agents/plugins/marketplace.json` using
   `install/personal-marketplace.example.json`.
3. Restart Codex.
4. Open Plugins, choose the local marketplace, and install/enable
   `harness-ultragoal`.

In the personal marketplace example, `source.path` is
`./.codex/plugins/harness-ultragoal`. This is the official personal-marketplace
pattern and resolves from `$HOME`, so it points at
`$HOME/.codex/plugins/harness-ultragoal`.

The local copy/cache sync surface is narrower than app install. It may prove
that the package was copied into the Codex plugin source/cache locations and
that current custom-agent TOMLs were mirrored globally. It does not prove that
the Codex app UI shows or enables the plugin.

The next app-install phase must produce a post-copy smoke receipt that verifies:

- `.codex-plugin/plugin.json` parses;
- `skills/` exists at the plugin root;
- `custom-agents/*.toml` parse and contain `name`, `description`, and
  `developer_instructions`;
- the personal marketplace JSON entry points at the copied plugin directory.

The current proof surface may include detached file-copy/cache/global-agent
sync receipts when they are freshly generated against the current package. Those
receipts support only local staging and active registry exposure. They do not
support install-button success, Plugins UI visibility, workspace/public
marketplace listing, or production readiness. The marketplace example in this
package is:

```text
install/personal-marketplace.example.json
```

## Custom Agent Install

Codex custom agents are TOML files. Copy only the packaged current TOML files
from:

```text
custom-agents/*.toml
```

to:

```text
~/.codex/agents/
```

New sessions can then spawn the named agents. Already-open sessions may need a
restart or reload before the new agent definitions appear.

The current material review team is the four falsifier agents named in
`docs/review-loop-record.md` and `templates/agent-standards/05-review-and-completion.md`.
Retired legacy reviewer TOMLs are historical context only and must not be
packaged under `custom-agents/` or installed as current review controls.

Disk files are not active registry proof. A material review round also needs a
`harness-ultragoal.multi-agent-registry-exposure.v1` receipt showing the
running Codex session can discover and spawn the current custom agent types.
See `docs/codex-custom-agent-registry-preflight.md`.

## Claim Ceiling

Copying files plus smoke checks proves local install staging only. Active
multi-agent registry exposure requires a separate live registry exposure
receipt. Neither surface proves public publishing, workspace sharing,
marketplace install-button success, Plugins UI visibility, or visibility inside
already-open Codex threads. Those require separate app-level confirmation.
