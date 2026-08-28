# Grove runtime primitives

Status: foundation for the first Pi-native vertical slice. `VISION.md` remains authoritative for product semantics and delivery order.

## Recommendation

Use Pi Coding Agent as Grove's coding-agent implementation, not merely as a model loop and not as inspiration for a parallel harness. Local and development-machine hosts should construct Pi through its official TypeScript SDK and typed extensions. Pi keeps responsibility for its loop, model catalog, OAuth subscriptions, messages, tools, compaction, conversation tree, and TUI.

Grove wraps Pi with one deep client-facing module for Grove-owned product state. That module projects Pi Sessions into durable Tasks, Environment state, revisions, commands, events, and renderer-safe snapshots. It does not expose Pi objects to a desktop, mobile, or network client.

There are three Environment kinds but initially two implementation families:

- **local** uses Pi Coding Agent and ordinary native directories on the current computer;
- **development machine** runs the same Pi composition and native-directory Environment on a remote host;
- **Cloudflare** is a constrained host with Durable Objects, R2, a logical filesystem, and fenced Sandbox command leases.

“Runtime” is an implementation term, not a third pluggable product boundary.

## Shape

```text
Pi official TUI ─────────────── Pi CLI + Grove extension

desktop / mobile ─ GroveClient ─ Grove host
                                    ├─ Pi Coding Agent SDK + Grove extension
                                    └─ Environment
                                         local │ development machine │ Cloudflare
```

`mise run tui` intentionally launches Pi's official TUI. It is the smallest executable test of Grove's Pi configuration and should not grow into a second Grove frontend.

Desktop and mobile use the transport-neutral `GroveClient`. The first local binding may be in-process. Electron later exposes the same serializable operations through validated preload calls, and a development-machine host carries them over an authenticated transport. The contract's semantics do not change with the binding.

## The external interface

Keep the behavioral interface to three operations plus cleanup:

```ts
interface GroveClient {
  sync(): Promise<GroveSync>;
  execute<TCommand extends GroveCommand>(command: TCommand): Promise<ResultFor<TCommand>>;
  watch(options?: GroveWatchOptions): AsyncIterable<GroveUpdate>;
  close(): Promise<void>;
}
```

Typed handles may sit on top, but must preserve these semantics.

### Sync

`sync()` returns an authoritative serializable snapshot and durable cursors. It includes only client-projectable state: Task and Session summaries, current Turn and approval state, Agent and Environment capabilities, model summaries, credential status without secrets, and journal cursors.

Snapshot and watch registration must not lose updates. The current in-memory binding solves this with synchronous watch registration and replay; durable hosts must make the same guarantee at their journal boundary.

### Commands

Commands are discriminated serializable values with stable command IDs. Reusing an ID with identical input returns the recorded result; reusing it with different input is rejected. Long operations return an accepted receipt promptly and settle through a durable event.

Grow command families only as vertical slices require them: prompt and cancel, model and thinking selection, Task and Session lifecycle, approvals, then terminals and revision operations. Unsupported behavior is an explicit capability/result, never inferred from an Agent name.

### Updates

`watch()` carries two kinds of update:

1. **Durable events** have a stream identity and monotonically increasing sequence and reduce into authoritative Grove state.
2. **Progress** is transient and scoped to identified Turns, messages, tool calls, commands, or transfers.

Reconnection starts from a fresh snapshot plus cursor replay. Transient chunks may be lost; the next snapshot must still be correct. Terminal bytes remain a separate bounded stream.

## Ownership

| Concern                                                                                      | Owner                      |
| -------------------------------------------------------------------------------------------- | -------------------------- |
| Pi loop, model/auth handling, messages, tools, extensions, compaction, conversation tree     | Pi Coding Agent            |
| Workspace, Project, Task, Environment, revisions, durable journals, receipts, client schemas | Grove                      |
| Files, processes, terminals, credentials, isolation, and command execution                   | Environment implementation |
| Rendering, local view state, optimistic presentation, and reconnect UI                       | Desktop or mobile client   |

A Grove Session may retain Pi continuation metadata, but Pi's private entries and runtime objects do not become cross-client contracts. Grove should project only the state its product needs.

## Pi composition

`@grove/pi-extension` is Grove's typed customization point. Prefer an extension or Pi SDK option whenever Pi exposes the needed hook. The local host should not wrap or rename Pi concepts without adding a Grove semantic.

The extension can grow to provide:

- Grove-specific context and status;
- approval and permission hooks;
- Environment-backed tool definitions where native Pi operations are not appropriate;
- durable event projection hooks;
- typed commands needed by Grove hosts.

The programmatic host constructs the same extension through Pi's SDK. The official TUI loads its file directly. This keeps one customization layer while letting Pi own both interfaces.

## Models and authentication

Use Pi's model runtime, provider implementations, model metadata, OAuth subscriptions, refresh behavior, and request normalization. Grove projects serializable model references and credential status. Credential material stays in the owning Environment and never enters Task state, transfers, transcripts, or logs.

## Tools and Environments

Use Pi's built-in tools unchanged for ordinary local and development-machine directories until Grove needs a concrete permission, durability, or Environment behavior they do not provide.

For Cloudflare, keep Pi-compatible tool definitions but inject different operations:

- `read`, `list`, `search`, `edit`, and `write` operate on the Task's logical filesystem;
- command execution uses a revision-fenced Sandbox lease and settles only after changed content is committed;
- approval happens before the Environment operation;
- output truncation and projection happen once at the Pi/Grove seam.

An extension can replace Pi's registered tool definitions, as demonstrated by Pi's own operation-injection extensions. That makes the tool seam viable, but it does not eliminate the Cloudflare host: Durable Objects still own persistence, journals, receipts, authorization, and multiplayer connections.

Cloudflare should begin with a compatibility spike against the current Pi Coding Agent package. If host-only imports prevent a clean Worker build, use the smallest Pi core subset behind the same Grove boundary. Migrate to Pi AgentHarness V2 once its session operations are implemented and stable.

## Current checkpoint

The checked-in slice now has:

- a Promise/`AsyncIterable` Grove client contract with command deduplication, replay, transient progress, cancellation, and isolated subscribers;
- a Pi Coding Agent SDK-backed Session using Pi's real models, auth, tools, extensions, and in-memory session manager;
- a typed Grove Pi extension shared by the SDK host and official Pi TUI;
- Vitest contract coverage for the Grove client behavior.

The state journal and command receipts are process-local. The next runtime slice should put them behind durable local Task and Environment state before desktop integration.

## Next vertical slices

1. Persist Grove Task/Environment snapshots, journals, receipts, and Pi continuation identity locally.
2. Inject Task-directory and permission context into Pi without replacing its native local tools.
3. Project the Grove client through validated Electron main/preload calls for the desktop.
4. Add native-directory Task isolation and explicit copy fallback.
5. Add the authenticated development-machine binding using the same Pi composition.
6. Benchmark the Cloudflare Pi/VFS/Sandbox host before fixing its production design.

Do not begin with a universal wire protocol, daemon discovery, universal VFS, or a second extension system.
