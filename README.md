# grove

Grove is a small, Pi-first layer over Git worktrees. Git remains the source of
truth. Grove adds a durable local **Change** around each task so that creating,
leaving, finding, resuming, and safely archiving Pi work stays simple.

```sh
grove new
# work in Pi, then exit Pi normally

grove switch   # pick the Change by title and resume Pi
grove list
grove archive  # archive the current Change, or pick one from the primary checkout
```

See [VISION.md](VISION.md) for the product direction and [CONTEXT.md](CONTEXT.md)
for the domain vocabulary.

## Commands

```text
grove new [--from REF] [--shell]
grove switch [--shell]
grove list
grove sync
grove archive [--force]
grove init fish|zsh
```

`new` creates an untitled Change and opens Pi. It takes no name or branch
argument. `--from` accepts any revision that resolves to a commit; `--from @`
uses the invoking worktree's current commit. Without `--from`, Grove starts at
the repository's detected default branch.

`switch` always opens a terminal picker containing active Grove Changes. From a
Change it also offers the main repository first; selecting main returns the
calling shell there without opening Pi. `list` always includes main without
counting it as a Change. The picker and `list` lead with each Change's stable
inferred title. Until naming succeeds, Grove shows `Untitled`; duplicate and
untitled rows include a short opaque ID only to disambiguate them. Use the arrow
keys and press Enter to select. Ordinary, detached, and otherwise unmanaged Git
worktrees are not included.

`sync` is an explicit network operation that must run from the primary worktree
and requires its current branch to have a configured upstream. It quietly
fetches exactly the configured merge ref into that upstream-tracking ref; it
does not fetch or prune unrelated refs, and it does not move the local primary
branch. Among clean active Changes recorded with the primary branch as their
creation parent, it archives Changes already integrated upstream through the
same safe archive-before-delete path and rebases eligible linear Changes onto
the fetched upstream. Rebase rewrites Change commits. Grove conservatively
skips Changes whose creation base is absent from either fetched upstream or the
Change tip, Changes with merge history, failed rebases, dirty or busy Changes
(including Changes with an active Pi process), Git-locked or missing worktrees,
and Changes created from another parent branch. Sync is a best-effort batch:
skipped Changes remain untouched, while completed archives and rebases are not
rolled back if a later operation fails. It reports one ordered outcome and
reason for every active Change, followed by archived, rebased, and skipped
totals.

`archive` targets the current managed Change. From the primary checkout it opens
the same picker. Safe archival accepts work integrated by merge, cherry-pick or
rebase-shaped history, or an equivalent squash. It refuses dirty or genuinely
unmerged work, including unique content hidden in a merge resolution. `--force`
explicitly and irreversibly discards that work. Both paths delete an attached local
branch without a configured upstream; tracking branches are preserved.

`--shell` skips Pi and writes a navigation directive instead. It is useful for
creating or entering a Change with a normal shell. `new --shell`, `switch
--shell`, and archival from the current Change are the only operations that
request parent-shell navigation. Selecting the main repository in ordinary
`switch` also navigates without opening Pi. After managed Pi exits, the caller
stays in the directory where it invoked Grove.

## Native Pi sessions

Managed Pi is a direct, blocking child process:

```text
pi --session-dir <capsule>/pi --continue --extension <temporary-grove-extension>
```

There is no multiplexer, background server, detach/reattach protocol, or live
terminal persistence. Closing Pi or its terminal ends that process. Pi's native
JSONL session remains in the Change capsule, and a later picker selection runs
Pi with the same `--session-dir --continue` arguments so it resumes
automatically. You never need to copy or remember a Pi session ID.

Grove holds a per-Change advisory lock while Pi is open. A second managed Pi or
archival, including forced archival, refuses to proceed until the first process
exits. Starting `pi` manually is unmanaged: Grove does not install its extension
globally or discover arbitrary sessions.

Pi owns its native session files and session names. Grove's extension appends a
small versioned link from each managed Pi session to its Change. The first
substantial prompt in each unnamed native session also starts a fire-and-forget,
isolated `pi --print` request to infer a three- or four-word session name. The
first successful result initializes the Change's one stable title; later Pi
sessions may receive different native names but never retitle the Change or
rename Git.

The naming request uses `--no-session --no-tools --no-context-files --no-skills
--no-extensions`. It does not delay the real turn, and malformed output or any
failure leaves an honest `Untitled` fallback. It is still an additional request
to Pi's configured provider: the first prompt is sent a second time and may
incur provider cost. Treat that prompt according to the provider's privacy
terms.

## Change identity and storage

Each Change has one immutable Grove-owned 8-character lowercase hexadecimal ID,
unique within its repository and used only for its capsule identity. Grove
creates `workspace/` as a native Git worktree with detached HEAD and finds it by
its exact capsule path, not by a branch name. Native detached commits are
supported. If the user or an agent later creates a branch, Grove may rebase its
checked-out commits during explicit `sync`. Archival deletes that local branch
only when it has no configured upstream; a tracking branch is preserved. The
human Title and Pi session IDs remain separate identities.

Everything local to a Change lives together:

```text
~/.grove/
  <repository-name>-<path-hash>/
    <change-id>/
      change.json
      .activity.lock
      .metadata.lock
      workspace/          # active Changes only
      pi/
        <Pi-native session>.jsonl
```

The repository directory combines its readable name with a 8-hex digest of
the canonical Git common directory; there is no repository registry. The
minimal `change.json` records identity, title, state, creation base and parent,
plus archival time and outcome. Detailed recovery facts exist only while the
record is `closing`. Pi JSONL remains the canonical conversation, usage, and
tool history.

Grove stores no source snapshot or statistics. Successful archival removes
`workspace/` and any attached local branch without a configured upstream.
Tracking branches, the record, and Pi sessions remain. A registered detached
worktree keeps commits reachable while active; after archival, unbranched
source history is intentionally gone.

The two empty lock files have separate purposes: `.activity.lock` excludes a
second managed Pi and archival while Pi is open; `.metadata.lock` serializes
atomic `change.json` updates. They contain no persistent state.

Capsules are local-only and private (`0700` directories and `0600` Grove-owned
records on Unix). They can contain source, prompts, tool output, and
secrets. Beyond the documented title request, Grove performs no implicit
network activity.

This is a pre-1.0 clean break. Grove accepts only the current Change record
version and does not contain runtime migration or compatibility paths. Existing
local capsules can be converted once with explicit Git and filesystem commands;
Pi sessions remain untouched.

## Shell setup

The wrapper lets Grove change the calling shell's directory and supplies
command/flag completion. It does not edit shell configuration and does not
complete Change titles as positional arguments because `switch` and primary
`archive` use the picker. Add the appropriate line to your shell configuration
so it loads in every terminal; `--shell` fails before mutation when the wrapper
is not loaded.

For Fish, add this to `~/.config/fish/config.fish`:

```fish
grove init fish | source
```

For Zsh, add this to `~/.zshrc`:

```sh
eval "$(grove init zsh)"
```

## Install

Git 2.38 or newer and `pi` on `PATH` are required for the full workflow.

```sh
cargo install --path .
```
