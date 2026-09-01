# Grove development guide

## Where to look

- `VISION.md` is the authority for product vocabulary, semantics, Agent/Environment boundaries, and delivery order. Read it before product, runtime, or architecture work.
- `DESIGN.md` is the authority for renderer visual intent and UI principles. Renderer CSS and existing components are the source of truth for tokens, dimensions, theme values, and component implementation.
- `.repos/` contains ignored, read-only inspiration checkouts. Before relying on one, run `git -C <path> pull --ff-only`.
  - `.repos/opencode-dev` — current OpenCode V2 development on `dev`; inspect its shared core/server/client and multi-frontend shape.
  - `.repos/pi-dev` — current Pi development on `dev`; use this as the primary source for active harness, protocol, server, client, and coding-agent work.
  - `.repos/pi-main` — Pi's released `main` baseline; use it only for release comparisons.

Inspiration is not authority. Preserve Grove product semantics even when an upstream shape is useful.

## Principles

- Ship the smallest clear vertical slice. Avoid speculative providers, protocols, configuration, storage, or extension systems.
- Agent and Environment are Grove's only pluggable boundaries; use **adapter** only for an external protocol such as ACP. Git is optional and never defines Task or change semantics.
- Pi's session, runtime, protocol, and extension contracts are Grove's Agent primitives. Extend them; do not create parallel Grove contracts for prompts, messages, tools, models, compaction, session trees, or the TUI. A non-Pi Agent adapts to the Pi-shaped seam.
- Prefer platform-native APIs and add a dependency only when it materially reduces code or risk. The renderer projects Grove-owned durable state.

## Boundaries

- `main` owns Electron, persistence, credentials, agents, environments, filesystem/process access, and validated IPC. `preload` exposes only a narrow typed Grove API. `renderer` is unprivileged: never import Node or Electron there.
- Keep Grove contracts limited to Grove product state such as Workspaces, Projects, Tasks, Environments, revisions, and multiplayer. Preserve context isolation, sandboxing, and no Node integration; validate renderer input in main and network data at its receiver.
- Keep credentials out of Task state, transfers, logs, and transcripts. Treat plugins, hooks, executable skills, and Agent extensions as trusted until isolation exists.
- Agent execution uses Pi's runtime directly. Local and development-machine Environments share native-directory semantics, while Cloudflare is a separate implementation of the same Environment interface.

## Implementation

- Follow local TypeScript/UI patterns, infer types, avoid `any`, validate untrusted data, and use discriminated unions for finite states. Mock only external/system boundaries.
- Do not edit generated files (including `routeTree.gen.ts`).
- Before creating renderer UI, inspect the nearest existing component and the semantic tokens in `theme.css`; reuse their variants, state selectors, accessibility behavior, and sizing.
- For shadcn updates, inspect `npx shadcn@latest diff --help`, then run `npx shadcn@latest add <component> --diff -y` from `apps/desktop`. Classify each difference before applying it; never overwrite a Grove deviation blindly.
- Assistant UI belongs in `apps/desktop/src/renderer/src/components/ai-elements`. Install it only with `npx assistant-ui@latest add <component> --path src/renderer/src/components/ai-elements`; keep transitive shadcn primitives in `components/ui`.
- Renderer-only slices use explicitly named mock modules. Do not smuggle preload, IPC, filesystem, process, Agent, or Environment behavior into renderer code.

## Verification

Run the narrowest relevant check; the full check is `mise run check`. Stop only processes you started and can identify—never kill by name or pattern.
