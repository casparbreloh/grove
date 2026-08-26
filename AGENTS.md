# Grove development guide

Read `VISION.md` before product or architecture work; it defines Grove vocabulary, product semantics, Agent/Environment boundaries, and delivery order. For renderer design-system or UI work, read `DESIGN.md`.

## principles

- Ship the smallest clear vertical slice. Avoid speculative providers, protocols, configuration, storage, or extension systems.
- Agent and Environment are Grove's only pluggable boundaries; use **adapter** only for an external protocol such as ACP. Git is optional and never defines Task or change semantics.
- Prefer platform-native APIs and add a dependency only when it materially reduces code or risk. The renderer projects Grove-owned durable state.

## boundaries

- `main` owns Electron, persistence, credentials, agents, environments, filesystem/process access, and validated IPC. `preload` exposes only a narrow typed Grove API. `renderer` is unprivileged: never import Node or Electron there.
- Keep serializable Grove contracts shared and integration types at their boundaries. Preserve context isolation, sandboxing, and no Node integration; validate renderer input in main and network data at its receiver.
- Keep credentials out of Task state, transfers, logs, and transcripts. Treat plugins, hooks, executable skills, and Agent extensions as trusted until isolation exists.

## implementation

- Follow local TypeScript/UI patterns, infer types, avoid `any`, validate untrusted data, and use discriminated unions for finite states. Mock only external/system boundaries.
- Do not edit generated files (including `routeTree.gen.ts`) or add Effect without a concrete lifecycle/concurrency need.
- Assistant UI belongs in `apps/desktop/src/renderer/src/components/ai-elements`. Install it only with `npx assistant-ui@latest add <component> --path src/renderer/src/components/ai-elements`; keep transitive shadcn primitives in `components/ui`.
- During the frontend-first phase, use explicitly named renderer mock modules; do not add preload, IPC, persistence, Agent, Environment, filesystem, process, or backend implementations.

## verification

Run the narrowest relevant check; the full check is `mise run check`. Stop only processes you started and can identify—never kill by name or pattern.
