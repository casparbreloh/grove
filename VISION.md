# Grove Vision

Grove is an agent-first worktree manager: a small layer over Git that makes
isolated terminal-agent work easy to create, return to, inspect, and ship. Git
remains the source of truth. Grove should improve the workflow without becoming
a version-control system or an agent framework.

## Principles

- Keep the common workflow obvious and the command surface small.
- Use the branch as the workspace identity.
- Give each worktree one persistent native agent session.
- Make `new` and `switch` agent-first, with `--shell` as the explicit escape.
- Use agents through their native interfaces instead of replacing their TUIs.
- Keep network activity explicit.
- Protect destructive operations and avoid complexity for unlikely cases.

## Foundation

Grove provides branch-backed worktrees, agent-first switching, lineage-aware
removal, and one persistent agent session per worktree. Pi is the default, with
Claude Code, Codex, and global custom commands supported. For the built-in
agents, bare `grove new` derives the branch from the first typed prompt without
inventing a fallback name. The embedded rmux runtime provides persistence
without another multiplexer installation.

## Next: shipping a branch

Close the path from local work to review while keeping every remote effect
explicit:

- Generate a commit message from the current diff, with review before commit.
- Fetch and synchronize with the recorded base through an explicit command.
- Push the branch and create or update a pull request through an explicit
  command.
- Show concise remote, pull-request, and CI state in `grove list`.

Agent differences should remain behind a small capability boundary. Grove asks
for a persistent interactive session or a narrow one-shot result; each built-in
preset owns the provider-specific details.

## Later

Only deepen session management when real use demonstrates a need. Useful agent
state and explicit local-to-cloud handoff may earn their place. Multiple
sessions, orchestration, analytics, and automation should not enter the core
workflow merely because they are possible.
