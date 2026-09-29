# UltraGoal structural analyzer dependency

This directory is a project-owned copy of the standalone Rust legibility engine
from Tree's retired Engineering Judgment source tree. UltraGoal uses its raw AST,
dependency and authority inventory through `engines/ultragoal-next/check_ownership.py`.
It does not install, enable, or depend on the EJ agent/plugin.

`UPSTREAM_PROVENANCE.json` records the selected source, exact file and lock hashes,
and the qualified macOS ARM64 binary/compiler identity. The provenance digest is
bound in the UltraGoal checker. The checker rejects missing, changed, symlinked,
or differently built inputs; it builds a missing binary offline with Cargo and
checks its qualified digest. `scripts/check-next` also runs the analyzer's own
116 tests before UltraGoal's dependent tests. Other targets require their own
qualified binary record, not an unchecked fallback.

Run `bash scripts/check-next --quick "$PWD"` from the repository root. The raw
analyzer's 250-line and unsupported-language findings remain separate from the
project's reviewed Rust effect/dependency ownership gate; Bend/Python meaning
uses their native proof and runtime checks. Neither result grants permission
for an effect or establishes a universal semantic pass.
