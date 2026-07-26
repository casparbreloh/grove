# Grove Vision

Grove is a small, Pi-first Change manager over native Git worktrees. It should
make isolated agent work as easy to begin, leave, resume, synchronize, publish,
and safely remove as ordinary local development.

Git remains authoritative for source and history. Pi remains authoritative for
conversation and interactive agent behavior. The code host remains authoritative
for pull requests and CI. Grove owns only the narrow local coordination needed
to connect them safely.

## Product contract

- A **Change** is a durable local entity, not a branch, process, terminal, or Pi
  session.
- Creating a Change is one dependable local action that opens Pi in an isolated
  native Git worktree.
- Closing Pi does not end the Change. Resuming starts Pi normally against its
  existing native sessions; Grove does not keep an agent process alive.
- The bare command is the fast, local-only navigator. It launches the agent or
  moves the calling shell without becoming a dashboard.
- Explicit commands authorize their documented effects. Grove performs no
  implicit fetch, push, provider mutation, or destructive cleanup.
- AI supplies bounded semantic output where it is useful. Grove performs Git,
  filesystem, and provider mutations deterministically.
- Every destructive or remote transition validates the exact state it observed,
  fails closed on races, and leaves interruption recoverable.
- The common workflow stays small. New abstractions and commands must eliminate
  real repeated work rather than expose every possible Git operation.

## Current foundation

A Change has an immutable opaque ID, one stable human Title, a private capsule,
a path-backed Git worktree, and any number of Pi-native sessions. Those
identities remain separate from its publication branch and pull request.

Workspaces begin detached. Grove creates a readable Title-derived publication
branch only when shipping needs one and adds the Change ID only on collision.
The selected branch and last pushed commit are recorded so retries can prove
ownership. Exact push leases prevent unobserved remote history from being
replaced.

`grove sync` is the repository-wide convergence command. From Main it fetches
and fast-forwards only Main's configured upstream, archives conservatively
proven integrated Changes, and rebases every other eligible unpublished Change
onto the updated Main tip. It never rewrites published history or aborts a Git
operation it did not start. Ordinary untouched Changes stay quiet; restored
rebase conflicts are reported and make the completed batch unsuccessful.

`grove ship` stages the complete Change, obtains structured commit and
pull-request metadata from an isolated Pi worker, commits, pushes, and creates or
updates the pull request. Existing pull-request targets and concurrent edits are
revalidated. Failures may leave honest partial effects; rerunning converges from
them instead of pretending Git and the provider form one transaction.

Archival uses Git topology and resulting content rather than provider status. It
preserves tracking branches, honors worktree and activity locks, rejects active
Git operations, and records only the facts needed to finish an interrupted
removal safely.

## Current foundation: make local state legible

Reliability comes before a broader workflow surface. The deliberately narrow
`grove doctor` command inspects local Grove and Git state without changing it.
It explains:

- malformed or incomplete Change records;
- capsules whose worktrees are missing, stale, or registered at the wrong path;
- interrupted archival state;
- publication branches or pushed commits that no longer agree with the record;
- unsafe permissions and lock files that cannot be opened.

Grove's private JSON deliberately has no schema-version field and ordinary
commands contain no migration framework. Pre-1.0 changes may break old local
records. They should remain untouched and fail clearly rather than accumulate
compatibility code.

Repairs must be explicit, narrow, and inspectable. If `doctor` eventually writes
state, it should validate again immediately before replacement and preserve a
private rollback copy. One-off development conversions belong in one-off tools,
not in every future Grove invocation.

Corruption or an intentional clean break should produce one useful diagnosis;
source, sessions, branches, and worktrees remain untouched.

## Next product direction: begin from intent

A future Change may start from an issue URL or a concise goal rather than an
empty first Pi turn. The exact interface is intentionally undecided; possible
shapes include:

```text
grove new --issue <url>
grove new --goal "replace the cache implementation"
```

The command would resolve only the requested context, create the Change through
the normal local transaction, and hand Pi a focused initial brief. It must keep
issue-provider access explicit, bounded, and replaceable. Grove should not grow
a project-management database, backlog, generic connector framework, or hidden
planning agent.

The principle is more important than the flags: move cleanly from intent to an
isolated interactive Change without requiring the user to copy context or invent
a branch name.

## Demand-gated frontiers

These remain possibilities, not a roadmap:

- **Exact-state forks.** Create another Change from explicitly captured staged,
  unstaged, deleted, mode-changed, and non-ignored untracked state. Validate the
  source before and after capture, exclude secrets and regenerable ignored data,
  and roll back completely on failure. Native commits remain the preferred
  sharing mechanism.
- **CI-aware continuation.** Review remains owned by CI and the code host. If a
  repeated workflow emerges, an explicit Grove action may bring failed checks or
  review context back into the existing interactive Change. The command shape is
  deliberately not prescribed, and there will be no background polling.
- **Alternative evaluation.** Explicitly compare Changes from a shared base with
  tests, benchmarks, or a bounded read-only evaluator without becoming an agent
  scheduler.
- **Additional native seams.** More shells, code hosts, or interactive agents
  require demonstrated demand and the same small, trustworthy boundary as the
  current Pi integration.

## Lessons from the edge

Grove borrows principles, not product surfaces:

- [Worktrunk](https://worktrunk.dev/) demonstrates that worktree creation,
  navigation, integration detection, and removal can be fast enough for routine
  parallel agent work. Grove keeps its local-first inventory and conservative
  cleanup, but not hooks, services, dashboards, or branch-as-identity.
- [Rift](https://github.com/anomalyco/rift) demonstrates the value of treating
  unfinished workspace state as a valid creation base and keeping workspace
  identity independent of Git branches. Grove would adapt that idea through
  validated Git-native capture rather than whole-directory snapshots or copied
  secrets and caches.
- [Cloudflare Agents](https://developers.cloudflare.com/agents/) reinforces that
  durable identity and durable recovery facts matter more than a permanently
  running process. Grove applies that locally: Changes survive, Pi processes do
  not; side effects use explicit checkpoints and idempotent boundaries rather
  than a workflow engine.

The shared direction is clear: cheap isolation, durable identity, disposable
execution, progressive local truth, least-authority tools, and recoverable
side effects.

## Deliberately outside Grove

Grove is not a daemon, terminal multiplexer, hosted workspace, generic workflow
engine, agent scheduler, plugin platform, project setup system, or alternate
version-control system.

It should not own background agents, persistent activity status, automatic CI
polling, notifications, preview dashboards, arbitrary lifecycle hooks, cloud
uploads, ignored-file synchronization, provider credentials, Pi transcripts, or
model-generated Git operations.

The test for every future feature is simple: does it make the path from intent to
a safe reviewed Change materially shorter while preserving native Git and Pi?
If not, it does not belong in Grove.
