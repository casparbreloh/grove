# Phase 1: Grove as the navigator

## Goal

Make bare `grove` the small, title-first interface for finding and acting on
active Changes. Replace the old switch and `--shell` workflows completely while
keeping Git inventory, direct native Pi sessions, and calling-shell navigation
as the authoritative seams.

## Interaction contract

```text
grove
grove new [--from REF]
grove list
grove sync
grove archive [--force]
grove init fish|zsh
```

The navigator uses the same active Change inventory as `grove list`:

- typing filters titles by case-insensitive substring;
- Up and Down move the selected Change;
- Enter launches or natively resumes Pi for the selected Change;
- Tab navigates the calling shell to the selected Change workspace;
- Home navigates the calling shell to Main;
- Ctrl-N creates a Change and launches Pi;
- Ctrl-T validates shell integration, creates a Change, and navigates to it;
- Escape and Ctrl-C cancel.

Main is a separate action rather than a selectable Change. Global actions remain
available when no Changes exist or no title matches. Shell navigation preserves
the invoking relative subdirectory when it exists in the destination and falls
back to the destination workspace root.

`grove switch`, `switch --shell`, and `new --shell` are removed without aliases,
deprecations, or compatibility behavior. The Fish and Zsh wrappers remain only
as the mechanism that applies Grove's navigation directive to the parent shell.

## Implementation

- `src/main.rs` owns the optional Clap subcommand, navigator event loop,
  adaptive table rendering, shell actions, archive picker, and navigation.
- `src/git.rs` remains the sole source of workspace inventory and Git facts.
- `src/session.rs` remains the narrow direct-Pi launch/resume boundary.
- The navigator uses a temporary terminal screen, restores terminal state on
  every exit path, and redraws from a stable origin after input or resize.
- Narrow rendering removes Path, divergence, Changes, and Base before
  truncating the title, and preserves a duplicate/Untitled Change ID suffix.
- Standalone `grove new --from REF` remains the explicit creation-base surface;
  navigator creation uses the default base.

## Tasks

- [x] Make bare `grove` open the navigator and remove `switch`/`--shell`.
- [x] Add incremental title filtering and the visible action vocabulary.
- [x] Separate Enter/Pi activation from calling-shell navigation.
- [x] Add Pi and shell creation actions, validating shell support before mutation.
- [x] Preserve equivalent relative subdirectories during shell navigation.
- [x] Make list and navigator rendering adaptive while preserving Change identity.
- [x] Rewrite compiled-CLI workflows and helpers around current behavior only.
- [x] Update README, VISION, and contributor guidance for the clean break.

## Checks

```sh
cargo build
cargo run -- --help
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
git diff --check
```

The compiled-CLI workflows use real disposable Git repositories and PTYs, real
Fish and Zsh wrappers, and only the fake external Pi seam. They cover filtering,
action separation, Pi resume, empty inventory, cancellation, non-TTY failure,
relative navigation and fallback, pre-mutation shell validation, adaptive
width, terminal restoration, and removal of obsolete help/completion surfaces.
