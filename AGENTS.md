# Grove

Grove is a small Rust CLI layer over Git. Git remains the source of truth, and
the local Change/worktree workflow should stay simple. See `README.md` for
user-facing behavior, `VISION.md` for direction, and `CONTEXT.md` for the domain
language and invariants.

Grove is fast-moving and pre-1.0. Prioritize common, high-impact workflows and
destructive safety boundaries; do not add complexity for merely hypothetical
edge cases.

## Layout

- `src/main.rs` owns Clap definitions, rendering, picker/shell integration, and navigation.
- `src/change.rs` owns repository directories, immutable Change identity, minimal capsule records, titles, and lifecycle transitions.
- `src/git.rs` is the deep Git module. It owns creation lineage, path-based workspace inventory, integration detection, rollback, branch cleanup, and destructive validation; keep raw Git operations private there.
- `src/session.rs` is the narrow Pi adapter: validation, capsule lock, direct blocking launch/resume, extension materialization, and isolated title inference.
- `src/pi-extension.ts` links native Pi sessions and starts fire-and-forget naming without delaying Pi.
- `src/shell.fish` and `src/shell.zsh` are thin calling-shell wrappers.
- `tests/cli.rs` contains coherent compiled-CLI workflows; `tests/support/` owns real disposable repositories and the fake external Pi seam.

## Commands

```sh
cargo build
cargo run -- --help
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
git diff --check
```

Safe squash detection requires Git 2.38 or newer. The extension contract test
requires Node.js, matching Pi's runtime.

## Testing

- For behavior changes, invoke the `tdd` skill when available. In every environment, observe RED at the compiled CLI seam, implement the smallest GREEN change, then refactor with the suite green.
- Exercise the real binary against real disposable Git repositories. Do not mock Git.
- Fake only the external `pi` executable; keep its native session shape and launch contract visible.
- Keep a minimal, potent suite that describes the current pre-1.0 product. Fold changed behavior into the smallest coherent workflow, replace obsolete assertions, and do not accumulate tests per bug, helper, or historical regression.
- Assert user-visible output plus Git and filesystem state, not private call structure.

## Conventions

- Use **Change**, **Change ID**, **Title**, and **Pi session** as defined in `CONTEXT.md`; do not reintroduce a semantic branch name or Grove session identity.
- Keep `main.rs` thin and deepen `change.rs`, `git.rs`, or `session.rs` around their existing authority.
- Let rustfmt and Clippy define mechanical Rust style.
- Return contextual `Result`s for recoverable Git, filesystem, Pi, and process failures. Reserve panics for genuine invariants and tests.
- Keep visibility and dependencies minimal; expose semantic operations rather than command plumbing.
- Use `///` when Clap consumes it as user-facing help. Avoid non-functional implementation and test comments.
- Validate before mutation. Preserve rollback, private modes, advisory locking, conservative integration detection, exact worktree/HEAD validation, compare-and-delete branch cleanup, and recoverable `closing` state. Keep destructive safety boundaries covered by coherent compiled-CLI workflows against real disposable Git repositories.
- Pi JSONL is agent-owned and must remain byte-for-byte untouched by Grove. Grove-owned records must be atomic and private.
- Do not add implicit network activity. The managed title request is the documented narrow exception initiated by starting Pi; all other remote effects belong to explicit commands.
- Do not migrate or delete pre-1.0 state implicitly.
