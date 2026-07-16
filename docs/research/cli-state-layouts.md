# Repository-scoped state patterns in developer CLIs

Research date: 2026-07-16

## Question

How should Grove arrange repository-scoped Change capsules below `~/.grove`
while keeping the common path readable, same-name repositories safe, native Pi
sessions intact, and later analytics straightforward?

## Findings

### Codex

Codex uses one global `CODEX_HOME` (normally `~/.codex`) rather than one folder
per repository. Canonical rollout JSONL is date-sharded below `sessions/`, while
an append-only session-name index and SQLite state support discovery. Repository
association is metadata: a thread records its CWD, branch, Git SHA, and origin
URL. Bare `codex resume` opens a picker normally filtered to the exact CWD;
search matches human name, preview, UUID, branch, and CWD.

Source: OpenAI Codex
[`9ff47868`](https://github.com/openai/codex/tree/9ff47868eb2afeec579183e01bb9d3d3e9df2bcd),
[`utils/home-dir/src/lib.rs`](https://github.com/openai/codex/blob/9ff47868eb2afeec579183e01bb9d3d3e9df2bcd/utils/home-dir/src/lib.rs#L5-L17),
[`rollout/src/recorder.rs`](https://github.com/openai/codex/blob/9ff47868eb2afeec579183e01bb9d3d3e9df2bcd/rollout/src/recorder.rs#L1524-L1547),
[`state/src/model/thread_metadata.rs`](https://github.com/openai/codex/blob/9ff47868eb2afeec579183e01bb9d3d3e9df2bcd/state/src/model/thread_metadata.rs#L76-L130),
and
[`tui/src/resume_picker.rs`](https://github.com/openai/codex/blob/9ff47868eb2afeec579183e01bb9d3d3e9df2bcd/tui/src/resume_picker.rs#L835-L889).

Lesson: a title is presentation and an immutable ID is identity. Search can be
rich without making the title part of a filesystem path.

### Claude Code

Claude Code stores transcripts globally but groups them by an encoded absolute
working-directory path:
`~/.claude/projects/<encoded-working-directory>/<session-id>.jsonl`. Its resume
picker is the ordinary interaction, explicit resume accepts an ID or name, and
discovery understands worktrees belonging to the current project. Claude's own
managed worktrees use the repository-local `.claude/worktrees/` directory.

Sources: Anthropic's
[session documentation](https://code.claude.com/docs/en/sessions),
[CLI reference](https://code.claude.com/docs/en/cli-reference), and
[worktree documentation](https://code.claude.com/docs/en/worktrees).

Lesson: project grouping is valuable, but a repository basename alone is not
a safe identity. Encoding the full path avoids collisions but produces poor
human and analytics paths.

### Worktrunk

Worktrunk puts shared repository state below the Git common directory,
normally `.git/wt/`, so all linked worktrees naturally share it. Worktree paths
are configurable and may use a central `~/worktrees/<repo>/<branch>` hierarchy.
Where Worktrunk needs a stable project key, it prefers normalized
`host/owner/repo` remote identity and falls back to the canonical local path.

Source: Worktrunk
[`52dce937`](https://github.com/max-sixty/worktrunk/tree/52dce93760b74fc6187913500d33c9225370687a),
[`src/git/repository/mod.rs`](https://github.com/max-sixty/worktrunk/blob/52dce93760b74fc6187913500d33c9225370687a/src/git/repository/mod.rs#L1081-L1103),
[`src/git/repository/remotes.rs`](https://github.com/max-sixty/worktrunk/blob/52dce93760b74fc6187913500d33c9225370687a/src/git/repository/remotes.rs#L381-L415),
and
[`skills/worktrunk/reference/config.md`](https://github.com/max-sixty/worktrunk/blob/52dce93760b74fc6187913500d33c9225370687a/skills/worktrunk/reference/config.md#L87-L131).

Lesson: repository name is a useful label, while remote identity or canonical
path is the collision-safe key. Durable state and disposable caches should not
be mixed.

### Entire

Entire keeps active session state under the Git common directory and permanent
checkpoint data in dedicated Git refs. Session records retain both the absolute
worktree path and Git's internal worktree ID. Permanent checkpoint data is
structured around analytics fields while retaining native agent session bytes
and references.

Source: Entire CLI
[`df765ab9`](https://github.com/entireio/cli/tree/df765ab952185595d65f561f8ccb8036598980a8),
[`docs/architecture/sessions-and-checkpoints.md`](https://github.com/entireio/cli/blob/df765ab952185595d65f561f8ccb8036598980a8/docs/architecture/sessions-and-checkpoints.md#L61-L76),
[`cmd/entire/cli/session/state.go`](https://github.com/entireio/cli/blob/df765ab952185595d65f561f8ccb8036598980a8/cmd/entire/cli/session/state.go#L119-L142),
and
[`api/checkpoint/metadata.go`](https://github.com/entireio/cli/blob/df765ab952185595d65f561f8ccb8036598980a8/api/checkpoint/metadata.go#L101-L147).

Lesson: the Change capsule is the right Grove analytics boundary. Pi sessions
should remain nested native inputs, with normalized Grove records and artifacts
beside them.

### Graphite, Jujutsu, and GitHub CLI

Graphite stores repository configuration in `.git/.graphite_repo_config`, while
user preferences live under `~/.config/graphite`. Jujutsu likewise makes the
workspace own a pointer to shared repository state under `.jj`. GitHub CLI
normally identifies repositories as `host/owner/name`, inferred from remotes,
and keeps almost no repository-local metadata of its own.

Sources: Graphite's
[quick start](https://graphite.com/docs/cli-quick-start#initializing-graphite)
and [configuration guide](https://graphite.com/docs/configure-cli), Jujutsu
[`30c5071a`](https://github.com/jj-vcs/jj/tree/30c5071a87d252f092b26afa02a352c2ae346788)
[`lib/src/workspace.rs`](https://github.com/jj-vcs/jj/blob/30c5071a87d252f092b26afa02a352c2ae346788/lib/src/workspace.rs#L291-L336),
and GitHub CLI
[`c14cbaa2`](https://github.com/cli/cli/tree/c14cbaa24a75272958161751240fd538a68e6c04)
[`pkg/cmdutil/repo_override.go`](https://github.com/cli/cli/blob/c14cbaa24a75272958161751240fd538a68e6c04/pkg/cmdutil/repo_override.go#L36-L84).

Lesson: repository-local ownership eliminates basename collisions, while
remote-derived identity is excellent when every repository has a canonical
remote. Grove has local-only Changes and wants one inspectable home, so neither
pattern should be copied directly.

## Three refined Grove layouts

### 1. Name first, suffix only on a real collision — recommended

```text
~/.grove/
  repositories/
    grove/
      repository.json
      7fd31a8c/
    grove-a13f92c1/       # only if another unrelated `grove` exists
      repository.json
      2c80e941/
  runtime/
```

`repository.json` records the version, human repository name, canonical Git
common directory, and normalized remote when available. Grove reuses a folder
whose identity matches. It claims the exact name when free and uses
`<name>-<short-repository-key>` only when that name is already owned by a
different local repository.

This gives the requested exact repository-name folder in the normal case,
preserves isolation, and keeps the hierarchy shallow. It requires a small,
atomic repository-folder claim operation but no global database.

### 2. Remote hierarchy with a local fallback

```text
~/.grove/repositories/github.com/casparbreloh/grove/7fd31a8c/
~/.grove/repositories/local/grove-a13f92c1/7fd31a8c/
```

This follows Worktrunk and GitHub CLI's normalized remote identity. It is very
safe and useful for cross-machine analytics, but it is deeper, remote-centric,
and ambiguous when two local clones of the same remote should remain separate.

### 3. Git-owned repository identity with central capsules

```text
<git-common-dir>/grove/repository-id
~/.grove/repositories/grove/<repository-id>/7fd31a8c/
```

This follows Graphite, Worktrunk, Entire, and Jujutsu by letting the repository
own its identity. It handles moves and same-name repositories cleanly, but adds
an identity level and splits Grove ownership between Git metadata and
`~/.grove`.

## Recommendation

Use layout 1. Keep eight lowercase hexadecimal characters for a Change ID,
reserve the capsule atomically, and reject or regenerate on a capsule or branch
collision. Treat `(repository identity, Change ID)` as the durable compound key;
never treat the eight-character Change ID as globally unique.

Keep `grove switch` and primary-checkout `grove remove` argument-free. Add only
incremental title filtering inside their shared picker. This follows the
Codex/Claude/Graphite pattern of making the picker ordinary while avoiding a
second selector interface and its completion, quoting, and ambiguity rules.
