# Herdr-inspired session architecture for Grove

> **Historical research — superseded.** The project subsequently chose the
> smaller model recorded in [ADR 0001](../adr/0001-id-backed-native-pi-changes.md):
> direct blocking Pi, native JSONL resume, and no multiplexer, PTY host,
> detach/reattach, background process persistence, or multi-agent adapter work.
> The identity separation and warning against reproducing Herdr remain useful;
> the runtime recommendation, migration sequence, terminology, and source-line
> descriptions below are not the current Grove contract.

Research date: 2026-07-16

Herdr source: [`ogulcancelik/herdr`](https://github.com/ogulcancelik/herdr) pinned at [`a22454f27ce096585e19d1787dba43f56d1505cf`](https://github.com/ogulcancelik/herdr/tree/a22454f27ce096585e19d1787dba43f56d1505cf) (v0.7.4-era `master`)
Grove source: `ece0c39`

## Recommendation

Grove should be heavily inspired by Herdr's **identity and agent-integration model**, but should not become a Herdr-like full multiplexer.

The strongest direction is:

1. Make a stable Grove **task ID** the user-facing selector and internal worktree identity.
2. Keep a real Git branch as the durable commit anchor, but generate an opaque internal name such as `grove/7f3a91c2`; stop asking users to name it during `new`.
3. Treat a human title, the internal Git branch, the Grove task ID, the runtime process, and the agent's native session reference as five separate identities with different lifetimes.
4. Add Pi, Claude, and Codex through small agent adapters and agent-native hooks/extensions. Do this before replacing ZMX.
5. Prototype Herdr as an optional external runtime/attention adapter to validate the product benefit without giving it Git authority.
6. Then, if the value is proven and ZMX is the limiting factor, build a deliberately tiny Grove session host—one PTY, one agent, one worktree, attach/detach/inspect/stop—and run it beside the current ZMX implementation until compiled-CLI parity is proven.
7. Do not build panes, tabs, layout, screen-scraping status manifests, plugins, remote UI, orchestration, or a general socket API unless later product evidence demands them.

So the answer is **yes to owning the narrow persistent-session primitive eventually; no to building a general multiplexer now**. The CLI and identity simplification do not depend on replacing ZMX and should be validated first.

## Why this fits Grove

Grove's current vision says Git remains the source of truth, Grove is not an agent framework, the common command surface stays small, and each worktree has one native agent session ([`VISION.md`](../../VISION.md), lines 3–16). The current session module already exposes a narrow semantic surface—prepare, attach, active, terminate—while hiding ZMX invocation ([`src/session.rs`](../../src/session.rs), lines 84–203). This is already a good seam to deepen rather than discard.

The implementation is also closer to the proposed identity model than the written “branch as workspace identity” principle suggests. Grove already writes a stable 32-hex session ID into the linked worktree's Git directory ([`src/git.rs`](../../src/git.rs), lines 384–406 and 1158–1205), hashes that into the runtime session name ([`src/session.rs`](../../src/session.rs), lines 90–109), and therefore keeps runtime identity stable when the worktree path or branch changes.

The main accidental complexity is the delayed branch naming workflow. Bare `grove new` creates a detached worktree and metadata, Pi's first interactive prompt invokes hidden `grove __name`, then Git creates a branch, records lineage, and moves the worktree ([`src/git.rs`](../../src/git.rs), lines 433–460; [`src/pi-extension.ts`](../../src/pi-extension.ts), lines 4–50). An opaque branch created up front would remove the pending/naming/rename state machine while preserving a durable Git ref from the first mutation.

## What Herdr actually provides

### 1. A full terminal multiplexer and persistent server

Herdr advertises real terminal panes, detach/reattach, SSH access, session restoration, mouse/keyboard interaction, plugins, and a socket interface ([README](https://github.com/ogulcancelik/herdr/blob/a22454f27ce096585e19d1787dba43f56d1505cf/README.md#L25-L32)). Its implementation includes a background server, local sockets, client transport, PTY actors, terminal emulation, render streaming, layouts, tabs, workspaces, snapshots, handoff, remote support, plugins, and agent integrations.

The server auto-start path creates a detached process and waits for its socket before the foreground command becomes a client ([`src/server/autodetect.rs`](https://github.com/ogulcancelik/herdr/blob/a22454f27ce096585e19d1787dba43f56d1505cf/src/server/autodetect.rs#L179-L291)). Writable terminal ownership is explicit: one client owns an attachment, takeover is deliberate, and observe mode is read-only ([`src/server/headless.rs`](https://github.com/ogulcancelik/herdr/blob/a22454f27ce096585e19d1787dba43f56d1505cf/src/server/headless.rs#L2474-L2560)). Its versioned wire protocol distinguishes terminal attachment from app rendering and bounds payload sizes ([`src/protocol/wire.rs`](https://github.com/ogulcancelik/herdr/blob/a22454f27ce096585e19d1787dba43f56d1505cf/src/protocol/wire.rs#L1-L62), [`src/protocol/wire.rs`](https://github.com/ogulcancelik/herdr/blob/a22454f27ce096585e19d1787dba43f56d1505cf/src/protocol/wire.rs#L306-L398)). These are useful correctness patterns for a future Grove runtime.

Herdr's “survives restarts” story is also nuanced. Normal snapshot restoration starts a fresh shell in the saved working directory and can resume a native agent session; it does not resurrect an arbitrary live PTY. Preserving live processes across a server upgrade is a separate Unix FD-handoff protocol with quiesce/commit/rollback behavior ([`src/persist/restore.rs`](https://github.com/ogulcancelik/herdr/blob/a22454f27ce096585e19d1787dba43f56d1505cf/src/persist/restore.rs#L64-L118), [`src/server/headless.rs`](https://github.com/ogulcancelik/herdr/blob/a22454f27ce096585e19d1787dba43f56d1505cf/src/server/headless.rs#L4286-L4349)). Grove should initially preserve processes while its per-task host lives and use native agent resume after a host crash; live binary-upgrade handoff is not a first milestone.

This is not a small replacement for ZMX. At the pinned commit, Herdr contains roughly 199,000 lines of Rust across about 220 Rust files in `src/` and `tests/`; Grove contains roughly 3,800 Rust/TypeScript lines across `src/` and `tests/`. Herdr depends on Tokio, Ratatui, a vendored/patched `portable-pty`, Ghostty terminal bindings, local IPC, image handling, and more ([`Cargo.toml`](https://github.com/ogulcancelik/herdr/blob/a22454f27ce096585e19d1787dba43f56d1505cf/Cargo.toml#L21-L46)). The lesson is architectural, not that Grove should reproduce the implementation breadth.

### 2. Agent kind and activity inference

Herdr uses two related but distinct mechanisms:

- It identifies the foreground agent from the pane's foreground process group, including common wrapper runtimes and command lines ([`src/detect/mod.rs`](https://github.com/ogulcancelik/herdr/blob/a22454f27ce096585e19d1787dba43f56d1505cf/src/detect/mod.rs#L154-L188), [`src/detect/mod.rs`](https://github.com/ogulcancelik/herdr/blob/a22454f27ce096585e19d1787dba43f56d1505cf/src/detect/mod.rs#L270-L307)).
- It infers `idle`, `working`, `blocked`, or `unknown` from the terminal's live screen/OSC state using per-agent rule manifests ([`src/detect/mod.rs`](https://github.com/ogulcancelik/herdr/blob/a22454f27ce096585e19d1787dba43f56d1505cf/src/detect/mod.rs#L1-L39), [`src/detect/manifest.rs`](https://github.com/ogulcancelik/herdr/blob/a22454f27ce096585e19d1787dba43f56d1505cf/src/detect/manifest.rs#L138-L181)). Claude and Codex manifests show how version-sensitive these rules are: they match prompt chrome, OSC titles, permission dialogs, transcript viewers, and spinners ([Claude manifest](https://github.com/ogulcancelik/herdr/blob/a22454f27ce096585e19d1787dba43f56d1505cf/src/detect/manifests/claude.toml#L1-L158), [Codex manifest](https://github.com/ogulcancelik/herdr/blob/a22454f27ce096585e19d1787dba43f56d1505cf/src/detect/manifests/codex.toml#L1-L69)).

Grove does not need process inference to know which agent it launched. It also should not build a terminal emulator merely to gain screen-derived status. A smaller, more reliable contract is agent-native events where available, with `unknown` as an honest fallback.

### 3. Native agent session discovery and restoration

Herdr does **not** infer a Claude, Codex, or Pi native session ID from the process tree or screen. It installs agent-specific hooks/extensions/plugins. Those integrations receive the agent's native session ID or session path and report it to the pane-scoped Herdr socket. Herdr accepts resume-capable references only from official source/agent pairs and distinguishes ID references from path references ([`src/agent_resume.rs`](https://github.com/ogulcancelik/herdr/blob/a22454f27ce096585e19d1787dba43f56d1505cf/src/agent_resume.rs#L35-L70), [`src/agent_resume.rs`](https://github.com/ogulcancelik/herdr/blob/a22454f27ce096585e19d1787dba43f56d1505cf/src/agent_resume.rs#L94-L113)).

Restoration is an adapter table: Claude becomes `claude --resume <id>`, Codex becomes `codex resume <id>`, and Pi becomes `pi --session <path-or-id>` ([`src/agent_resume.rs`](https://github.com/ogulcancelik/herdr/blob/a22454f27ce096585e19d1787dba43f56d1505cf/src/agent_resume.rs#L115-L196)). The snapshot restoration path deduplicates native agent sessions before relaunching them ([`src/persist/restore.rs`](https://github.com/ogulcancelik/herdr/blob/a22454f27ce096585e19d1787dba43f56d1505cf/src/persist/restore.rs#L25-L35), [`src/persist/restore.rs`](https://github.com/ogulcancelik/herdr/blob/a22454f27ce096585e19d1787dba43f56d1505cf/src/persist/restore.rs#L257-L303)).

This separation is the most useful idea for Grove. A native session reference is late-arriving, optional, replaceable after reset/fork/new-session operations, and specific to an agent. It must not be Grove's worktree identity.

### 4. Agent integrations are intentionally asymmetric

Herdr supports many agent command names through a registry and has separate installation layouts for each integration ([`src/integration/registry.rs`](https://github.com/ogulcancelik/herdr/blob/a22454f27ce096585e19d1787dba43f56d1505cf/src/integration/registry.rs#L7-L52), [`src/integration/registry.rs`](https://github.com/ogulcancelik/herdr/blob/a22454f27ce096585e19d1787dba43f56d1505cf/src/integration/registry.rs#L252-L330)). It does not pretend all integrations have equal authority. Only selected sources are full-lifecycle hook authorities; others rely on screen-derived activity and use hooks primarily for native session identity ([`src/detect/mod.rs`](https://github.com/ogulcancelik/herdr/blob/a22454f27ce096585e19d1787dba43f56d1505cf/src/detect/mod.rs#L233-L243)).

At this commit, the Claude and Codex shell hooks accept `SessionStart` and report native identity, but do not provide full lifecycle activity ([Claude hook](https://github.com/ogulcancelik/herdr/blob/a22454f27ce096585e19d1787dba43f56d1505cf/src/integration/assets/claude/herdr-agent-state.sh#L15-L23), [Codex hook](https://github.com/ogulcancelik/herdr/blob/a22454f27ce096585e19d1787dba43f56d1505cf/src/integration/assets/codex/herdr-agent-state.sh#L15-L23)). Pi's extension, by contrast, reports both native session identity and lifecycle state ([Pi integration](https://github.com/ogulcancelik/herdr/blob/a22454f27ce096585e19d1787dba43f56d1505cf/src/integration/assets/pi/herdr-agent-state.ts#L87-L169)). Herdr also ignores stale monotonic reports and guards against cross-talk from replaced session generations ([`src/terminal/state.rs`](https://github.com/ogulcancelik/herdr/blob/a22454f27ce096585e19d1787dba43f56d1505cf/src/terminal/state.rs#L616-L729), [`src/terminal/state.rs`](https://github.com/ogulcancelik/herdr/blob/a22454f27ce096585e19d1787dba43f56d1505cf/src/terminal/state.rs#L1117-L1131)).

Grove should copy that honesty in a smaller form. `AgentAdapter` capabilities should be explicit—for example `resume`, `reports_session`, `reports_activity`—rather than forcing every agent into a fictional common lifecycle.

### 5. Herdr does not eliminate Git branches

Herdr's worktree creation accepts an optional branch, but if it is absent Herdr generates a namespaced branch such as `worktree/brave-river-0000` ([`src/worktree.rs`](https://github.com/ogulcancelik/herdr/blob/a22454f27ce096585e19d1787dba43f56d1505cf/src/worktree.rs#L21-L32), [`src/app/api/worktrees/deferred.rs`](https://github.com/ogulcancelik/herdr/blob/a22454f27ce096585e19d1787dba43f56d1505cf/src/app/api/worktrees/deferred.rs#L94-L139)). It then runs normal `git worktree add -b <branch>` plumbing ([`src/worktree.rs`](https://github.com/ogulcancelik/herdr/blob/a22454f27ce096585e19d1787dba43f56d1505cf/src/worktree.rs#L228-L305)).

Herdr's worktree removal delegates to `git worktree remove` and does not implement Grove's lineage-aware integrated-change detection and compare-and-delete branch cleanup ([`src/worktree.rs`](https://github.com/ogulcancelik/herdr/blob/a22454f27ce096585e19d1787dba43f56d1505cf/src/worktree.rs#L158-L184); compare Grove [`src/git.rs`](../../src/git.rs), lines 557–650 and 876–956). Herdr is therefore not a replacement for Grove's deep Git module.

### 6. Licensing matters

Herdr is AGPL-3.0-or-later with a commercial-license option ([README](https://github.com/ogulcancelik/herdr/blob/a22454f27ce096585e19d1787dba43f56d1505cf/README.md#L81-L88), [license header](https://github.com/ogulcancelik/herdr/blob/a22454f27ce096585e19d1787dba43f56d1505cf/LICENSE#L1-L7)). Architectural study and independently implementing general ideas are different from copying or embedding code. Any code reuse or linking should be treated as an explicit licensing decision, not an incidental refactor.

## Proposed Grove domain model

| Identity | Example | Lifetime | Authority |
|---|---|---|---|
| `TaskId` | `7f3a91c2` | Creation through removal | Grove metadata in the linked Git dir |
| `TaskLabel` | `Fix login redirect` | Mutable presentation | User text or first prompt |
| `GitBranch` | `grove/7f3a91c2` | Creation through ship/remove | Git |
| `AgentKind` | `pi`, `claude`, `codex` | Task default; explicitly changeable | Grove config/task metadata |
| `AgentSessionRef` | Claude UUID or Pi JSONL path | Optional and replaceable | Agent-native hook/extension |
| `RuntimeState` | running/exited | One hosted process generation | Grove session host |
| `AgentActivity` | working/blocked/idle/unknown | Ephemeral | Agent integration, when trustworthy |

The task ID is the join key. Everything else attaches to it.

This preserves Git as source of truth: commits remain anchored by a real `refs/heads/grove/<id>` branch, lineage stays in Git config, and removal still validates the real repository before mutation. “Branchless” should mean **branchless UX**, not detached, unreferenced history.

Using detached worktrees permanently is not recommended. While a linked worktree keeps its detached `HEAD` reachable during its lifetime, removal would discard the durable name protecting unique commits, complicate shipping, and weaken Grove's compare-and-delete safety. A generated internal branch is cheap and honest.

## Target deep modules

### `Git` module

Keep `src/git.rs` authoritative for creating, locating, shipping, and removing changes. Deepen its external interface around task identities rather than branch strings:

```rust
pub struct TaskId(String);

pub struct Task {
    pub id: TaskId,
    pub label: Option<String>,
    pub branch: String,
    pub path: PathBuf,
}

impl Git {
    pub fn create_task(&self, from: Option<&str>) -> Result<Task>;
    pub fn tasks(&self) -> Result<Vec<TaskView>>;
    pub fn resolve_task(&self, selector: Option<&str>) -> Result<Task>;
    pub fn prepare_task_removal(&self, id: &TaskId, force: bool) -> Result<PreparedRemoval>;
}
```

The implementation hides opaque branch generation, worktree paths, lineage, detached/pending migration, label persistence, and compare-and-delete.

### `SessionManager` module

Deepen `src/session.rs` into the one interface callers and compiled-CLI tests use:

```rust
pub struct OpenRequest<'a> {
    pub task: &'a Task,
    pub agent: AgentKind,
}

impl SessionManager {
    pub fn open(&self, request: OpenRequest<'_>) -> Result<()>;
    pub fn inspect(&self, task: &TaskId) -> Result<SessionView>;
    pub fn stop(&self, task: &TaskId) -> Result<()>;
}
```

`open` means “start or resume if needed, then attach.” The implementation hides executable validation, runtime selection, PTY launch, native-session resume commands, hook environment, attach transport, terminal mode restoration, and cleanup. `main.rs` should not know whether the runtime is ZMX or Grove-native.

During migration, a private `SessionRuntime` seam is real because it has two adapters: `ZmxRuntime` and `NativeRuntime`. Once ZMX is deleted, collapse indirection that no longer earns leverage.

### `AgentAdapter` seam

Multiple agents make this a real seam:

```rust
trait AgentAdapter {
    fn kind(&self) -> AgentKind;
    fn launch(&self, task: &Task, resume: Option<&AgentSessionRef>) -> Result<LaunchSpec>;
    fn integration(&self, task: &Task) -> Result<IntegrationSpec>;
    fn capabilities(&self) -> AgentCapabilities;
}
```

Adapters construct argv/environment and install or expose the smallest native hook. They do not own Git, PTYs, worktree identity, removal, or UI. Unknown activity is valid; a missing hook must not prevent attaching to a live terminal.

## The native session host Grove should build

The smallest useful implementation is not a layout multiplexer. It is one helper process per task:

- Own one PTY master and one agent child process.
- Listen on a per-task local socket under Grove's runtime directory.
- Proxy raw input/output and terminal resize messages.
- Support a single writable attached client; reject or explicitly make additional clients read-only.
- Interpret a detach escape without sending it to the child.
- Keep running when the client terminal closes.
- Expose `inspect` and compare-and-stop by task/runtime identity.
- Write minimal durable metadata atomically; keep live sockets/PIDs ephemeral.
- On host crash, relaunch via a trustworthy native `AgentSessionRef` when possible; otherwise report exited and preserve the worktree.

There is one hard technical gate: raw byte proxying alone cannot reconstruct an arbitrary TUI screen after detach. Replaying old bytes can repeat terminal side effects; sending only future bytes loses the current alternate-screen contents. Herdr solves this with a real terminal model and sends full/diffed rendered frames rather than raw replay ([`src/ghostty/mod.rs`](https://github.com/ogulcancelik/herdr/blob/a22454f27ce096585e19d1787dba43f56d1505cf/src/ghostty/mod.rs#L1-L27), [`src/server/render_stream.rs`](https://github.com/ogulcancelik/herdr/blob/a22454f27ce096585e19d1787dba43f56d1505cf/src/server/render_stream.rs#L12-L126)).

The first native prototype should therefore test whether Pi, Claude, and Codex reliably redraw their complete TUI after attach plus resize/SIGWINCH. If they do, Grove can keep the runtime raw and agent-scoped. If any supported agent does not, exact parity requires a private terminal-emulator dependency; that is still smaller than Herdr's UI, but it materially changes the cost. Grove should keep ZMX rather than silently ship lossy reattachment.

Do not retain a layout. Do not make runtime snapshots another source of worktree truth. Do not let a hook directly trigger destructive Git operations.

This design retains native agent TUIs and preserves Grove's one-session-per-worktree invariant. It removes the embedded 6.5 MB of ZMX archives once parity is achieved, but its real cost is ongoing PTY/signal/terminal correctness, not binary size.

## CLI shape

The common workflow can become task-first without adding commands:

```text
grove new [--from REF] [--agent pi|claude|codex]
grove switch [TASK]
grove list
grove remove [--force] [TASK]
```

- `grove new` creates `TaskId`, `grove/<id>`, lineage, and a stable path before launching the configured default agent.
- The first prompt may become the task label asynchronously. It never renames the branch or worktree.
- `TASK` accepts an exact/unique ID prefix, then an exact/unique label; ambiguity opens the picker or errors in non-interactive mode.
- `list` leads with label/ID and may show agent plus trustworthy activity. Branch is an advanced/detail field.
- `--agent` is an explicit override. Auto-detecting the default from installed executables is ambiguous when several are installed.
- `--shell` can remain as the explicit native-agent escape; changing it is independent of this architecture.

Existing named branches can continue to resolve as compatibility selectors. New opaque branches need not be renamed for a pull request unless a hosting workflow or user convention requires it; a future explicit `ship --branch <name>` can perform a validated rename. Remote effects remain explicit.

Automatic labels are best-effort, not a cross-agent guarantee. Grove's Pi extension sees the first interactive prompt, but Herdr's Claude and Codex session-start hooks report identity rather than a first prompt. A stable short ID must remain a good fallback label.

## Migration sequence

### Phase 1: identity without runtime change

- Introduce `TaskId`, `TaskLabel`, and generated `grove/<id>` branches.
- Stop moving worktrees or renaming branches from the first prompt; use the prompt only as a label.
- Migrate existing managed branches lazily: retain their branch and derive/read a task ID from their linked Git dir.
- Keep ZMX and Pi behavior otherwise unchanged.

This phase proves whether branchless UX is actually better and removes the most fragile naming state machine with minimal PTY risk.

### Phase 2: agent adapters and session reports

- Add `AgentKind` selection/configuration.
- Turn the existing Pi extension into the first `AgentAdapter` integration.
- Add Claude and Codex session-reference hooks with explicit capability flags.
- Store session reports by `(TaskId, source, sequence)`; reject stale/cross-task reports.
- Resume only from allowlisted first-party adapter reports.

This phase delivers multi-agent support and native resume without owning a PTY implementation.

### Phase 3: native runtime behind the existing seam

- First add an explicit experimental Herdr adapter. Grove remains the only module allowed to create or remove worktrees; the adapter may open/attach a Herdr workspace and overlay agent attention state onto Grove's Git inventory.
- Do not persist Herdr workspace IDs as Grove task identity; missing or deleted Herdr state must degrade to “runtime unknown,” never make Git work undiscoverable.
- Add the minimal session host and raw attach client.
- Keep `ZmxRuntime` available during development.
- Run the same compiled-CLI workflows against the ZMX, experimental Herdr, and native runtime adapters using real disposable Git repositories and the real fake-agent executable.
- Exercise detach, terminal close, resize, Ctrl-C forwarding, detach-key suppression, failed launch rollback, force stop, stale PID/socket reuse, and concurrent attach.
- Exercise full-screen redraw for each supported real agent before accepting a raw native runtime; otherwise add a terminal model or retain ZMX.

### Phase 4: remove ZMX

Delete ZMX only when the native adapter has behavioral parity on all supported OS/architecture pairs and the suite is green. Remove the old adapter and any migration-only seam that has become hypothetical.

### Deliberately later

Consider a dashboard, multiple sessions per worktree, socket automation, remote attach, or screen-derived status only after the one-task/one-agent workflow shows a concrete need. Each one changes Grove's product category and security/maintenance surface.

## Option comparison

| Option | Benefit | Cost/risk | Verdict |
|---|---|---|---|
| Keep ZMX and Pi-only | Lowest maintenance | Does not deliver multi-agent/task UX | Safe baseline, not destination |
| Use Herdr as Grove's runtime | Immediate rich mux and agent detection | External install, overlapping worktree authority/UI, protocol/version coupling, AGPL/commercial considerations | Optional future integration only |
| Embed/fork Herdr | Maximum feature reuse | Product-scale expansion, licensing decision, ~50× code surface | Reject |
| Build a full Herdr-like mux clean-room | Full control | Terminal emulator/layout/remote/plugin maintenance overwhelms Grove | Reject |
| Task IDs + adapters + tiny native session host | Simplifies UX, supports Pi/Claude/Codex, keeps Git deep and local | Real PTY correctness work, staged migration required | Recommend |

## Decision gates

Proceed with Phase 1 if opaque internal branches and label/ID selection feel better in a prototype. Proceed with Phase 2 if at least Claude or Codex demand is real. Proceed with the native runtime only if one or more of these are true:

- ZMX prevents a required lifecycle/status feature.
- Bundled binary provenance/update burden is unacceptable.
- Native session restoration must be coordinated with runtime crashes.
- Grove needs a stable local event channel that ZMX cannot provide cleanly.

If none is true after Phase 2, keeping ZMX is the simpler design—even with Herdr-inspired identities and adapters.
