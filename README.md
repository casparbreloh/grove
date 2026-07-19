# grove

Grove is a small, Pi-first layer over Git worktrees. Git remains the source of
truth. Grove adds a durable local **Change** around each task so that creating,
leaving, finding, resuming, and safely removing Pi work stays simple.

```sh
grove new
# work in Pi, then exit Pi normally

grove switch   # pick the Change by title and resume Pi
grove list
grove remove   # remove the current Change, or pick one from the primary checkout
```

See [VISION.md](VISION.md) for the product direction and [CONTEXT.md](CONTEXT.md)
for the domain vocabulary.

## Commands

```text
grove new [--from REF] [--shell]
grove switch [--shell]
grove list
grove sync
grove remove [--force]
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
and requires its current branch to have a configured upstream. It fetches and
prunes that upstream's remote, without moving the local primary branch. Among
clean active Changes recorded with the primary branch as their creation parent,
it archives Changes already integrated upstream through the same safe
archive-before-delete path and rebases the rest onto the fetched upstream.
Rebase rewrites Change commits. Before integration detection or rebasing,
Grove conservatively preserves and skips a Change if its recorded creation base
is no longer in the Change's history. It also preserves rather than rebases
non-integrated Changes whose work history contains a merge, and it aborts and
skips a conflicting linear rebase. It also skips dirty or busy Changes
(including Changes with an active Pi process), Git-locked or missing worktrees,
and Changes created from another parent branch.

`remove` targets the current managed Change. From the primary checkout it opens
the same picker. Safe removal accepts work integrated by merge, cherry-pick or
rebase-shaped history, or an equivalent squash. It refuses dirty or genuinely
unmerged work, including unique content hidden in a merge resolution. `--force`
explicitly archives and discards that work. `delete` is an alias for `remove`.

`--shell` skips Pi and writes a navigation directive instead. It is useful for
creating or entering a Change with a normal shell. `new --shell`, `switch
--shell`, and removal from the current Change are the only operations that
request parent-shell navigation. Selecting the main repository in ordinary
`switch` also navigates without opening Pi. After managed Pi exits, the caller
stays in the directory where it invoked Grove.

## Native Pi sessions

Managed Pi is a direct, blocking child process:

```text
pi --session-dir <capsule>/sessions/pi --continue --extension <grove-extension>
```

There is no multiplexer, background server, detach/reattach protocol, or live
terminal persistence. Closing Pi or its terminal ends that process. Pi's native
JSONL session remains in the Change capsule, and a later picker selection runs
Pi with the same `--session-dir --continue` arguments so it resumes
automatically. You never need to copy or remember a Pi session ID.

Grove holds a per-Change advisory lock while Pi is open. A second managed Pi or
removal, including forced removal, refuses to proceed until the first process
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
unique within its repository. Git uses that exact ID as a local branch name, but the normal UI hides it.
The human title and Pi session IDs are separate identities; neither renames the
branch or capsule.

Everything local to a Change lives together:

```text
~/.grove/
  repositories/
    <repository-name>-<path-hash>/
      repository.json
      <change-id>/
        change.json
        .activity.lock
        .metadata.lock
        worktree/
        sessions/pi/
          <Pi-native session>.jsonl
        artifacts/
          change.patch
          stats.json
  runtime/
```

Repository directories always combine the readable repository name with a
short hash of the canonical Git common directory. This makes repository claims
deterministic without a global lock. `repository.json` records that canonical
directory, while `change.json` records Grove-owned identity, creation lineage, stable title, and
the `active` → `closing` → `archived` lifecycle. Pi JSONL is the canonical
conversation, usage, and tool history. Before deletion, Grove snapshots the
non-ignored base-to-final worktree as a binary-capable patch and machine-
readable statistics. Successful removal deletes only `worktree/` and the local
ID branch; the record, Pi sessions, and artifacts remain for later inspection
or analytics.

The two empty lock files have separate purposes: `.activity.lock` excludes a
second managed Pi and removal while Pi is open; `.metadata.lock` serializes
atomic `change.json` updates. They contain no persistent state.

Capsules are local-only and private (`0700` directories and `0600` Grove-owned
records/artifacts on Unix). They can contain source, prompts, tool output, and
secrets. Beyond the documented title request, Grove performs no implicit
network activity.

This is a pre-1.0 clean break. Grove does not migrate or delete worktrees,
branch metadata, session-runtime data, or other state created by earlier
versions. Retain or remove that old data manually.

## Shell setup

The wrapper lets Grove change the calling shell's directory and supplies
command/flag completion. It does not edit shell configuration and does not
complete Change titles as positional arguments because `switch` and primary
`remove` use the picker. Add the appropriate line to your shell configuration
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
