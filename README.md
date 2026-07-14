# grove

Grove is a small worktree and Git manager for terminal coding agents. Git stays
the source of truth, while Grove gives each piece of work an isolated worktree:

```sh
grove new "Add passkey login"
grove switch
grove list
grove remove c-a13f7c45b829
```

## Switching and creating

```text
grove switch [change-id-or-branch]
grove new [--from <ref>] [task]
```

`switch` only navigates to a worktree. Without an argument it shows a small
interactive picker; an exact change ID or ordinary branch selects directly.
It never launches an agent, so inspecting a diff does not begin a session.

`new` creates an immutable ID branch such as `c-a13f7c45b829` and starts the
configured agent in its worktree. An optional task becomes the change title and
is passed to the agent:

```sh
grove new
grove new "Add passkey login"
```

Without a task, the agent opens normally with its own native input. Grove does
not render an editor or manage the agent session. The ID remains stable even as
the title evolves and can later serve as the backing branch for a pull request.

Without `--from`, the new branch starts at the repository's detected default
branch. `--from` accepts any revision that resolves to a commit, including a
local branch, remote-tracking branch, tag, commit expression, or commit ID:

```sh
grove new --from release "Backport the login fix"
grove new --from 'main~2' "Investigate the regression"
```

`--from @` starts at the invoking worktree's current branch, or its current
commit when detached:

```sh
grove new --from @ "Follow up on this change"
```

New worktrees live at `~/.grove/<repo>-<digest>/<change-id>`. The digest
identifies the Git repository, so repositories with the same directory name
cannot collide. Grove still discovers ordinary Git branches and worktrees.

## Agents and configuration

Grove reads project configuration from `grove.toml`, then global configuration
from `~/.config/grove/grove.toml` or `$XDG_CONFIG_HOME/grove/grove.toml`, then
uses built-in defaults. Pi is the default agent; `claude`, `claude-code`, and
`codex` are also built in:

```toml
agent = "codex"
```

Custom agents are defined in the global config because commands from a checked-in
project file would be unsafe. They use argument arrays and replace `{prompt}` as
one argument. Grove does not run these values through a shell:

```toml
agent = "opencode"

[agents.opencode]
command = ["opencode", "{prompt}"]
```

`{prompt}` is omitted when `grove new` has no task. If the placeholder is not
present, Grove appends a supplied task as one argument.

## Listing

`grove list` shows each change's title, stable ID, base, uncommitted line
changes, divergence from its base, and path. Untitled changes are shown as
`(untitled)`. Ordinary Git worktrees remain visible using their branch name. In
`Base↕`, `↑` means commits ahead of the base and `↓` means commits behind it.

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

## Install

```sh
cargo install --path .
```
