# Grove

Grove is a small Rust CLI layer over Git. Git remains the source of truth, and
the local Change/worktree workflow should stay simple. See `README.md` for
user-facing behavior, domain language, and invariants, and `VISION.md` for
direction.

Grove is fast-moving and pre-1.0. Prioritize common, high-impact workflows and
destructive safety boundaries; do not add complexity for merely hypothetical
edge cases.

## Layout

- `src/main.rs` owns Clap types and command dispatch only.
- `src/new.rs`, `src/sync.rs`, `src/ship.rs`, and `src/archive.rs` own their command flows; ship also owns publication branch, state validation, and the private `gh`/`glab` boundary.
- `src/navigator.rs` owns Change rows, the navigator, picker, terminal rendering, and calling-shell navigation shared with the archive flow.
- `src/init.rs` owns shell initialization.
- `src/change.rs` owns repository directories, immutable Change identity, minimal capsule records, titles, and lifecycle transitions.
- `src/git.rs` is the deep Git module. It owns creation lineage, path-based workspace inventory, integration detection, rollback, branch cleanup, and destructive validation; keep raw Git operations private there.
- `src/session.rs` is the narrow Pi adapter: validation, capsule lock, direct blocking launch/resume, extension materialization, and isolated structured workers.
- `src/extensions/` contains the flat Pi extension sources materialized by the session adapter.
- `src/shell.fish` and `src/shell.zsh` are thin calling-shell wrappers.
- `tests/new.rs`, `tests/sync.rs`, `tests/ship.rs`, `tests/archive.rs`, and `tests/navigator.rs` contain command-first compiled-CLI workflows matching `src/`; `tests/support/` owns real disposable repositories and minimal fake external seams.

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
- Fake only external process boundaries (`pi`, `gh`, `glab`, and SSH transport); keep their native contracts visible and never mock Git.
- Keep a minimal, potent suite that describes only the current pre-1.0 product. Fold changed behavior and enduring safety boundaries into the smallest coherent replacement workflows, replace obsolete tests and assertions, and do not retain regression or compatibility tests for removed behavior.
- Assert user-visible output plus Git and filesystem state, not private call structure.

## Conventions

- Use **Change**, **Change ID**, **Title**, and **Pi session** as defined in `README.md`; automatic Title-derived publication branches are Git refs, not Change identity or user input, and Pi sessions remain separate identities.
- Treat feature replacements as clean breaks: remove superseded commands, options, code paths, and tests in the same change. Do not add deprecations, aliases, or backward-compatibility surfaces unless explicitly requested.
- In the navigator, the agent is the obvious primary action. Keep Enter agent-first without styling the selected row differently from the others.
- Keep `main.rs` thin and deepen `change.rs`, `git.rs`, or `session.rs` around their existing authority.
- Let rustfmt and Clippy define mechanical Rust style.
- Return contextual `Result`s for recoverable Git, filesystem, Pi, and process failures. Reserve panics for genuine invariants and tests.
- Keep visibility and dependencies minimal; expose semantic operations rather than command plumbing.
- Use `///` when Clap consumes it as user-facing help. Avoid non-functional implementation and test comments.
- Validate before mutation. Preserve rollback, private modes, advisory locking, conservative integration detection, exact worktree/HEAD validation, compare-and-delete branch cleanup, and recoverable `closing` state. Keep destructive safety boundaries covered by coherent compiled-CLI workflows against real disposable Git repositories.
- Pi JSONL is agent-owned and must remain byte-for-byte untouched by Grove. Grove-owned records must be atomic and private.
- Do not add implicit network activity. The managed title request is the documented narrow exception initiated by starting Pi; all other remote effects belong to explicit commands.
- Do not migrate or delete pre-1.0 state implicitly.
