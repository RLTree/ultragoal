# DESIGN

Design work serves the product or workflow this repo exists to support. The
goal is not decoration; the goal is a coherent, usable surface that helps users
understand state, take action, recover from errors, and trust the system.

Start with `ARCHITECTURE.md`, then use `docs/design-docs/index.md` for design
records and `PRODUCT_SENSE.md` for the user/job/acceptance framing.

## Required Proof For Design Or UI Changes

- Run the repo documentation/discovery check.
- Run the architecture check when UI files, routes, or module boundaries move.
- Run the relevant frontend, API, or product tests named in `FRONTEND.md`.
- Capture browser-visible or app-visible proof for UI changes: screenshot,
  trace, accessibility output, or an explicit blocker.
- Record the proof path in the active ExecPlan or final receipt.

## Design Rules

- Use existing tokens, layout primitives, and component conventions before
  inventing new styling.
- Every interactive state needs a reachable state or an explicit non-goal.
- Accessibility proof is part of the work: labels, focus, keyboard, reduced
  motion, and high-contrast/forced-color states must be checked when touched.
- Do not bury product behavior in decorative cards or unsupported copy.
- Operational products should prioritize scanability, provenance, status,
  reversible actions, and low cognitive load.
