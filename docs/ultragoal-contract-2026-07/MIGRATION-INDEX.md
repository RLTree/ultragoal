# Harness Ultragoal Contract Split Migration Index

This file records how the legacy parent-session binding files were split into
the modular contract collection.

The split is a builder-contract migration only. It is not package evidence, not
Gate 92 proof, not Product Usage proof, not Phase 4 evidence, not final-packet
proof, and not `update_goal` evidence.

## Legacy Prompt Mapping

Legacy file:
`docs/parent-session-full-ultragoal-compliance-prompt-2026-06-25.md`

- Lines 1-1682 -> `01-prompt-product-doctrine-and-base-contract.md`
- Lines 1683-3327 -> `02-prompt-gates-089-091-cli-rust.md`
- Lines 3328-3522 -> `03-prompt-gate-092-observability-fast-loop.md`
- Lines 3523-4071 -> `04-prompt-gates-093-105-research-product-evolution.md`
- Lines 4072-4281 -> `05-prompt-update-goal-stop-conditions.md`

## Legacy Spine Mapping

Legacy file:
`docs/parent-session-full-ultragoal-execution-spine-2026-06-30.md`

- Lines 1-383 -> `10-spine-operating-rules-and-doctrine.md`
- Lines 384-705 -> `11-spine-phase-order.md`
- Lines 706-772 -> `12-spine-lane-forbidden-parallel-rules.md`

## Legacy Checklist Mapping

Legacy file:
`docs/parent-session-full-ultragoal-compliance-checklist-2026-06-25.md`

- Lines 1-825 -> `checklist/20-checklist-context-ledger-and-gates-000-088.md`
- Lines 826-1522 -> `checklist/21-checklist-gate-089-cli-control-plane.md`
- Lines 1523-2359 -> `checklist/22-checklist-gates-090-091-namespace-rust-gc.md`
- Lines 2360-2659 -> `checklist/23-checklist-gate-092-observability-fast-loop.md`
- Lines 2660-3078 -> `checklist/24-checklist-gates-093-105-research-product-evolution.md`
- Lines 3079-3592 -> `checklist/25-checklist-validation-update-goal-final-stop.md`

## Parity Checks

Run before replacing or validating compatibility shims:

```bash
cmp -s <(cat \
  docs/ultragoal-contract-2026-07/01-prompt-product-doctrine-and-base-contract.md \
  docs/ultragoal-contract-2026-07/02-prompt-gates-089-091-cli-rust.md \
  docs/ultragoal-contract-2026-07/03-prompt-gate-092-observability-fast-loop.md \
  docs/ultragoal-contract-2026-07/04-prompt-gates-093-105-research-product-evolution.md \
  docs/ultragoal-contract-2026-07/05-prompt-update-goal-stop-conditions.md) \
  docs/parent-session-full-ultragoal-compliance-prompt-2026-06-25.md

cmp -s <(cat \
  docs/ultragoal-contract-2026-07/10-spine-operating-rules-and-doctrine.md \
  docs/ultragoal-contract-2026-07/11-spine-phase-order.md \
  docs/ultragoal-contract-2026-07/12-spine-lane-forbidden-parallel-rules.md) \
  docs/parent-session-full-ultragoal-execution-spine-2026-06-30.md

cmp -s <(cat \
  docs/ultragoal-contract-2026-07/checklist/20-checklist-context-ledger-and-gates-000-088.md \
  docs/ultragoal-contract-2026-07/checklist/21-checklist-gate-089-cli-control-plane.md \
  docs/ultragoal-contract-2026-07/checklist/22-checklist-gates-090-091-namespace-rust-gc.md \
  docs/ultragoal-contract-2026-07/checklist/23-checklist-gate-092-observability-fast-loop.md \
  docs/ultragoal-contract-2026-07/checklist/24-checklist-gates-093-105-research-product-evolution.md \
  docs/ultragoal-contract-2026-07/checklist/25-checklist-validation-update-goal-final-stop.md) \
  docs/parent-session-full-ultragoal-compliance-checklist-2026-06-25.md
```

All three comparisons returned exit code 0 before the legacy files were turned
into compatibility shims.

## Compatibility Shim Rule

After this split, the three legacy files must route agents to
`docs/ultragoal-contract-2026-07/README.md`. They must not retain unique hidden
authority. If any external process still names the old paths as binding files,
the shim preserves that entrypoint while making the modular contract root the
actual loading surface.

## Residual Follow-Up

Future hardening should add a validator or lightweight docs check that:

- verifies the shim files point to the root;
- verifies every module named by `README.md` exists;
- verifies the migration index names every module;
- verifies no checklist module is used as a receipt ledger; and
- verifies parent-session builder-contract files remain outside package digest
  and package evidence surfaces.
