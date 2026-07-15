# grove

Grove is a small worktree manager for terminal coding agents. Git remains the
source of truth. Each branch gets an isolated worktree and one persistent agent
session.

```sh
grove new auth-refresh
# work in the agent, then detach with Ctrl-b d

grove switch
grove list
grove remove auth-refresh
```

See [VISION.md](VISION.md) for the product direction.

## Creating worktrees

```text
grove new [OPTIONS] [BRANCH]
```

With a branch, `new` creates the branch and its worktree, then opens that
worktree's agent:

```sh
grove new auth-refresh
grove new --from release backport-auth
```

Without a branch, Grove opens the configured agent in a pending detached
worktree. The first typed prompt becomes a lowercase, hyphenated branch name.
There is no random fallback. Automatic naming supports Pi, Claude Code, and
Codex through their native session files.

Detaching before the first prompt cancels an untouched pending worktree. If the
agent has already changed files, or naming fails, Grove preserves the worktree
and reports its path instead of discarding work.

`--from` accepts any revision that resolves to a commit. `--from @` starts at
the invoking worktree's current branch, or at its current commit when detached.
Without `--from`, Grove starts from the detected default branch.

Use `--shell` when you only want the worktree:

```sh
grove new --shell auth-refresh
```

A branch is required with `new --shell`, because there is no agent prompt from
which to infer one.

Managed worktrees live at `~/.grove/<repo>-<digest>/<branch>`. The repository
digest prevents equal directory names from colliding. A worktree created by
bare `grove new` keeps its temporary directory after the branch is inferred;
the branch remains its public identity.

## Switching

```text
grove switch [OPTIONS] [BRANCH]
```

`switch` opens the worktree's sole agent session. If that session already
exists it is reused. Without a branch, Grove shows a picker that includes the
current worktree. Use Up and Down, then Enter.

To enter a worktree without its agent:

```sh
grove switch --shell auth-refresh
```

Grove embeds rmux, so nothing else needs to be installed. Detach from the agent
with the standard tmux binding, `Ctrl-b d`. After detaching, the shell wrapper
moves the calling shell into that worktree. Running `grove switch` again is all
that is needed to return to the agent.

## Agents

Pi is the default. Claude Code and Codex are built in. Select one per project in
`grove.toml`:

```toml
agent = "codex"
```

The project setting overrides `~/.config/grove/grove.toml` or
`$XDG_CONFIG_HOME/grove/grove.toml`. Custom commands are configured globally:

```toml
agent = "opencode"

[agents.opencode]
command = ["opencode", "--mode", "agent"]
```

Custom agents work with explicitly named branches. Bare `grove new` rejects
them because Grove cannot reliably observe their first prompt. Command
arguments, including spaces and empty values, are passed directly without a
shell. `{prompt}` has no special meaning.

## Listing and removal

`grove list` shows branch, base, uncommitted line changes, divergence from the
base, and path. In `Base↕`, `↑` means commits ahead and `↓` means commits behind.

```text
grove remove [--force] [BRANCH]
```

`grove delete` is an alias of `grove remove`.

Without a branch, `remove` targets the current linked worktree. Safe removal
refuses dirty or genuinely unmerged work and follows the recorded creation
base. It accepts work integrated by merge, rebase, cherry-pick, or an equivalent
squash. `--force` explicitly discards changes and deletes an unmerged branch.

A live agent protects its worktree from safe removal. Forced removal stops that
worktree's session before changing Git state. Safe squash detection requires
Git 2.38 or newer.

## Shell setup

The wrapper lets `new` and `switch` change the calling shell's directory and
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
