# Portable EJ agent legibility checker

This crate adapts the recovered UltraGoal source laws into a portable Rust-project
check. EJ calls its library directly. It is not the historical UltraGoal executable,
and the EJ observation engine alone is not enforcement.

## Run

Integrated plugin:

```sh
ej check --root /absolute/project --registry docs/legibility/registry.json --inventory
```

Standalone crate:

```sh
cargo run --locked --offline --bin plugin-eval-legibility -- --root /absolute/project --registry docs/legibility/registry.json --inventory
```

The standalone command exits 0 only for a passing audit, 1 for findings and 2 for
invalid CLI arguments. The integrated CLI owns its error-envelope/exit convention.
Both use the same `plugin_eval_legibility::run(&Path, &str, bool) -> serde_json::Value`
implementation. The standalone audit schema is `engineering-judgment.legibility.v1`;
the integrated EJ command identifies its result as `ej.legibility.v1`. Inspect
`passed`, `failures`, `limitations` and `exclusions`. The optional inventory is
diagnostic evidence, not permission to approve discovered authority.

## First project setup

1. Run with `--inventory`. Missing registry deliberately fails with
   `registry_setup_required`; the command does not generate approving records.
2. Author `docs/legibility/registry.json`, schema version 1, listing exact relative
   source-map, dependency, command, boundary and output registry files.
3. Give every discovered file an exact source row: `path`, `class`, `owner`,
   `purpose`. Owners must resolve inside the same inventory. Source-map omission,
   duplicate rows, wildcard paths, traversal and symlinks fail.
4. Declare actual Cargo direct-dependency versions, owners, upstream documentation,
   purpose and exact adapter profiles. Include timeout/retry applicability and
   cache-invalidating inputs. Profiles cannot approve unrelated call sites.
5. Register real privileged operations with closed request/response/error ownership
   and validation symbols. Output records identify actual producers, validators
   and tests. Adding a nominal type or assertion to a registry cannot make an
   open JSON payload structurally closed.
6. Resolve findings and rerun. Keep the check in the project's required build gate.

The six-file `fixtures/receiving` project is a deliberately small complete worked
example: one pure Rust function, no dependencies or privileged operations, and
three manually authored registry files. Empty authority/dependency lists are
correct only because its source has neither. Do not copy them onto a real project
and call that approval.

```sh
ej check --root /absolute/plugin/engines/legibility/fixtures/receiving --inventory
```

## Enforced scope

- Discover every file recursively from the actual project root; no required
  evaluator directory layout or historical-document exceptions.
- Prune only real directories named `.git`, `target`, `node_modules`, or
  `__pycache__`, and report them. A symlink with any such name fails.
- Discover every Cargo manifest. Read real Cargo metadata with
  `--locked --offline`; inspect local packages, workspace members and targets.
  Local packages or targets outside the project or hidden beneath a pruned
  directory fail. Virtual workspaces use their actual workspace lock.
- Enforce 250 physical authored lines, including comments, tests, examples,
  guidance and registry records; verified Cargo-generated locks are exempt.
- Check Rust module ownership/reachability, partial factoring and repeated
  namespace prefixes; inspect cfg applicability and exact declared effects,
  outputs and dependency ownership.
- Tests retain the recovered production-boundary exemption when Cargo/module
  ownership actually establishes test-only applicability. Tests still require
  source ownership, parsable syntax, topology and the physical line cap.
- Examples and fixture directories are **not** blanket exemptions. Fixture data
  currently has no line/encoding bypass. Large/binary assets therefore block this
  version and need an explicit reviewed artifact-policy extension, not a rename.

## Prerequisites and limits

Cargo must be on PATH. The checked project needs a valid generated Cargo lock and
dependencies already available to offline Cargo metadata. Metadata does not build
or run the project's build script or tests. A missing cache or stale lock is a
failed prerequisite, never a passing audit.

Rust semantic analysis is conservative syntax analysis. It does not prove complete
compiler type resolution, arbitrary macro expansion, dynamic method-call behavior,
validation quality or architectural quality. Conventional `src` module layouts
and workspace package prefixes are supported; unconventional roots may fail
unresolved ownership rather than being silently authorized. Unsupported syntax and
unresolved paths retain their actual diagnostics/limitations.

Recognized non-Rust executable extensions, executable shebang files, script source
classes, and every HTML/HTM file fail with `semantic_coverage_unsupported`.
There is no HTML parser: event attributes, JavaScript URLs and other executable
HTML features cannot be safely ruled out by scanning for script tags. Even plain
HTML therefore requires a semantic adapter before this gate can approve it.
Other-language code cannot receive a semantic green by merely declaring ownership.
This checker has no Python/JavaScript/TypeScript semantic adapter. Plain document,
configuration and styling files receive inventory/ownership/line checks only.
File extensions and source classification remain part of the explicit project
contract; this is not a universal content-language detector.

## Verification

```sh
cargo test --locked --offline --all-targets
cargo clippy --locked --offline --all-targets -- -D warnings
```

The receiving controls exercise a clean pass, omitted roots, examples and tests,
unsupported claimed Python, absent/outside registry paths, symlinks, regular files
named like build directories, the physical cap, virtual-workspace members,
workspace members hidden in pruned directories and outside local dependencies.
HTML event handlers, JavaScript URLs, plain HTML and extensionless shebangs have
negative controls; plain CSS/documents and the Rust receiving project still pass.
The original 103 enforcement tests remain alongside these portable controls.
