# grove

Grove is a small, Pi-first layer over Git worktrees. Git remains the source of
truth. Grove adds a durable local **Change** around each task so that creating,
leaving, finding, resuming, and safely archiving Pi work stays simple.

```sh
grove new
# work in Pi, then exit Pi normally

grove          # find a Change, resume Pi, or navigate workspaces
grove ship     # have Pi commit, push, and open or update a pull request
grove archive  # archive the current Change, or pick one from the primary checkout
```

See [VISION.md](VISION.md) for the product direction.

## Commands

```text
grove
grove new [--from REF]
grove sync
grove ship
grove archive [--force]
grove init fish|zsh
```

`new` creates an untitled Change and opens Pi. It takes no name or branch
argument. `--from` accepts any revision that resolves to a commit; `--from @`
uses the invoking worktree's current commit. Without `--from`, Grove starts at
the repository's detected default branch.

Bare `grove` opens a transient inline navigator. Main is pinned first and active
Changes follow it. Main is selected initially and the arrow keys move the
selection. Enter launches or natively resumes Pi for a Change, while Tab
navigates the calling shell to it. Main navigates to the primary worktree with
either key. Escape or Ctrl-C closes the navigator. Change creation remains
explicit through `grove new`.

The navigator uses the terminal's default foreground throughout, with a plain
`›` selection marker, a bold header, and no selected-row emphasis or in-UI key
legend. The current keybindings are documented above. Styling honors `NO_COLOR`
and `TERM=dumb`. The Changes column reports tracked changed lines, untracked
files as `?N`, and conflicts. Until naming succeeds, Grove shows `Untitled`;
duplicate and untitled rows include a short opaque ID only to disambiguate them.
Ordinary, detached, and otherwise unmanaged Git worktrees are excluded. On narrow
terminals secondary columns disappear from the right before Change identity is
truncated.

`sync` is an explicit network operation that must run from the primary worktree
and requires its current branch to have a configured upstream. It quietly
fetches exactly the configured merge ref into that upstream-tracking ref, then
fast-forwards the local primary branch to it; it does not fetch or prune
unrelated refs and refuses divergent primary history or an unsafe worktree
update. Among clean active Changes recorded with the primary branch as their
creation parent, it archives Changes already integrated upstream through the
same safe archive-before-delete path and rebases eligible linear Changes onto
the fetched upstream. Rebase rewrites Change commits. Grove conservatively
skips Changes whose creation base is absent from either fetched upstream or the
Change tip, Changes with merge history, failed rebases, dirty or busy Changes
(including Changes with an active Pi process), Git-locked or missing worktrees,
non-integrated publication branches, and Changes created from another parent
branch. Sync is a best-effort batch:
skipped Changes remain untouched, while completed archives and rebases are not
rolled back if a later operation fails. It reports one ordered outcome and
reason for every active Change, followed by archived, rebased, and skipped
totals.

`ship` runs only from the current managed Change. Grove refuses unresolved Git
operations, untitled Changes, missing push remotes, local-only remotes,
unsupported hosts, and unavailable code-host access before publication. GitHub
uses `gh`; GitLab uses `glab`.

Grove creates or reuses the Change's stable `grove/<change-id>` publication
branch, stages the complete Change, and starts one isolated GPT-5.6 Sol JSON
worker for structured
commit and pull-request metadata. The worker receives a compact file index,
statistics, commit subjects, current pull-request metadata, and a bounded
zero-context patch. It may selectively read files when that context is
insufficient, but it cannot mutate the workspace. Grove deterministically
commits without a body, pushes without rewriting published history, and creates
or conditionally updates the pull request. New publication uses the pull-request title as the
initial commit subject; later work receives an incremental subject. Grove
rechecks existing pull-request metadata immediately before replacing it and
aborts when it observes a concurrent edit. Failures may leave staged work, a
commit, branch, or push in place; rerunning converges from that Git and code-host
state. On success, stdout contains only the pull-request URL. The Change remains
active through review.

`archive` targets the current managed Change. From the primary checkout it opens
the same picker. Safe archival accepts work integrated by merge, cherry-pick or
rebase-shaped history, or an equivalent squash. It refuses dirty or genuinely
unmerged work, including unique content hidden in a merge resolution. `--force`
explicitly and irreversibly discards that work, but never overrides a Git
worktree lock. Both paths delete an attached local branch without a configured
upstream; tracking branches are preserved.

Navigator shell actions write a navigation directive for the calling shell.
They preserve the current relative subdirectory when that directory exists in
the destination, otherwise they enter the destination root. Archiving the
current Change also navigates to Main. After managed Pi exits, the caller stays
in the directory where it invoked Grove.

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

Grove holds one per-Change advisory activity lock while Pi is open. It prevents
a second managed Pi, synchronization, or archival from mutating the Change at
the same time. It does not block an explicit `grove ship`; shipping validates
and pushes an exact Git snapshot, so it can run from inside Pi. Starting `pi`
manually is unmanaged: Grove does not install its extension globally or
discover arbitrary sessions.

Pi owns its native session files and session names. Grove's Change-session
extension appends a small `grove.change` link from each managed Pi session to its
Change. The first substantial prompt in each unnamed native session also starts
a fire-and-forget, isolated Pi JSON request with structured output to infer a
three- or four-word title. The first successful result initializes the Change's
one stable Title and becomes that Pi session's native name; later Pi sessions
may receive different native names but never retitle the Change or rename Git.

The naming request uses GPT-5.6 Sol with only Grove's structured-output tool
and no session, context files, skills, prompt templates, or other extensions. It
does not delay the real turn. Grove retries transient or malformed failures
with brief backoff, stops after three attempts for the native session, and
warns when naming still fails, leaving an honest `Untitled` fallback. Each
attempt starts an additional worker invocation against the OpenAI Codex provider
and may consume tokens. Grove starts at most three naming workers for
a native session; Pi and the provider may perform their own request retries
within each worker. Treat the prompt according to the provider's privacy terms.

## Change identity and storage

Each Change has one immutable Grove-owned 8-character lowercase hexadecimal ID,
unique within its repository. It names the capsule and its `grove/<change-id>`
publication branch. Grove
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

The repository directory combines its readable name with an 8-character
hexadecimal digest of the canonical Git common directory; there is no repository
registry. The minimal `change.json` records identity, title, state, creation base and parent,
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
complete Change titles as positional arguments because the navigator and
primary `archive` are interactive. Add the appropriate line to your shell
configuration so it loads in every terminal; shell navigation fails when the
wrapper is not loaded.

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
Shipping and managed title inference require access to GPT-5.6 Sol. Shipping
also requires a network push remote and suitable authenticated hosting tools
such as `gh` for GitHub or `glab` for GitLab.

```sh
cargo install --path .
```
