# ADR 0001: ID-backed Changes with native Pi resume

- Status: Accepted
- Date: 2026-07-16

## Context

Grove previously coupled human branch naming, worktree paths, a Grove session
identity, and an embedded terminal runtime. The desired workflow is smaller:
start work without naming it, find it later by a useful title, and resume the
native Pi conversation without remembering session IDs. Live process survival
after closing a terminal is not required.

Git must remain the code-history authority, destructive removal must preserve
its safety boundaries, and local Change data should be straightforward to
inspect or analyze later.

## Decision

Each Grove Change receives an immutable random 32-hex Change ID at creation.
The ID is the exact local Git branch and the leaf capsule directory, but normal
commands expose a stable human Title instead. There is no semantic branch name,
pending naming state, branch rename, or worktree move.

All state is colocated at
`~/.grove/<repository-key>/<change-id>/`: the versioned `change.json`,
`worktree/`, Pi-owned `sessions/pi/`, and Grove-owned `artifacts/`.

Grove launches Pi directly and waits for it. It passes the capsule's native
session directory with `--continue` and a managed extension. Grove owns no PTY,
daemon, background process, or detach protocol. A per-capsule advisory lock
prevents concurrent managed writers and removal.

The extension links each native Pi session to the Change and starts an isolated,
fire-and-forget `pi --print` request from the first substantial prompt of each
unnamed session. Pi owns the resulting native session name. The first valid
result initializes the Change Title atomically; later sessions cannot retitle
the Change. Failure leaves `Untitled` and never blocks the real turn.

Before removal, Grove uses an alternate Git index to capture the complete
base-to-final worktree state as a binary-capable patch and statistics. Only
after durable artifact installation does it remove the worktree, compare-and-
delete the ID branch, and archive the record. Native Pi JSONL remains untouched.

## Consequences

- `new`, `switch`, and `remove` need no human selector argument; list and picker
  UX can be title-first while internal joins remain unambiguous.
- Closing Pi or a terminal stops computation. Resume means starting Pi again
  from its native JSONL, not reattaching to a live TUI.
- Naming can make one additional provider request per unnamed native session,
  sending the first prompt again and possibly incurring cost.
- Opaque local branches may need an explicit publication name in a future ship
  workflow.
- Capsules deliberately retain transcripts and source patches, so private modes
  and an eventual explicit retention policy matter.
- Pi is the only managed agent today. Independent Change and native-session
  identities leave room for future adapters without designing them now.
- This is a pre-1.0 break. Older runtime, worktree, and branch metadata is
  neither migrated nor deleted automatically.
