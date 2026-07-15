# grove

Grove is a small, Pi-first worktree manager. Git remains the source of truth.
Each linked worktree has one persistent native Pi session, while the primary
checkout stays where it already is.

```sh
grove new auth-refresh
# work in Pi, then detach with Ctrl+\

grove switch auth-refresh
grove list
grove remove auth-refresh
```

See [VISION.md](VISION.md) for the product direction.

## Creating worktrees

```text
grove new [OPTIONS] [BRANCH]
```

With a branch, `new` creates its worktree and opens Pi:

```sh
grove new auth-refresh
grove new --from release backport-auth
```

Without a branch, Grove opens Pi immediately in a detached pending worktree.
A bundled Pi extension turns the first typed prompt into a lowercase,
hyphenated branch name in the background and renames the worktree directory to
match. There is no fallback branch and no extra model call.

Detaching before the first prompt preserves the pending session, so Grove never
submits or discards unfinished editor input. It remains available as
`(pending)` in `grove switch`. Exiting Pi before the first prompt removes an
untouched pending worktree; changed worktrees are preserved.

`--from` accepts any revision that resolves to a commit. `--from @` starts at
the invoking worktree's current commit and records its branch as the parent
when possible. Without `--from`, Grove starts from the detected default branch.

Use `--shell` to create and enter a named worktree without Pi:

```sh
grove new --shell auth-refresh
```

Managed worktrees live at `~/.grove/<repo>-<digest>/<branch>`. Equal repository
directory names cannot collide.

## Switching

```text
grove switch [OPTIONS] [BRANCH]
```

`switch` opens the worktree's sole Pi session. A live session is reattached;
otherwise Pi resumes from the same session file. Without a branch, Grove shows
an interactive picker, including unnamed pending sessions.

After Pi detaches, the calling shell returns to the repository's primary
checkout. `grove switch main` also means that primary checkout, even when it
currently has another branch checked out; Grove never creates a linked
worktree for `main`.

Use `grove switch --shell <branch>` to enter a worktree directly.

## Sessions

Grove embeds [ZMX](https://github.com/neurosnap/zmx), so ZMX, tmux, and rmux do
not need to be installed. Pi keeps its native TUI. Press `Ctrl+\` once to
detach; `Ctrl+C` remains available to Pi.

Closing a terminal detaches its client while Pi continues in the background.
Closing a Mac laptop normally suspends the whole machine: the session survives
and resumes after wake, but it cannot keep computing during sleep. Continuous
lid-closed work requires an awake clamshell setup or an always-on remote host.

Grove currently supports one Pi session per worktree. There is no agent
configuration or multiplexer UI.

## Listing and removal

`grove list` shows branch, base, uncommitted line changes, divergence, and path.
In `Base↕`, `↑` means commits ahead and `↓` means commits behind.

```text
grove remove [--force] [BRANCH]
```

`grove delete` is an alias of `grove remove`.

Without a branch, `remove` targets the current linked worktree. Safe removal
refuses dirty or genuinely unmerged work and follows the recorded creation
base. It accepts work integrated by merge, rebase, cherry-pick, or an equivalent
squash. `--force` explicitly discards changes and deletes an unmerged branch.

A live Pi session protects its worktree from safe removal. Forced removal stops
only that worktree's session before changing Git state. Safe squash detection
requires Git 2.38 or newer.

## Shell setup

The wrapper lets Grove change the calling shell's directory and adds
completions. It does not edit shell configuration.

For Fish:

```fish
grove init fish | source
```

For Zsh:

```sh
eval "$(grove init zsh)"
```

## Install

Pi must be available as `pi` on `PATH`. Grove carries the session runtime.

```sh
cargo install --path .
```
