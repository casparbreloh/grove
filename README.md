# grove

Small, opinionated Git worktree management.

```sh
grove switch --create feature/login
grove switch feature/login
grove list
grove remove feature/login
```

`grove list` shows `●` when a worktree has changes and `×` when it has conflicts.
Grove deliberately does not expose Git's staging model.

Grove uses Git as its source of truth. It has no configuration, metadata, hooks,
or workflow steps. New worktrees live under `~/.grove/<repo>/<branch>`. Grove
percent-encodes branch names so their paths remain distinct on case-insensitive
and Unicode-normalizing filesystems.

## Shell setup

For Fish, add these lines in this order:

```fish
COMPLETE=fish grove | source
grove shell fish | source
```

For Zsh:

```sh
source <(COMPLETE=zsh grove)
eval "$(grove shell zsh)"
```

With the wrapper loaded, switching changes directory without printing a second
machine-readable path. New worktrees live under `~/.grove/<repo>/<branch>`.

Removal deletes both the worktree and its local branch when the branch is merged.
It refuses dirty or unmerged work. `grove remove --force` explicitly discards both.

## Install

```sh
cargo install --path .
```
