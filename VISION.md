# Grove Vision

Grove is an agent worktree manager: a small, AI-focused layer over Git that
makes isolated terminal-agent work easy to create, resume, inspect, and ship.
Git remains the source of truth. Grove should improve the workflow without
becoming a new version-control system or an agent framework.

## Principles

- Keep the common workflow obvious and the command surface small.
- Preserve stable change IDs while titles, commits, and pull requests evolve.
- Keep worktree navigation independent from agent launch.
- Use agents through their native interfaces instead of replacing their TUIs.
- Keep network activity explicit.
- Protect destructive operations and avoid complexity for unlikely cases.
- Prefer semantic agent capabilities over provider-specific flags throughout
  the application.

## Foundation

Grove currently provides ID-backed changes, worktree switching and listing,
lineage-aware removal, and persistent agent sessions. Pi is the default, with
Claude, Codex, and global custom commands supported. The embedded r/mux runtime
allows agents to detach and reattach without requiring another installed
multiplexer.

## Phase 3: AI-assisted commits

Add `grove commit` as the next narrow workflow:

- Generate a commit message from the current diff.
- Let the user review or edit the message before committing.
- Commit through Git and preserve normal Git behavior.
- Support built-in Pi, Claude, and Codex integrations plus custom agents.

Agent differences should stay behind a small capability boundary. Grove asks
for an interactive session or a one-shot generation; each preset owns the
corresponding argv and output behavior. Custom integrations configure those
same capabilities globally. This should make another CLI easy to add without
spreading provider checks through Grove or introducing a broad plugin system.

## Phase 4: Remote workflow

Close the path from a local change to review:

- `grove sync` explicitly fetches and synchronizes with the recorded base.
- `grove pr` pushes the backing branch and creates or updates a pull request.
- `grove list` shows concise remote, pull-request, and CI state.
- Ordinary pull requests come before stacked changes.

## Phase 5: Agent orchestration

Deepen session management without making Grove own the agent workflow:

- Show useful agent state alongside each worktree.
- Pick, resume, and intentionally create multiple sessions.
- Record native provider session IDs where supported.
- Support explicit local-to-cloud handoff.
- Keep plans, specifications, and execution policies user- and provider-owned.

r/mux 0.9 state and foreground-process APIs should be adopted once released
and when this phase needs them.

## Beyond: Analytics and automation

Optional observability can connect agent sessions, commits, reviews, and pull
request outcomes. It can reveal repeated requests, failure patterns, missing
skills, and opportunities to improve project instructions. Provider session
data and transcripts must remain opt-in; terminal process state alone cannot
capture intent.

User-defined automation may eventually carry an existing plan through
implementation, testing, review, and pull-request creation. Grove should
coordinate that lifecycle without prescribing how plans are written or which
agent performs each step.
