# Grove domain context

## Glossary

- **Change** — Grove's durable unit of work, from creation through archival.
- **Change ID** — Grove-owned immutable 32-character lowercase hexadecimal identity. It is also the exact local Git branch and capsule directory name, but is normally hidden from users.
- **Title** — The Change's only human-facing name. The first successful inference initializes it once.
- **Pi session** — A native Pi conversation with a Pi-owned ID, JSONL file, and optional native name. A Change may contain multiple Pi sessions.
- **Capsule** — `~/.grove/<repository-key>/<change-id>/`, containing all local Change state.
- **Creation base** — The ref, commit OID, and optional parent branch recorded when the Change is created.
- **Archive** — The durable base-to-final patch and statistics captured before worktree and branch deletion.

## Invariants

- Git commits and refs are authoritative for code history; `change.json` is authoritative only for Grove metadata.
- Change ID, Title, and Pi session identity are independent. Titles never rename Git or move the capsule.
- Active Changes have one ID branch and one worktree at `<capsule>/worktree`.
- Managed Pi runs directly and synchronously with `<capsule>/sessions/pi`; there is no process persistence.
- Pi owns native JSONL. Grove only supplies the directory and appends a versioned Change link through the managed extension.
- One advisory lock prevents concurrent managed Pi writers and any removal while Pi is open.
- Removal archives before Git mutation. Validation or artifact failure leaves the branch, worktree, and real index untouched.
- A successful removal retains the capsule and native sessions, deletes only the worktree and ID branch, and transitions `active` → `closing` → `archived`.
- Capsule directories and Grove-owned records/artifacts are private local data.
- Grove never migrates or deletes state created by older pre-1.0 models automatically.
