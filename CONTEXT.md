# Grove domain context

## Glossary

- **Change** — Grove's durable unit of work, from creation through archival.
- **Change ID** — Grove-owned immutable 8-character lowercase hexadecimal identity, unique within its repository and used as its capsule directory name.
- **Title** — The Change's only human-facing name. The first successful inference initializes it once.
- **Pi session** — A native Pi conversation with a Pi-owned ID, JSONL file, and optional native name. A Change may contain multiple Pi sessions.
- **Repository directory** — `~/.grove/<repository-name>-<path-hash>/`, deterministically identified from the canonical Git common directory.
- **Capsule** — `<repository-directory>/<change-id>/`, containing `change.json`, the active `workspace/`, native `pi/` sessions, and two lock files.
- **Creation base** — The commit OID and optional parent branch recorded when the Change is created.
- **Workspace** — A Change's directory, implemented as a native Git worktree and created with detached HEAD.

## Invariants

- Git commits, worktree HEADs, and refs are authoritative for code history; `change.json` is authoritative only for minimal Grove metadata.
- Change ID, Title, Git branch names, and Pi session identity are independent. Titles never rename Git or move the capsule.
- Active Changes have one registered Git worktree at `<capsule>/workspace`. A branch may later be attached by the user or agent.
- Grove identifies a Change by its record and exact workspace path, never by a branch name.
- Managed Pi runs directly and synchronously with `<capsule>/pi`; there is no process persistence.
- Pi owns native JSONL. Grove only supplies the directory and appends a versioned Change link through the managed extension.
- One Change activity lock prevents concurrent managed Pi writers and archival while Pi is open; a separate metadata lock serializes atomic `change.json` updates.
- Archival validates the exact worktree HEAD and integration target before mutation. Ordinary archival refuses dirty or unintegrated work; `--force` discards it irreversibly.
- Successful archival retains the capsule, native sessions, and tracking branches; it removes the workspace and compare-and-deletes an attached local branch only when that branch has no configured upstream. It transitions `active` → `closing` → `archived`.
- Capsule directories and Grove-owned records are private local data.
- Grove never migrates or deletes pre-1.0 state implicitly.
