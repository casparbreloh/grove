# Runtime inspiration: OpenCode V2 and Pi Agent Harness

Research date: 2026-08-28. Scope: canonical repositories, live branches, package boundaries, and what Grove should keep under `.repos/` as read-only architectural reference.

## Recommendation

Keep these checkouts:

| Directory              | Repository                                  | Branch          | Role                                                                      |
| ---------------------- | ------------------------------------------- | --------------- | ------------------------------------------------------------------------- |
| `.repos/opencode-2`    | `https://github.com/anomalyco/opencode.git` | `dev`           | Current OpenCode V2 and client/server architecture                        |
| `.repos/pi-main`       | `https://github.com/earendil-works/pi.git`  | `main`          | Current Pi AI, harness, tools, session, client, server, and protocol work |
| `.repos/pi-harness-v2` | `https://github.com/earendil-works/pi.git`  | `harness-v2/j4` | Historical V2/J4 comparison only; it is no longer Pi's active future line |

The directory name `.repos/opencode-2` should not be interpreted as a request to check out OpenCode's literal `2.0` branch. That branch is an old exploration. The maintained V2 implementation is on `dev`.

These are reference clones, not source dependencies. Preserve their `.git` directories, keep `.repos/` ignored, and update each checkout with a fast-forward-only pull from its configured branch. Do not import upstream product nouns or make either project the owner of Grove state.

## OpenCode

### Repository and branch

The canonical repository is [`anomalyco/opencode`](https://github.com/anomalyco/opencode); the former `sst/opencode` URL redirects there. Its default branch is `dev`. At research time, `dev` was at [`df35e842`](https://github.com/anomalyco/opencode/commit/df35e842f59bc115bb7c0479a8e11f017d443f2c), committed 2026-08-28.

OpenCode's literal [`2.0`](https://github.com/anomalyco/opencode/tree/2.0) branch is not the current V2 line. Its tip is the April 13 commit [`7a6ce05`](https://github.com/anomalyco/opencode/commit/7a6ce05d0939826aa6c8e1c481489a713b2d633f), and it has 1 unique commit while `dev` has 1,157 commits beyond their common history. The associated first-party [“2.0 exploration” pull request](https://github.com/anomalyco/opencode/pull/22335) was merged into `dev` the same day. Current V2 fixes continued on `dev`, including the explicit June commit [“port v2 runtime fixes onto dev”](https://github.com/anomalyco/opencode/commit/93159bccbf32c63a9f4510780d940131c2b3438b).

### Current architecture

OpenCode's official server documentation states the essential pattern directly: starting OpenCode starts a server and a TUI client; the headless server exposes an OpenAPI 3.1 contract used to generate its SDK, which permits multiple clients. [`opencode serve` documentation](https://dev.opencode.ai/docs/server/)

The V2 source is a migration inside the same monorepo:

- [`packages/core`](https://github.com/anomalyco/opencode/tree/df35e842f59bc115bb7c0479a8e11f017d443f2c/packages/core) is intended to own domain schemas, typed errors, state containers, events, and plugin hook contracts. Provider/config/auth/model policy moves to plugins, while the legacy `packages/opencode` package becomes thinner. [V2 core instructions](https://github.com/anomalyco/opencode/blob/df35e842f59bc115bb7c0479a8e11f017d443f2c/specs/v2/instructions.md#L1-L14)
- [`packages/server`](https://github.com/anomalyco/opencode/tree/df35e842f59bc115bb7c0479a8e11f017d443f2c/packages/server) owns the Effect HTTP API and handlers. Its route assembly composes the same core services for authenticated network routes and unauthenticated embedded routes. [Route assembly](https://github.com/anomalyco/opencode/blob/df35e842f59bc115bb7c0479a8e11f017d443f2c/packages/server/src/routes.ts#L26-L62)
- [`packages/sdk-next`](https://github.com/anomalyco/opencode/tree/df35e842f59bc115bb7c0479a8e11f017d443f2c/packages/sdk-next) is an in-process host. It executes the server's assembled HTTP router without a listener or network I/O, preserving the same routing, middleware, codecs, handlers, and errors. Its own README calls it transitional, and the official V2 SDK documentation calls the preview beta. [SDK package contract](https://github.com/anomalyco/opencode/blob/df35e842f59bc115bb7c0479a8e11f017d443f2c/packages/sdk-next/README.md#L1-L16), [V2 SDK documentation](https://opencode.ai/v2/docs/build/sdk)
- [`packages/tui`](https://github.com/anomalyco/opencode/tree/df35e842f59bc115bb7c0479a8e11f017d443f2c/packages/tui) is being separated into presentation and local terminal behavior. Its specified OpenCode boundary is the SDK; server startup, authentication, transport, and config discovery stay with CLI hosts, while domain operations stay with server and SDK. [TUI package boundary](https://github.com/anomalyco/opencode/blob/df35e842f59bc115bb7c0479a8e11f017d443f2c/specs/tui-package.md#L16-L31), [ownership split](https://github.com/anomalyco/opencode/blob/df35e842f59bc115bb7c0479a8e11f017d443f2c/specs/tui-package.md#L54-L90)
- [`packages/app`](https://github.com/anomalyco/opencode/tree/df35e842f59bc115bb7c0479a8e11f017d443f2c/packages/app) is the shared graphical client; [`packages/desktop`](https://github.com/anomalyco/opencode/tree/df35e842f59bc115bb7c0479a8e11f017d443f2c/packages/desktop) embeds it. The Electron build bundles a server module, and the desktop sidecar starts that server with a loopback address, password, and renderer-specific CORS origin. [Desktop server bundle](https://github.com/anomalyco/opencode/blob/df35e842f59bc115bb7c0479a8e11f017d443f2c/packages/desktop/electron.vite.config.ts#L34-L80), [sidecar startup](https://github.com/anomalyco/opencode/blob/df35e842f59bc115bb7c0479a8e11f017d443f2c/packages/desktop/src/main/sidecar.ts#L51-L66)
- [`packages/cli`](https://github.com/anomalyco/opencode/tree/df35e842f59bc115bb7c0479a8e11f017d443f2c/packages/cli) supplies the newer process host and daemon lifecycle. `serve` binds the shared server routes; the daemon persists a private credential and discovers or starts a compatible local server. [Serve host](https://github.com/anomalyco/opencode/blob/df35e842f59bc115bb7c0479a8e11f017d443f2c/packages/cli/src/commands/handlers/serve.ts#L15-L45), [daemon contract](https://github.com/anomalyco/opencode/blob/df35e842f59bc115bb7c0479a8e11f017d443f2c/packages/cli/src/services/daemon.ts#L11-L21)

The migration is active, not a finished stable architecture. `packages/opencode` still contains legacy runtime, CLI, and compatibility code; the desktop's embedded server currently points at that package's Node build. Use `specs/v2`, the new packages, and current source together rather than assuming every client has completed the migration.

### What Grove should take from it

The strongest reusable idea is one client-facing contract with two executions: an embedded local path and a network path. A TUI or desktop client should depend on Grove's stable domain operations and events, not import runtime internals. The same handler semantics can be hosted in-process for local use and behind an authenticated transport for a development machine.

Grove should not copy OpenCode's domain model or make OpenAPI/HTTP the internal truth. Grove's Workspace → Project → Task → Session hierarchy, Agent boundary, Environment boundary, event journal, receipts, and mobility semantics remain authoritative. OpenCode is most useful for studying client separation, SDK generation, daemon discovery, server selection, and gradual package extraction.

## Pi

### Repository and current line

The canonical repository is now [`earendil-works/pi`](https://github.com/earendil-works/pi); former `badlogic/pi-mono` and `earendil-works/pi-mono` URLs redirect there. The default branch is `main`. At research time, `main` was at [`6c87d9a`](https://github.com/earendil-works/pi/commit/6c87d9a026677b601e8278030dcf1ad97fe0bd86), committed 2026-08-28; the latest release was [`v0.84.3`](https://github.com/earendil-works/pi/releases/tag/v0.84.3), published 2026-08-24. The first-party [Pi Licensing RFC](https://rfc.earendil.com/0015/) records Earendil's acquisition of Pi and Mario Zechner joining the company.

The current monorepo describes itself as “Pi Agent Harness” and separates reusable packages from the coding-agent product. [Repository README](https://github.com/earendil-works/pi/blob/6c87d9a026677b601e8278030dcf1ad97fe0bd86/README.md#L10-L30)

### Relevant package boundaries

- [`packages/ai`](https://github.com/earendil-works/pi/tree/6c87d9a026677b601e8278030dcf1ad97fe0bd86/packages/ai), `@earendil-works/pi-ai`, is the most direct dependency candidate. It owns unified model/provider streaming, catalogs, auth resolution, credentials, subscription OAuth implementations, token/cost accounting, and provider wire differences. It includes OpenAI Codex subscription OAuth, GitHub Copilot OAuth, Cloudflare AI Gateway, and Workers AI support. [Pi AI overview and provider list](https://github.com/earendil-works/pi/blob/6c87d9a026677b601e8278030dcf1ad97fe0bd86/packages/ai/README.md#L1-L5), [providers and OAuth surface](https://github.com/earendil-works/pi/blob/6c87d9a026677b601e8278030dcf1ad97fe0bd86/packages/ai/README.md#L49-L90), [provider ownership model](https://github.com/earendil-works/pi/blob/6c87d9a026677b601e8278030dcf1ad97fe0bd86/packages/ai/README.md#L230-L234)
- [`packages/agent`](https://github.com/earendil-works/pi/tree/6c87d9a026677b601e8278030dcf1ad97fe0bd86/packages/agent), `@earendil-works/pi-agent-core`, contains the agent loop, messages, tools, events, compaction, harness, sessions, and backend conformance tests. Its SQLite backend is intentionally a separate package so core does not pull in Node or native SQLite. [Agent package boundary](https://github.com/earendil-works/pi/blob/6c87d9a026677b601e8278030dcf1ad97fe0bd86/packages/agent/README.md#L1-L14)
- Pi's current harness design is a durable runtime around immutable conversation entries, mutable namespaced registers, an append-only usage ledger, atomic transactions, named lanes, a durable operation state, and intent → effect → settlement recovery. [Current harness orientation](https://github.com/earendil-works/pi/blob/6c87d9a026677b601e8278030dcf1ad97fe0bd86/packages/agent/docs/harness.md#L80-L137) It explicitly assumes one process per session and no replication. [Non-goals](https://github.com/earendil-works/pi/blob/6c87d9a026677b601e8278030dcf1ad97fe0bd86/packages/agent/docs/harness.md#L207-L214)
- Pi now injects a runtime-neutral `ExecutionEnv` into built-in tools. Its `FileSystem` and `Shell` capabilities return typed results and place the Node implementation behind a separate entry point. [Execution capability contract](https://github.com/earendil-works/pi/blob/6c87d9a026677b601e8278030dcf1ad97fe0bd86/packages/agent/src/harness/types.ts#L222-L315), [read tool injection](https://github.com/earendil-works/pi/blob/6c87d9a026677b601e8278030dcf1ad97fe0bd86/packages/agent/src/harness/tools/read.ts#L45-L55), [bash tool injection](https://github.com/earendil-works/pi/blob/6c87d9a026677b601e8278030dcf1ad97fe0bd86/packages/agent/src/harness/tools/bash.ts#L51-L68) This is directly relevant to Grove's local, development-machine, and Cloudflare Environment implementations.
- [`packages/protocol`](https://github.com/earendil-works/pi/tree/6c87d9a026677b601e8278030dcf1ad97fe0bd86/packages/protocol), [`packages/server`](https://github.com/earendil-works/pi/tree/6c87d9a026677b601e8278030dcf1ad97fe0bd86/packages/server), and [`packages/client`](https://github.com/earendil-works/pi/tree/6c87d9a026677b601e8278030dcf1ad97fe0bd86/packages/client) are experimental remote-session primitives. The protocol carries validated, length-prefixed CBOR and treats snapshots as authoritative and progress as transient. The server requires applications to provide the runtime/storage service, while the client is transport-neutral. [Protocol contract](https://github.com/earendil-works/pi/blob/6c87d9a026677b601e8278030dcf1ad97fe0bd86/packages/protocol/README.md#L1-L16), [server boundary](https://github.com/earendil-works/pi/blob/6c87d9a026677b601e8278030dcf1ad97fe0bd86/packages/server/README.md#L1-L48), [client boundary](https://github.com/earendil-works/pi/blob/6c87d9a026677b601e8278030dcf1ad97fe0bd86/packages/client/README.md#L1-L40)
- [`packages/coding-agent`](https://github.com/earendil-works/pi/tree/6c87d9a026677b601e8278030dcf1ad97fe0bd86/packages/coding-agent) is product composition, not the core Grove should adopt wholesale. Its current harness factory demonstrates wrapping Pi's injected execution capabilities with coding-agent prompt contributions and default read/bash/edit/write tools. [Coding-agent harness composition](https://github.com/earendil-works/pi/blob/6c87d9a026677b601e8278030dcf1ad97fe0bd86/packages/coding-agent/src/server/create-harness.ts#L80-L159)

For Grove, reuse `pi-ai` and as much `pi-agent-core` behavior as stays compatible with Grove-owned Task/Session state. Treat `ExecutionEnv` and tool factories as a useful capability seam, but bind them to Grove Environment operations rather than letting Pi own filesystem, process, credential, or persistence semantics. Study Pi's protocol/client/server work for development-machine transport, but do not expose it as Grove's public product model while it is experimental.

### The V2 branch is historical

The exact surviving branch is [`harness-v2/j4`](https://github.com/earendil-works/pi/tree/harness-v2/j4), with head [`f7f933c`](https://github.com/earendil-works/pi/commit/f7f933c6e0a127bd2b56336338512092fec0399d), dated 2026-08-07. It is currently 9 commits ahead and 256 behind `main`. `j4` is a specific JSONL v3-normalization work package, not the name of the complete V2 effort.

First-party commit history is clearer than the remembered social post:

1. Mario Zechner introduced the V2 design in [`4f0437e`](https://github.com/earendil-works/pi/commit/4f0437e2d58d651dd934119ecabea2893975f62f) on 2026-07-29.
2. He promoted the durable harness API in [`4428955`](https://github.com/earendil-works/pi/commit/44289550aa06750542c0ace8ab4bac0c7e68ce54) on 2026-08-04.
3. After the J4 branch diverged, he began a successor harness-v3 specification on `main` in [`7a6a1c2`](https://github.com/earendil-works/pi/commit/7a6a1c2dbb5ef07040bac7a2b1c6a589a4f41e56) and consolidated the authoritative design into `packages/agent/docs/harness.md` in [`85a2060`](https://github.com/earendil-works/pi/commit/85a2060811a23f1580c13ab59a210b1409092837), deleting the former V2 design documents.

No accessible first-party social post or maintainer issue was found that explicitly says to use `harness-v2/j4`. The repository branch, Mario's commits, and the design files are stronger primary evidence. Keep the second checkout only because a historical comparison was requested; label it historical so future agents do not mistake it for the current Pi architecture.

## Grove-specific takeaways

The two upstreams converge on several useful primitives:

1. A UI consumes a typed client/domain surface and never imports the runtime implementation.
2. Embedded local execution and authenticated remote execution preserve one semantic contract without requiring one physical transport.
3. Agent execution receives filesystem and process capabilities; tools do not select their own runtime.
4. Snapshots are authoritative; progress and streaming chunks are projections, not durable state.
5. External effects need explicit admission, cancellation, settlement, and ambiguous-outcome handling.
6. Provider/model/auth behavior belongs below the Grove Agent boundary; Project/Task/Session durability and Environment ownership remain Grove's responsibility.

These should inform Grove's primitives, but neither upstream should override `VISION.md`: Grove still has exactly two pluggable product boundaries, Agent and Environment, and its own durable domain remains the source of truth.
