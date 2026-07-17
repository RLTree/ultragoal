# Codex Worktree Environment

This repo manages Codex App worktree setup through
`.codex/environments/environment.toml`. The config is part of the repo
contract; generated runtime state is not. Lane commands run through the
generated `.codex-worktree/run-command` wrapper, never by sourcing the
generated shell projection.

## Contract

- Codex setup must start from `${CODEX_WORKTREE_PATH:-$PWD}` and invoke the
  repo-local `.codex/setup-worktree-env.sh`.
- Environment actions use `script = '''...'''`; `command` is not the Codex App
  environment schema.
- `platform = "..."` is optional and should be present only when an action is
  platform-specific.
- No setup or cleanup script may hardcode a specific worktree path.
- Per-worktree generated state belongs under `.codex-worktree/`.
- App actions and lane commands must invoke
  `.codex-worktree/run-command <command>`. The wrapper uses `env -i` and an
  explicit allowlist of typed worktree paths, cache/target roots, the loopback
  host, locale, and resolved tool directories. It does not inherit arbitrary
  host environment variables, including `OPENAI_API_KEY`.
- The app bootstrap itself invokes the repo setup script through an absolute
  `/usr/bin/env -i` boundary containing only worktree paths, fixed `PATH`, and
  the non-secret `CODEX_HOST_CARGO_BIN="$HOME/.cargo/bin/cargo"` hint; a
  checked-out setup script never receives the parent secret-bearing
  environment. Setup validates that hint and resolves a direct host toolchain
  binary before generating the runner, without forwarding host Cargo or rustup
  state.
- `.codex-worktree/env.sh` is an ignored compatibility projection for explicit,
  separately governed key-resolution flows only. Normal execution must not
  source, package, or print it; the wrapper strips inherited secrets but cannot
  prevent a caller from explicitly passing a secret-bearing argument or
  command. The known app actions do neither.
- Repo-specific environment variables are allowed only after detecting the repo
  shape, not by matching a local absolute path.

## Generated State

`.codex/setup-worktree-env.sh` creates:

```text
.codex-worktree/env.sh
.codex-worktree/run-command
.codex-worktree/scratch/
.codex-worktree/state/
.codex-worktree/home/
.codex-worktree/tmp/
.codex-worktree/cargo-target/
.codex-worktree/cargo-llvm-cov-target/
.codex-worktree/cache/
.codex-worktree/data/
.codex-worktree/cargo-home/
.codex-worktree/rustup-home/
```

The generated `env.sh` exports isolated `CODEX_WORKTREE_*`, `TMPDIR`, `TMP`,
`TEMP`, `HOST`, `CARGO_TARGET_DIR`, and `CARGO_LLVM_COV_TARGET_DIR` values as a
compatibility projection. The generated `run-command` wrapper carries the
same typed values directly and starts child processes from an empty
environment; it does not source `env.sh`. `HOME`, XDG directories, Cargo
caches, toolchain state, and Cargo target/state roots remain worktree-local.
The runner resolves a direct host toolchain binary only to avoid rustup shims,
while `CARGO_HOME` and `RUSTUP_HOME` point at created private worktree roots
with `CARGO_NET_OFFLINE=true`; no host Cargo config or credentials path is
forwarded.

## Global Mirror

The repo-local setup script is the source of truth. A global
`~/.codex/bin/codex-worktree-env` may be mirrored for host tooling, but it is
not used by the checked-in app actions and must not be a fallback that sources
`env.sh`.

## Validation

Before relying on this setup:

```bash
bash -n .codex/setup-worktree-env.sh
python3 - <<'PY'
import tomllib
tomllib.load(open(".codex/environments/environment.toml", "rb"))
PY
bash .codex/setup-worktree-env.sh
OPENAI_API_KEY=sentinel .codex-worktree/run-command sh -c \
  'test -z "${OPENAI_API_KEY:-}" && env | sort'
```

The setup rejects symlinked, special, foreign-owned, or world-writable state
paths and publishes generated files through private temporary files and atomic
renames. These checks prove inherited-environment execution sanitation only. They do not revoke an
external provider credential or reconcile a historical plaintext-key claim;
those remain owner-authorized external actions with their own evidence and
claim ceiling.
