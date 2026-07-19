# Grove Vision

Grove is a Pi-first Change manager: a small layer over Git that makes isolated
agent work easy to create, leave, find, resume, inspect, and ship. Git remains
the source of truth. Grove should improve the workflow without becoming a
version-control system, agent framework, or terminal multiplexer.

## Principles

- Keep the common workflow and command surface small.
- Give every Change an immutable opaque identity and one stable human title.
- Keep titles, Git identity, and agent-native session identity separate.
- Use Pi through its native TUI and native JSONL instead of replacing either.
- Prefer native resume over owning background process or terminal persistence.
- Keep remote and other provider activity explicit and documented.
- Protect destructive operations and preserve useful local history.
- Add complexity only when common, demonstrated workflows require it.

## Foundation

Grove provides ID-backed Git worktrees, title-based list and picker navigation,
direct Pi launch/resume, recorded creation lineage, archive-before-delete
safety, and explicit upstream synchronization. A Change's hidden
repository-scoped 8-hex ID is also its durable local Git branch. Its capsule
groups the record, worktree, Pi-native sessions, and final Git artifacts
beneath one `~/.grove` path.

Bare `grove new` creates the complete Change before starting Pi. A small managed
extension links each native Pi session and makes one isolated, best-effort title
request from its first prompt. Naming never blocks the real turn, moves a path,
or renames Git. Pi itself owns the session transcript and resume behavior.

Grove deliberately does not keep Pi running after its terminal closes. There is
no daemon, PTY host, detach key, or multiplexer. `grove switch` starts Pi again
against the same native session directory, which is the simpler persistence the
product actually needs.

`grove sync` explicitly fetches the primary branch's upstream while leaving the
local primary branch in place. It archives clean integrated Changes, rebases
other eligible clean Changes onto the fetched upstream, and conservatively
skips Changes that cannot be synchronized safely. This is the local foundation
for eventually shipping a Change.

## Next: shipping a Change

Close the path from local work to review while keeping every remote effect
explicit:

- Generate a commit message from the archived or current diff, with review.
- Give an opaque local branch an appropriate publication name if hosting needs
  one.
- Push and create or update a pull request through an explicit command.
- Show concise remote, pull-request, and CI state without weakening the local
  Change model.

Pi remains the primary agent. Supporting another agent should require clear
user demand and a similarly small, trustworthy native seam.

## Later

The capsule and independent identities keep analytics and local-to-cloud
handoff possible without committing Grove to them now. Multiple agents,
orchestration, dashboards, uploads, cloud sandboxes, and live process
persistence should enter the core only after concrete use proves their value.
