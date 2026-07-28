#!/usr/bin/env bash
# shellcheck disable=SC2034 # Values are consumed through ${!variable}.
set -euo pipefail
umask 077
shell_quote() {
  printf '%q' "$1"
}
stat_owner_mode() {
  local path="$1"
  local owner mode
  case "$(uname -s)" in
    Darwin) owner="$(stat -f '%u' "$path")"; mode="$(stat -f '%Lp' "$path")" ;;
    *) owner="$(stat -c '%u' "$path")"; mode="$(stat -c '%a' "$path")" ;;
  esac
  printf '%s %s\n' "$owner" "$mode"
}
stat_identity() {
  local path="$1"
  case "$(uname -s)" in
    Darwin) stat -f '%d:%i' "$path" ;;
    *) stat -c '%d:%i' "$path" ;;
  esac
}
verify_private_directory() {
  local directory="$1"
  local owner mode
  [ -d "$directory" ] && [ ! -L "$directory" ] || return 1
  read -r owner mode < <(stat_owner_mode "$directory") || return 1
  [ "$owner" = "$(id -u)" ] && [ "$mode" = 700 ] || return 1
}
verify_private_file() {
  local file="$1"
  local expected_mode="$2"
  local owner mode
  [ -f "$file" ] && [ ! -L "$file" ] || return 1
  read -r owner mode < <(stat_owner_mode "$file") || return 1
  [ "$owner" = "$(id -u)" ] && [ "$mode" = "$expected_mode" ] || return 1
}
append_path_dir() {
  local directory="$1"
  [ -d "$directory" ] || return 0
  case ":$safe_path:" in
    *":$directory:"*) return 0 ;;
  esac
  safe_path="$safe_path:$directory"
}
append_assignments() {
  local prefix="$1" suffix="$2"; shift 2
  while [ "$#" -gt 0 ]; do
    printf '%s%s=%s%s\n' "$prefix" "$1" "$2" "$suffix"; shift 2
  done
}
worktree="${CODEX_WORKTREE_PATH:-$PWD}"
cd "$worktree"
root="$(pwd -P)"
state_root="$root/.codex-worktree"
scratch="$state_root/scratch"
state="$state_root/state"
home="$state_root/home"
tmp="$state_root/tmp"
cargo_target="$state_root/cargo-target"
cargo_cov_target="$state_root/cargo-llvm-cov-target"
cache="$state_root/cache"
data="$state_root/data"
cargo_home="$state_root/cargo-home"
rustup_home="$state_root/rustup-home"
if [ -L "$state_root" ] || { [ -e "$state_root" ] && [ ! -d "$state_root" ]; }; then
  printf 'refusing untrusted .codex-worktree state path\n' >&2
  exit 1
fi
[ -e "$state_root" ] || mkdir -m 700 "$state_root"
state_root_identity="$(stat_identity "$state_root")"
cd -P "$state_root"
[ "$(pwd -P)" = "$state_root" ] && [ "$(stat_identity .)" = "$state_root_identity" ] || {
  printf 'refusing replaced .codex-worktree state directory\n' >&2
  exit 1
}
chmod 700 .
verify_private_directory . || {
  printf 'refusing untrusted .codex-worktree state directory\n' >&2
  exit 1
}
for directory in scratch state home tmp cargo-target cargo-llvm-cov-target cache data cargo-home rustup-home; do
  if [ -L "$directory" ] || { [ -e "$directory" ] && [ ! -d "$directory" ]; }; then
    printf 'refusing untrusted worktree environment path\n' >&2
    exit 1
  fi
  if [ ! -e "$directory" ]; then
    mkdir -m 700 "$directory"
  fi
  directory_identity="$(stat_identity "$directory")"
  (
    cd -P "$directory"
    [ "$(stat_identity .)" = "$directory_identity" ] || exit 1
    chmod 700 .
    verify_private_directory .
  ) || {
    printf 'refusing untrusted worktree environment directory\n' >&2
    exit 1
  }
done
runner_file="./run-command"
env_file="./env.sh"
for generated in "$env_file" "$runner_file"; do
  if [ -L "$generated" ]; then
    printf 'refusing symlinked generated environment file\n' >&2
    exit 1
  fi
  expected_mode=600
  [ "$generated" = "$runner_file" ] && expected_mode=700
  if [ -e "$generated" ] && ! verify_private_file "$generated" "$expected_mode"; then
    printf 'refusing untrusted generated environment file\n' >&2
    exit 1
  fi
done
rm -f "$runner_file"
if command -v shasum >/dev/null 2>&1; then
  hash="$(printf '%s' "$root" | shasum -a 256 | awk '{print $1}')"
elif command -v sha256sum >/dev/null 2>&1; then
  hash="$(printf '%s' "$root" | sha256sum | awk '{print $1}')"
else
  hash="$(printf '%s' "$root" | cksum | awk '{print $1}')"
fi
worktree_id="${hash:0:16}"
port=$((20000 + (16#${hash:0:4} % 20000)))
env_binary="$(type -P env 2>/dev/null || true)"
bash_binary="$(type -P bash 2>/dev/null || true)"
python_binary="$(type -P python3 2>/dev/null || true)"
account_home="$(cd ~ && pwd -P)"
host_cargo_binary="${CODEX_HOST_CARGO_BIN:-}"
[ "$host_cargo_binary" = "$account_home/.cargo/bin/cargo" ] || exit 1
[ -x "$host_cargo_binary" ] && [ ! -d "$host_cargo_binary" ] || exit 1
read -r host_owner host_mode < <(stat_owner_mode "$host_cargo_binary")
[ "$host_owner" = "$(id -u)" ] && [ $((8#$host_mode & 022)) -eq 0 ] || exit 1
cargo_binary=""
for candidate in "$account_home/.rustup"/toolchains/*/bin/cargo; do
  if [ -x "$candidate" ]; then cargo_binary="$candidate"; break; fi
done
[ -x "$cargo_binary" ] || { printf 'direct host cargo toolchain not found\n' >&2; exit 1; }
cargo_cov_binary="$(type -P cargo-llvm-cov 2>/dev/null || true)"
if [ -z "$env_binary" ] || [ -z "$bash_binary" ]; then printf 'worktree environment requires env and bash\n' >&2; exit 1; fi
if [ -x "$python_binary" ]; then
  python_real="$("$python_binary" -c 'import sys; print(sys.executable)' 2>/dev/null || true)"
  if [ -x "$python_real" ]; then
    python_binary="$python_real"
  fi
fi
safe_path="/usr/local/bin:/opt/homebrew/bin:/opt/homebrew/sbin:/usr/bin:/bin:/usr/sbin:/sbin"
for binary in "$env_binary" "$bash_binary" "$python_binary" "$cargo_binary" "$cargo_cov_binary"; do
  if [ -n "$binary" ]; then
    append_path_dir "$(dirname "$binary")"
  fi
done
declare root_q worktree_id_q scratch_q state_q home_q tmp_q port_q cargo_target_q
declare cargo_cov_target_q cache_q data_q cargo_home_q rustup_home_q safe_path_q
for variable in root worktree_id scratch state home tmp port cargo_target cargo_cov_target cache data cargo_home rustup_home safe_path; do
  printf -v "${variable}_q" '%q' "${!variable}"
done
common_environment=(
  CODEX_WORKTREE_ROOT "$root_q" CODEX_WORKTREE_ID "$worktree_id_q"
  CODEX_WORKTREE_SCRATCH "$scratch_q" CODEX_WORKTREE_STATE "$state_q" CODEX_WORKTREE_HOME "$home_q"
  CODEX_WORKTREE_TMP "$tmp_q" CODEX_WORKTREE_PORT "$port_q" TMPDIR "$tmp_q" TMP "$tmp_q"
  TEMP "$tmp_q" HOST 127.0.0.1
  CARGO_TARGET_DIR "$cargo_target_q" CARGO_LLVM_COV_TARGET_DIR "$cargo_cov_target_q"
)
env_tmp=""; runner_tmp=""; manifest_q=""; receipt_q=""
cleanup_temps() { [ -z "$env_tmp" ] || rm -f "$env_tmp"; [ -z "$runner_tmp" ] || rm -f "$runner_tmp"; }
trap cleanup_temps EXIT
if [ -f "$root/plugin-manifest-draft.json" ] && [ -f "$root/validator/Cargo.toml" ]; then
  manifest_q="$(shell_quote "$root/plugin-manifest-draft.json")"
  receipt_q="$(shell_quote "$root/validation_artifacts/ultragoal-audit/validator-receipt.json")"
fi
if [ ! -e "$env_file" ]; then
  env_tmp="$(mktemp "./.env.sh.XXXXXX")"
  printf '%s\n' \
    '# Generated by .codex/setup-worktree-env.sh. Do not edit.' > "$env_tmp"
  append_assignments 'export ' '' "${common_environment[@]}" >> "$env_tmp"
  if [ -f "$root/plugin-manifest-draft.json" ] && [ -f "$root/validator/Cargo.toml" ]; then
    cat >> "$env_tmp" <<EOF
export HARNESS_ULTRAGOAL_PLUGIN_REPO="1"
export HARNESS_ULTRAGOAL_VALIDATOR_MANIFEST=$manifest_q
export HARNESS_ULTRAGOAL_VALIDATOR_RECEIPT=$receipt_q
EOF
  fi
  chmod 600 "$env_tmp"
  env_identity="$(stat_identity "$env_tmp")"
  mv -f "$env_tmp" "$env_file"
  env_tmp=""
  [ "$(stat_identity "$env_file")" = "$env_identity" ] || {
    printf 'generated environment file identity changed during publication\n' >&2
    exit 1
  }
fi
env_identity="$(stat_identity "$env_file")"
env_q="$(shell_quote "$env_binary")"
runner_tmp="$(mktemp "./.run-command.XXXXXX")"
cat > "$runner_tmp" <<EOF
#!$bash_binary
set -euo pipefail
if [ "\$#" -eq 0 ]; then
  printf 'usage: %s command [args...]\\n' "\${0##*/}" >&2
  exit 64
fi
exec $env_q -i \\
EOF
append_assignments '  ' ' \' "${common_environment[@]}" >> "$runner_tmp"
runner_environment=(
  HOME "$home_q" XDG_CONFIG_HOME "$home_q/.config"
  XDG_CACHE_HOME "$cache_q" XDG_DATA_HOME "$data_q" XDG_STATE_HOME "$state_q" CARGO_HOME "$cargo_home_q"
  RUSTUP_HOME "$rustup_home_q" CARGO_NET_OFFLINE true
)
append_assignments '  ' ' \' "${runner_environment[@]}" >> "$runner_tmp"
if [ -f "$root/plugin-manifest-draft.json" ] && [ -f "$root/validator/Cargo.toml" ]; then
  cat >> "$runner_tmp" <<EOF
  HARNESS_ULTRAGOAL_PLUGIN_REPO=1 \\
  HARNESS_ULTRAGOAL_VALIDATOR_MANIFEST=$manifest_q \\
  HARNESS_ULTRAGOAL_VALIDATOR_RECEIPT=$receipt_q \\
EOF
fi
cat >> "$runner_tmp" <<EOF
  PATH=$safe_path_q \\
  LANG=C \\
  LC_ALL=C \\
  $env_q "\$@"
EOF
chmod 700 "$runner_tmp"
runner_identity="$(stat_identity "$runner_tmp")"
mv -f "$runner_tmp" "$runner_file"
runner_tmp=""
[ "$(stat_identity "$runner_file")" = "$runner_identity" ] || {
  printf 'generated runner identity changed during publication\n' >&2
  exit 1
}
verify_private_file "$runner_file" 700 || {
  printf 'generated runner failed ownership or mode validation\n' >&2
  exit 1
}
final_state_identity="$(stat_identity "$state_root" 2>/dev/null || true)"
[ ! -L "$state_root" ] && [ "$final_state_identity" = "$state_root_identity" ] || {
  printf 'worktree state directory identity changed during setup\n' >&2
  exit 1
}
[ "$(stat_identity "$env_file")" = "$env_identity" ] &&
  [ "$(stat_identity "$runner_file")" = "$runner_identity" ] &&
  verify_private_file "$env_file" 600 &&
  verify_private_file "$runner_file" 700 || {
  printf 'generated worktree environment files changed during setup\n' >&2
  exit 1
}
printf 'Codex worktree environment ready: %s (runner: %s)\n' "$state_root/env.sh" "$state_root/run-command"
