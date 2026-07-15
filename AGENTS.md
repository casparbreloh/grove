# Grove

Grove is a small Rust CLI layer over Git. Git remains the source of truth, and
the local branch/worktree workflow should stay simple. See `README.md` for
user-facing behavior and roadmap.

Grove is fast-moving. Prioritize common, high-impact workflows and destructive
safety boundaries; do not add complexity for merely hypothetical edge cases.

## Layout

- `src/main.rs` owns Clap definitions, rendering, shell integration, and navigation.
- `src/git.rs` is the deep Git module. It owns workflows, lineage, rollback, and safety policy; keep raw Git operations private there.
- `src/session.rs` owns Pi session launch, the embedded ZMX runtime, stable identity, extraction, attachment, and termination; `src/pi-extension.ts` captures the first interactive prompt without delaying Pi.
- `src/shell.fish` and `src/shell.zsh` are thin calling-shell wrappers.
- `tests/cli.rs` contains compiled-CLI workflows; `tests/support/mod.rs` owns isolated disposable repositories.

## Commands

```sh
cargo build
cargo run -- --help
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
git diff --check
```

Safe squash detection requires Git 2.38 or newer.

## Testing

- For behavior changes, invoke the `tdd` skill when available. In every environment, observe RED at the compiled CLI seam, implement the smallest GREEN change, then refactor with the suite green.
- Exercise the real binary against real disposable Git repositories. Do not mock Git.
- Prefer a few coherent workflows and semantic assertions over tests per helper or coverage target.
- Assert user-visible output plus Git and filesystem state, not private call structure.

## Conventions

- Keep `main.rs` thin and deepen `src/git.rs` rather than exposing raw Git helpers.
- Let rustfmt and Clippy define mechanical Rust style.
- Return contextual `Result`s for recoverable Git, filesystem, and process failures. Reserve panics for genuine invariants and tests.
- Keep visibility and dependencies minimal; expose semantic Git operations rather than command plumbing.
- Use `///` when Clap consumes it as user-facing help. Avoid non-functional implementation and test comments.
- Validate before mutation. Preserve rollback, conservative removal, compare-and-delete behavior, and lineage cleanup; destructive changes require a real-Git regression test.
- Do not add implicit network activity. Remote effects must belong to an explicit command.
