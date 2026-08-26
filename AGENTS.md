# Grove development guide

Read `VISION.md` before product or architecture work. It is the source of truth for vocabulary, product semantics, Agent/Environment boundaries, and delivery order; do not duplicate or redefine them here.

For renderer design-system or UI work, read `DESIGN.md`.

## Working principles

- Ship the smallest clear vertical slice. Do not prebuild speculative providers, protocols, storage backends, configuration, or extension systems.
- Preserve Agent and Environment as Grove's only pluggable product boundaries. Use **adapter** only for translating an external protocol such as ACP.
- Keep Git optional and subordinate. Do not make Task or change semantics depend on branches, worktrees, commits, or staged versus unstaged state.
- Prefer Electron, operating-system, and platform-native APIs. Add a dependency only when it materially reduces code or risk and has a credible maintenance path.
- Keep the renderer a projection of Grove-owned durable state.

## Repository boundaries

- `apps/desktop/src/main` owns Electron, persistence, credentials, agents, environments, filesystem/process access, and validated IPC handlers.
- `apps/desktop/src/preload` exposes a narrow typed Grove API. Never expose generic IPC, RPC, Electron, Node, shell, SSH, or filesystem access.
- `apps/desktop/src/renderer` is unprivileged and must not import Node or Electron APIs.
- Put serializable Grove contracts in shared modules. Keep Pi, ACP, assistant-ui, Electron, transport, and platform-native types at their boundaries.
- Preserve context isolation, renderer sandboxing, and no Node integration. Validate every renderer input in main and every network payload at its receiver.

## Implementation

- Follow nearby TypeScript and UI patterns. Prefer inferred types; do not use `any`.
- Use runtime validation at untrusted boundaries and discriminated unions for finite states.
- Inject dependencies; mock only external APIs, time, randomness, and system boundaries.
- Keep credentials out of Task state, transfers, logs, and transcripts.
- Treat plugins, hooks, executable skills, and Agent extensions as trusted code until a real isolation model exists.
- Do not add Effect yet. Reconsider only for concrete lifecycle, cancellation, resource-scoping, or concurrency complexity, and keep it out of renderer state.
- Do not edit generated files such as `routeTree.gen.ts`.
- Keep Assistant UI components in `apps/desktop/src/renderer/src/components/ai-elements`. Do not configure its registry in `components.json`; from `apps/desktop`, install explicitly with `npx assistant-ui@latest add <component> --path src/renderer/src/components/ai-elements`, and review transitive shadcn primitives so they remain in `components/ui`.
- During the frontend-first phase, keep product data and behavior in explicitly named renderer mock modules. Do not add preload, IPC, persistence, Agent, Environment, filesystem, process, or backend service implementations.

## Verification

Run the narrowest relevant check. The normal full check is:

```sh
mise run check
```

Never kill processes by name or pattern; stop only processes you started and can identify.
