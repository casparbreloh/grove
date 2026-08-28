# Grove runtime primitives

Status: foundation for the first runtime vertical slice. `VISION.md` remains authoritative for product semantics and delivery order.

## Recommendation

Build one deep Grove runtime module that owns the Grove domain and composes exactly one Agent implementation with exactly one Environment implementation for each Task. Desktop and TUI consume one transport-neutral Grove client interface; neither imports Pi, filesystem, process, Electron, Cloudflare, or runtime implementation types.

“Runtime” is an implementation term, not a third pluggable product boundary and not a synonym for Environment.

There are three Environment kinds but initially only two implementation families:

- **local** uses the native-directory Environment in the current process or a supervised local process;
- **development machine** uses the same native-directory semantics behind an authenticated remote host;
- **Cloudflare** implements the Environment interface with Durable Objects, R2, a logical filesystem, and fenced command leases.

The development-machine Environment should not acquire separate Task semantics merely because a transport is involved.

## Shape

```text
desktop projection ─┐
                    ├─ Grove client interface ─ binding ─ Grove runtime
TUI projection ─────┘                                  ├─ Agent
                                                       └─ Environment
```

The first local slice may bind the client directly in-process. Electron later exposes the same serializable interface through validated preload calls. A development-machine host later carries it over an authenticated transport. The interface semantics do not change with the binding.

Do not make HTTP, OpenAPI, CBOR, Electron IPC, or a Pi protocol the source of truth. Those are possible bindings around Grove contracts.

Internally, the local runtime is an Effect service composed from scoped Layers. Effect owns command serialization, Agent lifetime, Turn fibers, update fan-out, and cleanup. `ManagedRuntime` is the single bridge to the Promise/`AsyncIterable` client; Effect types do not leak into the external contract.

### Current checkpoint

The checked-in prototype covers the contract, direct in-process binding, Pi Agent projection, and TUI harness. Its state, replay journal, and command receipts are intentionally process-local; it is not yet the durable local runtime described in step 2 below. The next runtime slice must put those responsibilities behind a real local Environment implementation before the desktop adopts the client.

## The external interface

Keep the behavioral interface to three operations, plus explicit lifecycle cleanup:

```ts
interface GroveClient {
  sync(): Promise<GroveSync>;
  execute<TCommand extends GroveCommand>(command: TCommand): Promise<ResultFor<TCommand>>;
  watch(options: GroveWatchOptions): AsyncIterable<GroveUpdate>;
  close(): Promise<void>;
}
```

Typed convenience handles for Tasks, Sessions, terminals, models, and auth may sit on top of these operations. They must not introduce different semantics.

### Sync

`sync()` returns an authoritative, serializable snapshot plus the durable cursors from which observation continues. It is safe to call initially or again after reconnecting; it is not command execution. It includes only state a client may project:

- Workspace, Project, Task, Session, and tab descriptors;
- current Turn and approval state;
- Environment and Agent capability summaries;
- model/provider summaries and credential status, never credential material;
- current journal cursors.

Snapshot and watch registration must be atomic: an update cannot fall into a gap between reading the snapshot and beginning observation.

### Commands

Commands are discriminated, serializable values. Every accepted mutation has an operation ID; redeliverable Environment mutations also have an idempotency key. A long-running command returns an accepted receipt promptly. Its eventual success, failure, or cancellation is authoritative only after a durable settlement event; synchronous commands may return their settled result directly.

The first command families should follow the delivery order rather than attempting a complete protocol:

- create/open/fork a Task and create/open a Session;
- prompt, steer, follow up, cancel, and resume a Turn where the selected Agent supports them;
- request and settle an approval;
- select a model and thinking level;
- create/write/resize/close a terminal resource.

Do not encode capabilities in command names. A client reads explicit capability data and handles unsupported results.

### Updates

`watch()` carries two deliberately different classes of update:

1. **Durable events** have stream identity and monotonically increasing sequence. They are replayable and reduce into authoritative Grove state. Prefer identified upserts and settlements over anonymous deltas.
2. **Progress** is transient and scoped to an identified Turn, message part, tool call, command, or transfer. Text/reasoning chunks and live tool output improve presentation but never become authoritative merely because a client received them.

Reconnection starts from a fresh snapshot plus durable cursor replay. It may lose transient progress; the next snapshot must still be correct. Terminal bytes remain a separate bounded stream.

## Runtime ownership

| Concern                                                                              | Owner                                   |
| ------------------------------------------------------------------------------------ | --------------------------------------- |
| Workspace, Project, Task, Session, Turn, message, approval, and tab semantics        | Grove runtime                           |
| Provider loop, context shaping, compaction, model invocation, and Agent capabilities | Agent implementation, mapped into Grove |
| Files, processes, terminals, credentials, revisions, journal, and command receipts   | Environment implementation              |
| Rendering, local view state, optimistic presentation, and reconnect UI               | Desktop or TUI projection               |

Pi state is provider-private Agent state. Grove persists the Grove transcript and useful opaque continuation metadata without turning Pi entries, lanes, or protocol snapshots into product contracts.

## Models and authentication

Reuse Pi's provider implementations, model metadata, OAuth subscriptions, refresh behavior, and request normalization behind the Pi Agent implementation. Do not reimplement them in Grove unless a required Environment cannot run the relevant Pi package.

A Grove model reference should include the Agent identity as well as provider and model identity. Model availability and credential status are Environment-scoped because credentials live in an Environment. Authentication is an identified interaction with explicit challenges such as opening a URL, displaying a device code, or requesting a value. Challenges and outcomes are serializable; secrets are submitted directly to the owning Environment and never enter Task state or the event journal.

## Tools

Grove owns the tool definitions presented through its Pi integration. A tool receives an invocation context containing Task, Session, Turn, message, and tool-call identity plus narrow Environment capabilities. It does not inspect an Environment kind and choose an implementation.

- `read`, `list`, `search`, `edit`, and `write` call Environment file capabilities. The local Environment may use native filesystem calls or `rg`; Cloudflare may query and mutate its logical filesystem.
- `bash` asks the Environment command capability to execute. The local and development-machine implementations use native processes; Cloudflare uses a fenced Sandbox lease and returns a command receipt only after synchronization settles.
- Permission and approval admission happen before the Environment effect. Tool output projection and bounding happen once at the Agent seam.

Pi's injected `ExecutionEnv` and tool factories are useful implementation references. Grove should reuse them where their contracts fit, or implement the narrow Pi capability from a Grove Environment. Pi must not become the owner of Task files, command receipts, credentials, or revisions.

## First vertical slice

1. Define serializable Grove IDs, snapshots, commands, results, capabilities, durable events, and transient progress. Add runtime/client contract tests for snapshot-watch race freedom and subscriber isolation.
2. Implement the Grove runtime over durable local Workspace → Project → Task → Session → Turn state, with one direct in-process client binding.
3. Replace the current callback-based Pi prototype with a Pi Agent implementation that projects Pi events into Grove messages and progress and uses Pi model/auth logic.
4. Inject local Environment-backed read, search, edit, write, and command tools. Start with one active mutating Turn per Task.
5. Put both TUI and desktop behind the Grove client interface. The renderer reaches it only through validated preload calls.
6. Add a network binding and development-machine host only after the direct contract and recovery behavior are stable. Benchmark the Cloudflare Environment later in the order defined by `VISION.md`.

Do not begin with a universal wire protocol, automatic daemon discovery, Cloudflare VFS implementation, plugin system, or public SDK. They add surface before the local contract has earned it.

## Upstream lessons

OpenCode V2 demonstrates one shared core/server contract consumed by an embedded SDK, TUI, and graphical clients. Pi `main` now demonstrates an injected execution environment, durable harness, authoritative snapshots versus transient progress, and experimental transport-neutral protocol/server/client packages. See `docs/research/runtime-inspirations.md` and refresh the ignored `.repos/` checkouts before inspecting their source.
