# grove

Small, opinionated Git worktree management.

```sh
grove switch --create feature/login
grove switch feature/login
grove list
grove remove feature/login
```

`grove list` shows staged (`+`), modified (`!`), and untracked (`?`) work plus
the number of changed lines in each worktree.

Grove uses Git as its source of truth. It has no configuration, metadata, hooks,
or workflow steps. New worktrees are siblings of the primary worktree, such as
`project.feature%2Flogin`. Grove preserves lowercase ASCII letters, digits,
`-`, `_`, and `.`, and percent-encodes every other UTF-8 byte. This keeps branch
paths distinct even on case-insensitive or Unicode-normalizing filesystems.

## Shell setup

Add this to `.zshrc` so `switch` changes the current shell's directory:

```sh
eval "$(grove shell zsh)"
```

With the wrapper loaded, removing the current linked worktree returns you to the
primary worktree. Removal refuses primary, dirty, untracked, or locked worktrees
and keeps the local branch.

## Install

```sh
cargo install --path .
```
