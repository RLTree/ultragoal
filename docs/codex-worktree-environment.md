# Codex Worktree Environment

This repo manages Codex App worktree setup through
`.codex/environments/environment.toml`. The config is part of the repo
contract; generated runtime state is not.

## Contract

- Codex setup must start from `${CODEX_WORKTREE_PATH:-$PWD}`.
- Environment actions use `script = '''...'''`; `command` is not the Codex App
  environment schema.
- `platform = "..."` is optional and should be present only when an action is
  platform-specific.
- No setup or cleanup script may hardcode a specific worktree path.
- Per-worktree generated state belongs under `.codex-worktree/`.
- Agents should source `.codex-worktree/env.sh` before validation commands.
- Repo-specific environment variables are allowed only after detecting the repo
  shape, not by matching a local absolute path.

## Generated State

`.codex/setup-worktree-env.sh` creates:

```text
.codex-worktree/env.sh
.codex-worktree/scratch/
.codex-worktree/state/
.codex-worktree/home/
.codex-worktree/tmp/
.codex-worktree/cargo-target/
.codex-worktree/cargo-llvm-cov-target/
```

The generated `env.sh` exports isolated `CODEX_WORKTREE_*`, `TMPDIR`, `TMP`,
`TEMP`, `HOST`, `CARGO_TARGET_DIR`, and `CARGO_LLVM_COV_TARGET_DIR` values.

## Global Mirror

Codex App setup invokes `~/.codex/bin/codex-worktree-env`. The repo-local
script is the source of truth and may be mirrored there when installing or
updating the harness.

## Validation

Before relying on this setup:

```bash
bash -n .codex/setup-worktree-env.sh
python3 - <<'PY'
import tomllib
tomllib.load(open(".codex/environments/environment.toml", "rb"))
PY
bash "$HOME/.codex/bin/codex-worktree-env"
. .codex-worktree/env.sh
```
