# FRONTEND

Use this file to route frontend-adjacent work. If the repo has no browser,
desktop, mobile, or visual control surface, say that clearly here and keep this
file as the future routing point.

## Frontend Surface

Name the frontend entrypoints, routes, asset roots, server commands, and local
URLs. State whether the surface is browser, desktop, mobile, CLI-TUI, or
embedded.

## Proof Spine

Replace these with the repo's actual commands:

- Documentation/discovery check.
- Architecture or dependency-boundary check.
- Frontend unit/component tests.
- API/contract tests that feed the UI.
- Browser or app interaction proof for pixels, layout, accessibility, and
  state changes.

## Expectations

- Keep local servers loopback-only unless a documented product requirement and
  approval says otherwise.
- Treat browser input, saved user data, model/tool output, local files, and API
  payloads as untrusted until parsed.
- Use real UI state evidence for UI claims. CLI proof does not prove UI
  behavior.
- Document visual proof artifacts in the active ExecPlan, product receipt, or
  Product Cohesion receipt.
