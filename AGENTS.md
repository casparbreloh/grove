# Grove development guide

## Priorities

Ship the smallest clear change. This is a fast-moving product, not an enterprise system: handle real failure modes, but do not add speculative abstractions, configuration, flags, or edge-case machinery.

Prefer native Electron and platform APIs when they make the product simpler. Extract a module only for a real boundary or a second caller.

## Architecture

- `apps/desktop/src/main` owns Electron, filesystem access, process spawning, persistence, credentials, agent adapters, and IPC handlers.
- `apps/desktop/src/preload` exposes a narrow, typed API to the renderer. Never expose generic IPC or Node/Electron APIs.
- `apps/desktop/src/renderer` is unprivileged UI. It must not import Node or Electron APIs.
- Keep provider-specific types inside their adapter. Grove consumes normalized sessions, events, tool calls, approvals, errors, and cancellation.
- Preserve Electron's security defaults: context isolation, sandboxing, and no Node integration. Validate IPC inputs in main.
- Treat plugins as trusted local executable code unless a real isolation model exists. Do not add plugin discovery or installation casually.

## Implementation

- Follow nearby TypeScript and UI patterns. Prefer inferred types; do not use `any`.
- Keep contracts and UI states symmetric: start/stop, enable/disable, approve/deny, loading/success/failure.
- Use native dialogs, menus, notifications, and OS storage where appropriate.
- Do not log secrets, credentials, or full private transcripts.
- Do not add Effect yet. Reconsider it only when concrete main-process lifecycle, cancellation, resource-scoping, or concurrency complexity warrants it; keep it out of renderer state.

## Verification

Run the narrowest relevant check after a change. The normal full check is:

```sh
mise run check
```

Do not edit generated files such as `routeTree.gen.ts`. Never kill processes by name or pattern; only stop processes you started and can identify.
