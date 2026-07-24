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

1. Main, pinned first;
2. active Changes;
3. New Change, pinned last.

- Up and Down move the selection.
- Enter on a Change launches or natively resumes Pi.
- Tab on a Change navigates the calling shell to it.
- Enter or Tab on Main navigates the calling shell to Main.
- Enter on New Change creates a Change and launches Pi.
- Tab on New Change validates shell support, creates a Change, and navigates.
- Escape and Ctrl-C close the navigator.

The agent is the primary action. The legend below the picker is contextual and
concise: Change and New Change rows show `↑↓ move`, `enter agent`, `tab shell`,
and `esc close`; Main shows `enter shell` instead of agent language.

`grove switch`, `grove list`, and every `--shell` option are removed without
aliases or compatibility behavior.

## Presentation

- Render transiently in the normal terminal flow, never the alternate screen.
- Clear Grove's inline region before launching Pi, navigating, or cancelling.
- Keep the picker contiguous, with its legend directly below the rows.
- Use the terminal's default foreground for the header and every row.
- Mark selection only with `›`; do not emphasize the selected row.
- Mute only the legend and indent it to align with the Title column.
- Use no foreground-color palette and honor `NO_COLOR` and `TERM=dumb`.
- Show Base, Changes, Base↕, and Path when width permits; drop secondary columns
  before truncating title/Change identity.

Shell navigation preserves the invoking relative subdirectory when it exists in
the destination and otherwise falls back to the destination workspace root.
Shell-based creation validates the calling-shell directive before mutation.

## Tasks

- [x] Replace the old full-screen key model with the ordered inline selector.
- [x] Pin Main and New Change around active Changes.
- [x] Keep selection arrow-only with no filter mode.
- [x] Make Enter agent-primary and Tab the consistent shell alternative.
- [x] Restore the plain picker styling and mute only its legend.
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
