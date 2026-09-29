# Task-owned computation sessions

Start one foreground owner before a task's first workspace check, verify, index or select. Keep its process and handle in the current task, reuse them, and close the owner before the task ends. `UG_AUTO_SESSION=0` is the narrow off switch. This owner runs the fixed pure Bend core; it is neither a global daemon nor a provider, permission or adoption authority.

Use a nonexistent path in a private task runtime parent outside the observed repository:

```sh
ultragoal session --root /absolute/repository --directory /absolute/task-runtime/owner
```

UltraGoal creates the owner directory as mode 0700 and prints a `session-ready` record with its handle. An existing directory is accepted only when it is empty and already owner-only. Keep the foreground process running in a task terminal, then pass its handle to later frontends:

```sh
ultragoal check --root /absolute/repository --contract /absolute/contract.tsv --no-cache --disclosure project --session /absolute/task-runtime/owner/session.json
```

The example's `project` class applies only when this contract's selected project evidence has existing disclosure authority. Otherwise omit `--disclosure`, use `--local` to keep semantic obligations unresolved, and report that limit. Workspace verify, index and select can use the same handle. Standalone named syntax verifiers use their native routes. Do not put the owner directory inside the observed repository. The owner removes only its own unchanged endpoint and handle at normal close. A long directory path uses a short socket name in the per-user runtime directory; the handle records it.

**Try the actual owner start.** A sandbox may refuse a local Unix socket; do not infer refusal from a config display. Some Codex workspace sandboxes need `sandbox_workspace_write.network_access = true`. If the start really fails, run the current command cold and name the lost reuse. Seek the host's allowed sandbox setting when available; never claim a cold repeat reused retained work. A refused endpoint yields `session_unavailable`.

Every frontend recaptures current source, membership and requirements before admission. The retained core compares typed current arguments and reuses only equal parse/rule work. Changed, missing or unknown inputs invalidate their dependents. Mutable disk caches remain reported comparisons. A local socket, PID or nonce is not protected provenance.

Ordinary content crosses bounded CHUNK requests and Bend folds complete partials. Larger logical files cross ordered source pages with path, version, offsets, EOF and SHA bound before one parser decision. Read planning and final check metadata also cross ordered pages, so a repository is not pruned to a single frame. Index reports write fact sets as cores answer. Current host headroom chooses capture batch size and retained traffic; transient pressure can pause a held descriptor and return a recoverable offset if it persists. Physical core frames and Jev provider requests remain bounded units, not total file or repository ceilings. A named incomplete result is never verified.

The owner normally lives 300 seconds and closes after 30 idle seconds. A named longer operation can set `--lifetime-seconds N` (30–7200) or `--idle-seconds N` (5–1800). One owner accepts at most 4096 core requests, then closes for a fresh task-owned owner; that lifecycle count is not a repository or source-size limit. Its core rotates between requests after ten evaluations while preserving the exact retained frame. Individual requests have their own stall deadline. Ordinary cores rotate before their 512 MiB cumulative traffic budget; retained page operations recheck live headroom instead. Provider calls and native verifiers never run inside this owner.

On expiry or interruption, verify the old handle and endpoint are gone, create a new private owner directory, reobserve current inputs, and retry the pending command once. If the host cancelled the owner, report cancellation rather than normal close. Before final delivery, close the foreground owner and verify its owned socket and handle are absent. A terminal check report, hash or journal is evidence of its own scope, not permission to replay effects or claim qualified whole-workspace freshness.
