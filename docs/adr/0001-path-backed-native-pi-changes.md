# ADR 0001: Path-backed Changes with native Pi resume

- Status: Accepted
- Date: 2026-07-16

## Context

Grove should make isolated Pi work easy without becoming a version-control
system, session runtime, or archive format. Git remains responsible for code
history and Pi remains responsible for conversation history.

## Decision

Each Change receives an immutable random 8-hex ID used only for its capsule.
The capsule lives at:

```text
~/.grove/<repository-name>-<path-hash>/<change-id>/
```

It contains only:

```text
change.json
workspace/          # while active
pi/
.activity.lock
.metadata.lock
```

The repository path is deterministic from the readable repository name and a
8-hex digest of its canonical Git common directory. There is no repository
registry or manifest.

`workspace/` is a native Git worktree created with detached HEAD. Grove finds a
Change by its exact capsule path, not by a branch. If a user or agent attaches a
branch, archival compare-and-deletes it only when it has no configured upstream;
tracking branches are preserved.

`change.json` stores only identity, title, state, creation base and parent, plus
an archival timestamp and outcome. Detailed recovery facts exist only while the
record is in `closing` state.

Grove launches Pi directly with `<capsule>/pi` as its native session directory.
The managed extension is materialized as a private temporary file for that Pi
process and removed afterward. Grove has no persistent runtime directory.

Archival stores no source snapshot or statistics. Ordinary archival requires
clean integrated work. `archive --force` explicitly and irreversibly discards
unintegrated and dirty workspace content. Native Pi JSONL remains untouched.

## Consequences

- The local layout stays small, readable, and suitable for both local and cloud
  workspaces.
- Git is the only durable source-history mechanism; users create or retain a
  tracking branch when commit history must survive workspace removal.
- Pi sessions survive archival independently of source history.
- Activity and metadata locks remain separate because metadata updates occur
  while Pi holds the activity lock.
- This is a pre-1.0 clean break. Grove contains no compatibility or implicit
  migration path.
