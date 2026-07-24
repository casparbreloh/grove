# Grove Vision

Grove is a Pi-first Change manager: a small layer over Git that makes isolated
agent work easy to create, leave, find, resume, inspect, and ship. Git remains
the source of truth. Grove should complete the path from an idea to a reviewed
pull request without becoming a version-control system, agent framework,
workflow engine, or terminal multiplexer.

## Principles

- Keep the common workflow and command surface small.
- Make the primary interface information-dense, simple, and clean.
- Give every Change an immutable opaque identity and one stable human title.
- Keep titles, Git identity, publication identity, and Pi-native session
  identity separate.
- Use Pi through its native TUI, native JSONL, and isolated `--print` workers
  instead of adding a second agent or provider abstraction.
- Keep interactive Pi lifecycle native, including `/new`, `/fork`, `/clone`,
  `/resume`, and native session navigation.
- Prefer native resume over owning a background process or terminal
  persistence.
- Treat an explicit command as authorization for its documented local,
  provider, and remote effects; do not add preview ceremony to ordinary work.
- Validate before destructive or remote mutation, report partial effects
  honestly, and make interrupted work safe to retry.
- Preserve useful local history without making Grove an undo system.
- Add complexity only when common, demonstrated workflows require it.

## Foundation

Grove provides path-backed Git workspaces, a stable Change identity and title,
direct Pi launch and resume, recorded creation lineage, Git-backed inventory,
destructive validation, explicit upstream synchronization, and safe archival.
A Change's repository-scoped 8-hex ID identifies only its capsule. Each
workspace is a native Git worktree created with detached HEAD; branches appear
only when a user, agent, or hosting provider needs one. Archival cleans up
local-only branches while preserving tracking branches. The capsule groups
minimal Grove metadata, the active workspace, and Pi-native sessions beneath
one private `~/.grove` path.

Creating a Change remains a dependable local operation. Grove completes the
capsule and worktree before starting interactive Pi. A small managed extension
links every native Pi session to the Change and starts isolated, best-effort
`pi --print` title inference from the first substantial prompt. Naming runs
without delaying the real turn, never moves a path or renames Git, and leaves
an honest `Untitled` state when it fails.

A Change may contain many native Pi sessions. Running `/new`, `/fork`, or
`/clone` inside its workspace creates another Pi-owned session in the same
Change; `/resume` and Pi's native navigation can return to an earlier one.
Grove's extension is rebound for each native session and links it without
inventing a Grove session identity. A session transition must not detach the
Change, lose another native session, retitle the Change, or allow stale
asynchronous naming work to affect the replacement session. Pi owns its JSONL
and Grove never rewrites or normalizes it.

Grove deliberately does not keep Pi running after its terminal closes. There
is no daemon, PTY host, detach key, or multiplexer. Resuming a Change starts Pi
again against the same native session directory. The advisory activity lock is
a safety boundary against concurrent writers and destructive mutation, not a
user-facing agent status model.

`grove sync` explicitly fetches exactly the primary branch's configured merge
ref into its upstream-tracking ref, fast-forwards the local primary branch, and
leaves unrelated remote refs in place. It archives clean integrated Changes,
rebases eligible clean linear Changes onto the fetched upstream, and
conservatively skips Changes that cannot be synchronized safely. The batch is
best-effort and may be partially completed if a later operation fails.

## Phase 1: Grove as the navigator

Bare `grove` becomes the primary and only inventory interface. It is an inline,
transient, Git-backed navigator: adaptive to terminal width, information-dense,
and visually quiet. Titles remain dominant while concise columns expose useful
local facts such as creation base, changed lines, conflicts, divergence, and
path. Narrow terminals remove secondary information before compromising Change
identity.

Main is pinned first as the base workspace, selected initially, and followed by
active Changes. Arrow keys move the selection. The agent is the obvious primary
action for a Change: Enter launches or natively resumes Pi, while Tab performs
the secondary shell action. Main navigates to the primary worktree with either key. Change
creation remains an explicit `grove new` operation.

The navigator renders in the normal terminal flow rather than taking over an
empty alternate screen and clears its transient region before returning. Its
header and rows use the terminal's default foreground, selection is shown only
with `›`, and the selected row receives no extra emphasis. One blank line
separates the rows from a concise, muted, Title-aligned hint containing only the
contextual Enter and Tab actions. Styling must honor `NO_COLOR` and `TERM=dumb`.

It is a selector and launcher, not a dashboard or replacement for Pi. It has no
preview panes, tab system, transcript view, agent activity model, background
refresh, implicit network queries, separate `list` snapshot, or secondary
action palette.

This interface completely replaces `grove switch`, `grove list`, and both uses
of `--shell`. The commands and options are removed without aliases, migration
paths, or compatibility surfaces. The navigator expresses shell navigation
through its selected row while the calling-shell wrapper remains the thin
mechanism that performs it. Grove is pre-1.0 and should prefer the smaller final
workflow over preserving obsolete commands or options.

Shell navigation should preserve an equivalent relative subdirectory when it
exists in the destination and otherwise use the workspace root. Actions that
create or mutate before navigating must validate calling-shell support first.
Calling-shell support should remain a thin explicit wrapper and grow to common
shells without turning shell configuration into Grove state.

## Phase 2: AI-native shipping

`grove ship` is one explicit, foreground, end-to-end command that takes an
active Change from working state to a created or updated pull request. It does
not expose a dry-run or staged publication workflow. Invoking it authorizes the
documented commit, branch, provider, push, and hosting effects needed to ship
the Change.

After inexpensive local validation, Grove starts a purpose-built, noninteractive
Pi `--print` worker in the Change workspace with full agent tools and project
context. The worker inspects the complete non-ignored Change, prepares a clear
commit series following Conventional Commits, creates an appropriate
publication branch, pushes it, and creates or updates the pull request with a
useful title and body. Existing commits are source history rather than a
required presentation: the shipping agent may organize the final publication
history when necessary. Grove captures the starting Git state for that process
and validates the resulting repository and pull request without persisting a
source snapshot or remote-state record.

The command remains in the foreground and returns success only with the created
or updated pull-request identity and concise current CI or review information
when the host provides it. There is no daemon, queue, hidden continuation,
background polling, or Grove-owned copy of remote state. Pi, Git, the remote,
and the hosting provider remain authoritative for their respective data.

Before starting the worker, Grove rejects a busy Change, unresolved Git state,
missing publication prerequisites, unsupported hosting provider, unavailable
authentication, or a repository without a usable push remote. A local-only
repository therefore receives one short explanation and no provider request or
partial local shipping work. The command promises a pull request rather than
silently redefining `ship` to mean commit-only work.

Shipping is explicitly agentic and may send source and repository context to
Pi's configured provider, consume tokens, run tools, create or revise commits,
create branches, contact remotes, and invoke hosting credentials. These are the
expected effects of the explicit command, not implicit activity elsewhere in
Grove.

Local Git and remote hosting operations cannot form one transaction. A failure
may leave prepared commits, a publication branch, a successful push, or a
partially created pull request. Grove reports completed effects precisely and
makes rerunning `grove ship` converge safely instead of promising rollback it
cannot provide. A shipped Change stays active through review; `grove sync`
archives it after conservative integration detection.

## Phase 3: exact-state forks, only if proven useful

Rift demonstrates the value of starting another workspace from unfinished
state, but Grove should adapt that workflow without becoming a whole-directory
snapshot manager. A future explicit Change-creation mode may fork the invoking
worktree's staged changes, unstaged tracked changes, deletions, modes, and
non-ignored untracked files into a new native Git worktree.

It must remain distinct from `--from @`, which means the invoking worktree's
commit. Capture must validate that the source did not change underneath it and
roll back completely on failure. The initial contract excludes ignored files,
`.env` secrets, dependency trees, build products, caches, external symlink
targets, Pi sessions, editor state, arbitrary setup commands, and copies of the
source `.git` directory.

This phase is demand-gated and must not delay the navigator or shipping. Native
Git commits remain the simple and preferred way to share a base between
Changes.

## Later and deliberately outside the core

Additional forge adapters and bounded AI assistance may follow demonstrated
shipping needs. Another interactive agent should require clear user demand and
a similarly small, trustworthy native seam.

Persistent agent status, dashboards, watch modes, notifications, statusline
ownership, generic hooks and plugins, automatic project setup, ignored cache or
secret copying, archived-source restoration, multi-agent orchestration,
uploads, cloud sandboxes, and live process persistence remain outside Grove.
The capsule and independent identities keep future integrations possible
without committing the core product to them now.

### Possible AI-native ideas, not a roadmap

These are small directions to reconsider only after the navigator and shipping
workflow prove what users actually need. Each would be an explicit command
backed by a bounded Pi `--print` worker, never implicit background activity.

- **Create from an issue or task.** Start a Change from an issue URL or concise
  goal, let Pi inspect the repository and available issue context, and hand a
  focused brief to the new interactive session.
- **Address pull-request feedback.** Read unresolved review comments and failing
  CI, implement and validate fixes, create Conventional Commits, and update the
  existing pull request.
- **Resolve synchronization conflicts.** In an explicitly requested mode, let
  Pi resolve a blocked rebase with repository context and tests while retaining
  Grove's ability to abort safely.
- **Review a Change independently.** Use a fresh worker to report correctness,
  test, security, scope, and commit-quality findings without silently changing
  the work.
- **Compare alternative Changes.** Evaluate implementations from a shared base,
  run relevant tests or benchmarks, explain tradeoffs, and recommend one
  approach without turning Grove into an agent scheduler.
