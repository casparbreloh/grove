# grove

Grove is a small worktree and Git manager for terminal coding agents. Git stays
the source of truth, while Grove gives each piece of work an isolated worktree:

```sh
grove switch --create "Add passkey login"
grove agent
grove switch
grove list
grove remove c-a13f7c45b829
```

See [VISION.md](VISION.md) for the product direction and upcoming phases.

## Switching and creating

```text
grove switch [change-id-or-branch]
grove switch --create [--from <ref>] [title]
```

`switch` only navigates to a worktree. Without an argument it shows the same
worktree details as `list`; move the `›` cursor with Up and Down, then press
Enter. An exact change ID or ordinary branch selects directly.
It never launches an agent, so inspecting a diff does not begin a session.

`switch --create` creates an immutable ID branch such as `c-a13f7c45b829` and
navigates to its worktree. An optional title describes the change:

```sh
grove switch --create
grove switch --create "Add passkey login"
```

Without a title, Grove records an untitled change. Creation never launches an
agent or opens the picker. The ID remains stable even as the title evolves and
can later serve as the backing branch for a pull request.

Without `--from`, the new branch starts at the repository's detected default
branch. `--from` accepts any revision that resolves to a commit, including a
local branch, remote-tracking branch, tag, commit expression, or commit ID:

```sh
grove switch --create --from release "Backport the login fix"
grove switch --create --from 'main~2' "Investigate the regression"
```

`--from @` starts at the invoking worktree's current branch, or its current
commit when detached:

```sh
grove switch --create --from @ "Follow up on this change"
```

New worktrees live at `~/.grove/<repo>-<digest>/<change-id>`. The digest
identifies the Git repository, so repositories with the same directory name
cannot collide. Grove still discovers ordinary Git branches and worktrees.

## Agents

```text
grove agent [agent-name]
```

`agent` opens a persistent terminal session in the current worktree. Detach
with `Ctrl-b d`; running the same command again reattaches. Different agent
names have separate sessions. Grove embeds rmux, so there is no multiplexer to
install or run separately.

Pi is the default. `claude`, `claude-code`, and `codex` are also built in. A
project can select one in `grove.toml`:

```toml
agent = "codex"
```

The project setting overrides the global setting in
`~/.config/grove/grove.toml` or `$XDG_CONFIG_HOME/grove/grove.toml`. Custom
commands are global-only and use a direct argument array rather than a shell:

```toml
agent = "opencode"

[agents.opencode]
command = ["opencode", "--mode", "agent"]
```

Arguments, including spaces and empty values, are passed unchanged. The old
`{prompt}` placeholder is no longer supported because Grove does not capture
or forward agent prompts.

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
and deletes the branch. A live Grove agent session protects its worktree from
safe removal. Forced removal stops every agent session in the worktree before
changing Git state.

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
