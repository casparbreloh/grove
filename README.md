# grove

Grove is a small branch-and-worktree layer above Git. It keeps Git as the
source of truth and makes the common worktree workflow short:

```sh
grove switch --create feature/login
grove switch feature/login
grove list
grove remove feature/login
```

Phase 1 is deliberately local and simple. Grove does not yet provide AI,
`grove.toml` configuration, hooks, network synchronization, or pull-request
commands.

## Switching and creating

```text
grove switch <branch>
grove switch --create [--from <ref>] <branch>
```

`switch` reuses an existing worktree or creates one for an existing local
branch. `--create` creates both the branch and its worktree. Without `--from`,
the new branch starts at the repository's detected default branch. `--from`
accepts any revision that resolves to a commit, including a local branch,
remote-tracking branch, tag, commit expression, or commit ID:

```sh
grove switch --create --from release feature/backport
grove switch --create --from 'main~2' investigate/regression
```

`--from @` starts at the invoking worktree's current branch, or its current
commit when detached:

```sh
grove switch --create --from @ feature/follow-up
```

New worktrees live at
`~/.grove/<repo>-<digest>/<percent-encoded-branch>`. The digest identifies the
Git repository, so repositories with the same directory name cannot collide.
Grove still discovers existing worktrees through Git, including worktrees made
by hand or by an older Grove path layout.

## Listing

`grove list` shows each worktree's branch, base, uncommitted line changes,
divergence from its base, and path. In `Base↕`, `↑` means commits ahead of the
base and `↓` means commits behind it.

For a branch created from a local branch, Grove follows that parent while it
still contains the original creation point. Other bases remain fixed at their
recorded creation commit. Older branches without Grove lineage use the detected
default branch. If recorded lineage is incomplete or malformed, the row says
`invalid metadata`; listing continues, but safe removal refuses that branch.

## Removing

```text
grove remove [branch]
grove remove --force [branch]
```

With no branch argument, Grove removes the current linked worktree. Safe
removal refuses a dirty worktree and checks the branch against its recorded
lineage, falling back to the detected default branch for older branches or a
rewritten/missing local parent. It accepts changes integrated by a merge,
rebase or cherry-pick, as well as squash-equivalent changes. Genuine unmerged
work is refused. `--force` is the explicit escape hatch that discards changes
and deletes the branch.

Safe squash detection requires Git 2.38 or newer for
`git merge-tree --write-tree`.

## Shell setup

The shell wrapper lets `grove switch` change the calling shell's directory and
adds completions. It does not edit shell configuration.

For Fish:

```fish
grove init fish | source
```

For Zsh:

```sh
eval "$(grove init zsh)"
```

## Direction

Later phases will keep ordinary Git branch names while allowing a task prompt
to infer the name, launch a configured coding agent (especially Pi), and
generate commits. Remote synchronization and GitHub pull-request workflows come
after that. These are direction, not available commands or configuration yet.

## Install

```sh
cargo install --path .
```
