# Phase 1 navigator refinement

## Goal

Make bare `grove` the only inventory: a rich, inline, title-first selector that
keeps the agent as the obvious primary action without looking like a full-screen
application or exposing a large key vocabulary.

## Interaction contract

```text
grove
grove new [--from REF]
grove sync
grove archive [--force]
grove init fish|zsh
```

The navigator is one ordered selection model:

1. Main, pinned first and selected initially;
2. active Changes.

A Change's primary action launches or resumes Pi, its secondary action navigates
the calling shell, and Main navigates to the primary worktree. Change creation
remains explicit through `grove new`. Current keybindings stay in the README
rather than navigator chrome or a new command.

`grove switch`, `grove list`, and every `--shell` option are removed without
aliases or compatibility behavior.

## Presentation

- Render transiently in the normal terminal flow, never the alternate screen.
- Clear Grove's inline region before launching Pi, navigating, or cancelling.
- Do not render an in-UI key legend.
- Use the terminal's default foreground for the header and every row.
- Mark selection only with `›`; do not emphasize the selected row.
- Use no foreground-color palette and honor `NO_COLOR` and `TERM=dumb`.
- Show Base, Changes, Base↕, and Path when width permits; drop secondary columns
  before truncating title/Change identity.

Shell navigation preserves the invoking relative subdirectory when it exists in
the destination and otherwise falls back to the destination workspace root.

## Tasks

- [x] Replace the old full-screen key model with the ordered inline selector.
- [x] Pin Main before active Changes and keep creation out of the picker.
- [x] Keep selection arrow-only with no filter mode.
- [x] Make Enter agent-primary and Tab the consistent shell alternative.
- [x] Restore the plain picker styling without a persistent legend.
- [x] Remove `grove list` and its obsolete tests/helpers/docs.
- [x] Preserve rich adaptive Git facts and relative shell navigation.

## Checks

```sh
cargo build
cargo run -- --help
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
git diff --check
```
