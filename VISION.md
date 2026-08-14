# Grove

Grove is a local-first, native desktop GUI for coding agents.

It helps people work with an agent in a workspace: start and resume sessions, follow messages and tool activity, and approve consequential actions. Workspace content, transcripts, and credentials stay on the machine unless the user explicitly chooses otherwise.

## Approach

- **Pi-first, not Pi-only.** Pi is the first supported runtime. Grove's UI and local data model use stable agent concepts—sessions, events, tools, approvals, errors, and cancellation—not provider-specific ones.
- **Adapters at the boundary.** An adapter translates an agent's protocol into Grove's small, versioned capability and event contract. Other agents can be added when they can support a useful local experience; they are not forced through Pi.
- **Extensible by design.** A small trusted core owns workspaces, local persistence, permissions, and process boundaries. Built-in features and explicitly trusted local plugins can add tools, commands, integrations, and eventually UI. A marketplace and arbitrary plugin loading are not early goals.
- **Native and inspectable.** Use Electron and platform capabilities where they are the clearest choice. The renderer is unprivileged; filesystem, processes, credentials, and agent lifecycle stay behind narrow, validated IPC.
- **Simple by default.** Prefer one excellent local workflow over broad provider parity, hidden automation, cloud services, or speculative abstractions.

## First useful release

A person can select a local workspace, start or resume a Pi session, stream readable agent activity, approve or deny consequential tool actions, cancel work, understand failures, and retain useful local history.

## Non-goals for now

Cloud sync, accounts, remote multi-user control, unattended destructive work, a plugin marketplace, a full IDE, and multi-agent orchestration.
