# Grove

Grove is an agentic development environment for durable software work with coding agents. It should feel local and immediate while allowing the same product model to operate on the local device, a development machine, or Cloudflare. People retain clear control over where source, commands, agents, and credentials live.

## Product model

The hierarchy is **Workspace → Project → Task → Session**.

- A **Workspace** is the switchable collaboration and settings context. It contains Projects, skills, defaults, Tasks, and restorable layout.
- A **Project** is fundamentally an Environment-owned folder-shaped file tree registered in one Workspace. Local and development-machine Projects are native filesystem folders; Cloudflare Projects use a logical filesystem until materialized. Its declared portable state and generated/cache policy are explicit and inspectable.
- A **Task** is the durable sidebar work unit. It starts from sealed Project state or a sealed Task revision and owns isolated mutable filesystem state, Sessions, revisions, and resources such as terminals and approvals. A Task remains in one Environment.
- A **Session** is one persisted, independent agent conversation inside a Task. Sessions share the Task's files but have private conversation and provider continuation state. Replacing an Agent or its continuation does not replace the Session or Task.
- A **Turn** is the internal cycle from one accepted prompt to settlement, failure, or cancellation. It is not another sidebar resource.
- An **Agent** is a selectable coding-agent implementation or configuration.
- An **Environment** is where Project and Task files, processes, terminals, credentials, journals, and command receipts live. The supported kinds are local, development machine, and Cloudflare.
- A **Tab** is a device-local view onto a resource. It never owns the resource's lifetime.

New product nouns require an independent user-visible lifecycle. Provider continuations, transport connections, overlays, whiteouts, snapshots, and cursors remain implementation details.

A Task cannot acquire a second authoritative Environment. Initial movement explicitly seals and exports supported state, imports it into a distinct destination Project, and creates a new Task there while preserving the source. Grove does not maintain live, bidirectional writable replicas.

## Experience

The sidebar switches Workspaces and presents durable Tasks as its primary rows. Device-local views may show all Tasks flat, group them by Project, or scope them to one Project; Projects remain structural context rather than competing sidebar resources. Flat Task rows retain visible Project context. Opening a Task restores its Sessions and relevant device-local layout. A Task may have multiple independent Sessions for alternate approaches or parallel reasoning while all Sessions see the same current Task files.

A new-Task draft selects its Project and starting state above the composer. The same control can register a Project through its Environment. Project selection locks when Grove accepts the Task; an existing Task cannot be reparented by changing composer context. Starting a new Session keeps the current Task, Project, Environment, and files.

Core tab kinds initially include chat and terminal. Persist serializable descriptors with a `tabId` distinct from the viewed resource ID. Closing a tab hides a view; it does not cancel a Turn, stop a terminal, delete a Session, or delete a Task. Unknown tab kinds restore as recoverable unavailable views.

The Environment is visible for consequential actions. Local Tasks are private. Cloudflare Tasks in shared Workspaces are shared, with multiplayer presence and ordered activity replay.

## Durable workflow is the product

Grove's moat is the durable Task workflow: exact forks of declared current state, independent Sessions over shared Task files, multiplayer journal and replay, recovery after disconnection or executor loss, compare/apply between Tasks, and explicit mobility between Environments. Reconstructable caches may be copied opportunistically but are not part of the fork's semantic state.

The moat is not a copy-on-write syscall, a particular virtual-filesystem schema, or a Git abstraction. Those are replaceable implementation choices beneath stable Task semantics.

## Change-native review direction

Grove should be change-native without becoming another version-control system. A Task remains the durable work unit; Sessions and Turns contribute to its revisions. Git commits, branches, and pull requests are integrations projected from Task state rather than Grove's source of truth.

Jujutsu and Sapling suggest the direction: checkpoint declared Task state at safe boundaries instead of exposing a staging area; separate stable review intent from immutable content revisions; and make a linear stack of reviewable Changes easy to edit and reorder. A forge integration can project resolved Changes into stacked commits, branches, and pull requests without making those Git objects authoritative.

Later vertical slices may add operation-log undo, automatic descendant restacking, split, squash, absorb, and first-class conflicts. These are product ideas to validate, not requirements for the initial Task model or a reason to implement a VCS now.

## Architecture

Grove has exactly two pluggable product boundaries: **Agent** and **Environment**.

```text
renderer
  assistant-ui projection + Grove views
          │ narrow typed API
          ▼
desktop main / client core
  Grove domain, local persistence, permissions, tabs
          │
          ├── Agent boundary
          │     Pi │ ACP-compatible process
          │
          └── Environment boundary
                local │ development machine │ Cloudflare
                Task files, revisions, commands, terminals,
                credentials, journals, receipts, replay
```

The **Agent boundary** maps an Agent's capabilities and behavior into Grove Sessions, Turns, messages, content, tools, approvals, plans, usage, cancellation, and errors. Provider-native state remains private. Optional capabilities such as resume, fork, steer, attachments, models, and commands are discovered rather than inferred from names.

The **Environment boundary** provides Project access, Task isolation and revisioning, filesystem operations, processes, terminals, Environment-scoped credentials, durable event journals, mutating-command receipts, connection lifecycle, and replay.

An Agent must not assume an Environment; an Environment must not assume an Agent. Source-local tools use injected Environment capabilities. Only translation to an external protocol is called an **adapter**; ACP is such an adapter, while Agent and Environment are implementation boundaries.

Grove's domain is the source of truth, not Pi, ACP, assistant-ui, Electron, or a transport. assistant-ui renders it through the library's external-store interface.

### Electron security

Electron main owns Grove persistence, permissions, IPC, the local Environment, local Agent instances, and supervision of remote Environment connections. Preload exposes a narrow typed Grove API. The renderer is unprivileged: it cannot import Node or Electron or access generic IPC, RPC, shell, SSH, filesystem, or Environment operations.

Preserve context isolation, renderer sandboxing, and no Node integration. Validate every renderer input in main and every network payload at its receiver.

## Agent support

Pi is Grove's universal default and the only planned Cloudflare Agent.

| Agent path                     | Local                                                        | Development machine                                          | Cloudflare                                                                    |
| ------------------------------ | ------------------------------------------------------------ | ------------------------------------------------------------ | ----------------------------------------------------------------------------- |
| Pi                             | Official TypeScript SDK; official RPC optional               | Official TypeScript SDK; official RPC optional               | `pi-agent-core` and selective `pi-ai`, with Grove-owned persistence and tools |
| ACP-compatible process         | Supported through the ACP adapter                            | Supported through the ACP adapter                            | Not supported                                                                 |
| Claude or Codex via ACP bridge | Only after bridge maintenance and capabilities are validated | Only after bridge maintenance and capabilities are validated | Not supported                                                                 |

Local and development-machine Pi use the official TypeScript SDK for high-fidelity events, tools, extension UI, and conversation-tree behavior. Official Pi RPC is an isolation option, not a different product path. Do not route Pi through a community ACP bridge without demonstrated parity and a credible maintenance path.

Cloudflare cannot use the full Pi coding-agent package because it is not Worker-native. A Worker-native Pi integration uses `pi-agent-core`, only the needed `pi-ai` providers, and Grove-owned Session persistence and Environment-backed tools. Pi file, list, search, and edit tools operate directly against Grove's virtual filesystem without a Sandbox.

ACP requires a compatible subprocess and therefore works only on local and development-machine Environments. Claude and Codex currently need separately maintained bridges. Grove does not plan to lease Cloudflare Sandboxes for agent runtimes, so ACP, Claude, and Codex are unavailable there.

## State, events, and recovery

Use stable IDs for Workspaces, Projects, Tasks, Sessions, Turns, Agents, Environments, messages, tool calls, approvals, terminals, commands, and tabs. Preserve useful opaque provider IDs only as namespaced metadata.

Durable streams have identity and monotonically ordered sequence numbers. Reconnection uses a current snapshot plus cursor replay; events prefer ID-addressed upserts and chunks over anonymous deltas. Terminal byte streams stay separate from durable domain events.

The selected Environment is authoritative for active-Turn journals, Task revisions, and mutating-command receipts. Redeliverable mutations carry idempotency keys and support deduplication or status lookup. Initially, each Task permits one active mutating Turn; Sessions may remain independently readable, but filesystem mutation is serialized.

Credentials are scoped to a user and Environment, stored by that Environment, and never shared through a Task or included in transfer. Persisted Session transcripts are private Task data and may contain source excerpts; credentials, private source, and full transcripts must not enter operational logs.

## Task filesystem semantics

The isolation primitive is a **sealed base snapshot plus writable Task state**. It is not a Git branch or worktree. Forking seals an exact declared current state and creates independent writable descendants; no descendant may mutate immutable ancestry.

Git is optional and subordinate. Grove preserves declared dirty, untracked, generated, and non-Git source state without presenting staged versus unstaged files as its change model. Compare and apply operate on Grove Task revisions and declared Project state; Git integrations may project those results separately.

### Declared portable state and caches

A Project declares which source state is portable. Seal, transfer, diff, and apply use explicit manifests and immutable content, including declared non-Git and untracked files. Exclusions are Grove policy and must not be silently equated with `.gitignore`.

Reconstructable Environment caches are separate. A local clone may retain `node_modules`, build outputs, package caches, or other generated files for speed, but that does not make them portable. Generated-dependency and cache policy must remain explicit and inspectable so a person can understand fidelity, transfer size, and reconstruction cost.

Portable snapshots are explicit seal/export/import manifests plus content. Live synchronization between writable replicas is not supported.

### Local and development machine

Tasks use ordinary native directories so external Pi, ACP, Claude, Codex, editors, Git, compilers, and package managers receive a normal `cwd`.

Grove orchestrates first-party platform commands from TypeScript: APFS clones, reflinks, or btrfs copy-on-write snapshots where available. There is no Rust service or native addon now, and Grove must not build a universal local FUSE/SQLite filesystem.

The implementation must probe capabilities and report the selected strategy honestly. If native copy-on-write is unavailable, use an explicit full-copy fallback and disclose its cost, metadata fidelity, exclusions, and consistency limits. Do not claim atomicity that the platform operation does not provide. Quiesce Grove-controlled Agent tools, terminals, and background processes when a consistent seal or clone requires it; detect or disclose that uncontrolled external writers can still race. Do not silently omit dirty, untracked, or non-Git declared state.

A development-machine Grove service deploys the same native Environment implementation and the selected Agent implementation. The Environment owns files, processes, credentials, journals, and receipts without depending on Agent details. Disconnection is recoverable through snapshot and cursor replay; ambiguous mutations are resolved by command receipt rather than repeated blindly.

### Cloudflare

Cloudflare implements the same Task semantics with a different storage and execution design:

```text
Worker gateway
      │
      ▼
Task-family Durable Object
  SQLite: Tasks, Sessions, revisions and snapshot ancestry,
          overlays and whiteouts, journals, receipts,
          multiplayer connections, blob references
      │
      ├── R2: large immutable content blobs
      ├── Worker-native Pi: direct VFS file/list/search/edit tools
      └── exclusive revision-fenced Sandbox lease
            materialize → execute POSIX command → collect delta
            → upload blobs → commit revision and outcome → release
```

A Task-family Durable Object is the durable coordinator. Its SQLite database owns Tasks, Sessions, revisions, snapshot ancestry, logical overlays and whiteouts, ordered journals, command receipts, and multiplayer connection state. Large immutable file content belongs in R2, not SQLite. Durable Object WebSocket hibernation keeps idle multiplayer inexpensive.

Normal Pi filesystem tools read and mutate the logical VFS directly; they do not require a Sandbox. A side Sandbox is leased only for an actual POSIX command, build, test, or package installation. Grove does not place the Agent runtime there.

A mutating command receives an exclusive lease fenced to the Task's starting revision. Grove materializes that revision into a normal directory, executes the command, quiesces Agent tools, terminals, background processes, and every other filesystem writer, collects changed content and tombstones, uploads immutable blobs, then transactionally commits the new revision and command outcome before releasing the lease. A stale fence cannot commit. The command remains provisional until synchronization commits; unsynchronized files and processes may disappear. One active mutating Turn per Task keeps this protocol initially tractable.

Benchmark materialization, command startup, dependency installation, delta collection, synchronization, recovery, and many-small-file behavior before production design is fixed. Cloudflare Computer and Turso AgentFS are preview/beta design references only, not dependencies. Grove also does not depend on Artifacts, ArtifactFS, `code.storage`, or an unsupported filesystem product.

## Security and privacy

- Validate schema, identity, authorization, resource ownership, and lifecycle state across every process and network boundary.
- Authenticate remote connections with scoped, short-lived connection credentials.
- Store user/Environment credentials with native secure facilities where available; never place them in Task snapshots, transfers, shared state, provider metadata, or logs.
- Project files can contain secrets. Export requires explicit scope and cannot promise automatic secret detection.
- Model approvals as identified requests with approve/deny and settled/failure states; show the Environment and consequential action.
- Treat plugins, hooks, executable skills, and Agent extensions as trusted code until a real isolation model exists.

## Delivery order

Ship the smallest vertical slices in this order:

1. Grove-owned Workspace, Project, Task, Session, Turn, message, capability, event, and durable local persistence.
2. Local Pi on that state with real Environment-backed tools.
3. Local native-directory Task isolation with copy-on-write probing and an explicit full-copy fallback.
4. Restorable chat and terminal tabs; compare/apply; cancellation and restart recovery.
5. The ACP adapter, followed by explicit Claude and Codex bridge validation.
6. An authenticated development-machine Environment with journaling, receipts, and replay.
7. A narrow Cloudflare Durable Object/Pi/VFS/Sandbox benchmark.
8. Production Cloudflare durability and multiplayer only after the benchmark validates the design.

Do not build a premature universal VFS to unify local and Cloudflare implementations.

## Near-term success

A person can open a Workspace and Project, create or resume a Task, use independent Sessions with Pi against isolated ordinary files, inspect tool activity, approve consequential actions, use chat and terminal tabs, fork exact declared current state, cancel safely, recover after restart, and understand what is portable.

The architecture succeeds when a new Agent or Environment can be added without replacing Grove's domain or renderer.

## Not yet

Live bidirectional writable replicas, automatic cross-Environment movement, unattended destructive work, arbitrary plugin installation, a public marketplace, a universal local VFS, and Cloudflare Sandbox-based agent runtimes are not early goals.

## Design references

These inform Grove's design but are not dependencies:

- [Agent Client Protocol](https://agentclientprotocol.com/)
- [assistant-ui runtimes](https://www.assistant-ui.com/docs/runtimes/concepts/architecture)
- [Rift](https://github.com/anomalyco/rift)
- [Jujutsu](https://jj-vcs.github.io/jj/latest/)
- [Sapling](https://sapling-scm.com/)
- [GitButler](https://gitbutler.com/)
- [Turso AgentFS](https://github.com/tursodatabase/agentfs)
- [Cloudflare Computer](https://blog.cloudflare.com/cloudflare-computer-use/)
- [Cloudflare Durable Objects](https://developers.cloudflare.com/durable-objects/)
- [Cloudflare Sandbox](https://developers.cloudflare.com/sandbox/)
