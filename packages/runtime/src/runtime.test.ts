import assert from "node:assert/strict";
import test from "node:test";
import { Effect } from "effect";

import type {
  AgentRunResult,
  AgentSessionCapabilities,
  AgentSessionService,
  AgentSessionSink,
} from "./agent-session.ts";
import { createDirectGroveClient } from "./runtime.ts";
import type { ModelSummary, ThinkingLevel } from "./types.ts";

const models = [
  model("openai-codex", "gpt-5.6-sol", "GPT-5.6 Sol"),
  model("anthropic", "claude-sonnet-4-6", "Claude Sonnet 4.6"),
];

test("sync exposes authoritative session state and model commands update it", async () => {
  const client = createDirectGroveClient(new ScriptedAgentSession(models));
  const initial = await client.sync();

  assert.equal(initial.task.environment.kind, "local");
  assert.equal(initial.session.phase.type, "idle");
  assert.deepEqual(initial.models, models);
  assert.deepEqual(initial.session.model, models[0]?.ref);

  const result = await client.execute({
    type: "session.select-model",
    commandId: "command-model",
    sessionId: initial.session.id,
    model: models[1]!.ref,
  });

  assert.equal(result.ok, true);
  assert.deepEqual((await client.sync()).session.model, models[1]?.ref);
  await client.close();
});

test("prompt publishes transient progress and settles an authoritative transcript", async () => {
  const client = createDirectGroveClient(new ScriptedAgentSession(models));
  const initial = await client.sync();
  const iterator = client.watch({ after: initial.cursor })[Symbol.asyncIterator]();

  const execution = client.execute({
    type: "session.prompt",
    commandId: "command-prompt",
    sessionId: initial.session.id,
    text: "Say hello",
  });

  const first = await iterator.next();
  const second = await iterator.next();
  const third = await iterator.next();
  const result = await execution;

  assert.equal(first.value?.kind, "event");
  assert.equal(second.value?.kind, "progress");
  assert.equal(third.value?.kind, "event");
  assert.equal(result.ok, true);
  assert.equal(result.type, "session.prompt");
  if (result.ok && result.type === "session.prompt") assert.equal(result.status, "accepted");

  const settled = await client.sync();
  assert.equal(settled.session.phase.type, "idle");
  assert.deepEqual(
    settled.session.messages.map((message) => message.role),
    ["user", "assistant"],
  );
  assert.equal(settled.session.messages[1]?.parts[0]?.type, "text");
  assert.equal(settled.session.messages[1]?.parts[0]?.text, "Hello from the scripted agent.");

  await iterator.return?.();
  await client.close();
});

test("a second prompt is rejected while the session is running and abort settles the first", async () => {
  const agent = new ScriptedAgentSession(models, true);
  const client = createDirectGroveClient(agent);
  const { session } = await client.sync();

  const first = await client.execute({
    type: "session.prompt",
    commandId: "command-first",
    sessionId: session.id,
    text: "Wait",
  });
  assert.equal(first.ok, true);
  if (first.ok && first.type === "session.prompt") assert.equal(first.status, "accepted");

  const second = await client.execute({
    type: "session.prompt",
    commandId: "command-second",
    sessionId: session.id,
    text: "Overlap",
  });

  assert.deepEqual(second, {
    ok: false,
    type: "session.prompt",
    commandId: "command-second",
    error: { code: "busy", message: "Session already has an active Turn" },
  });

  const aborted = await client.execute({
    type: "session.abort",
    commandId: "command-abort",
    sessionId: session.id,
  });
  assert.equal(aborted.ok, true);

  await waitForIdle(client);
  assert.equal((await client.sync()).session.lastTurn?.outcome, "aborted");
  await client.close();
});

test("redelivering a command ID returns its receipt without running the command twice", async () => {
  const agent = new ScriptedAgentSession(models);
  const client = createDirectGroveClient(agent);
  const { session } = await client.sync();
  const command = {
    type: "session.prompt" as const,
    commandId: "command-retried",
    sessionId: session.id,
    text: "Only once",
  };

  const [first, retried] = await Promise.all([client.execute(command), client.execute(command)]);

  assert.deepEqual(retried, first);
  assert.equal(agent.runCount, 1);
  await waitForIdle(client);
  assert.equal((await client.sync()).session.messages.length, 2);
  await client.close();
});

test("watch replay closes the snapshot gap and isolates subscribers", async () => {
  const client = createDirectGroveClient(new ScriptedAgentSession(models));
  const initial = await client.sync();
  await client.execute({
    type: "session.select-model",
    commandId: "command-between-bootstrap-and-watch",
    sessionId: initial.session.id,
    model: models[1]!.ref,
  });

  const first = client.watch({ after: initial.cursor })[Symbol.asyncIterator]();
  const second = client.watch({ after: initial.cursor })[Symbol.asyncIterator]();
  const firstUpdate = await first.next();
  const secondUpdate = await second.next();

  assert.equal(firstUpdate.value?.kind, "event");
  assert.equal(secondUpdate.value?.kind, "event");
  if (firstUpdate.value?.kind === "event")
    firstUpdate.value.event.session.model.modelId = "mutated";
  if (secondUpdate.value?.kind === "event") {
    assert.equal(secondUpdate.value.event.session.model.modelId, "claude-sonnet-4-6");
  }

  await first.return?.();
  await second.return?.();
  await client.close();
});

test("Agent capabilities are authoritative for command admission", async () => {
  const agent = new ScriptedAgentSession(models, false, { prompt: false });
  const client = createDirectGroveClient(agent);
  const { session } = await client.sync();

  const result = await client.execute({
    type: "session.prompt",
    commandId: "command-unsupported",
    sessionId: session.id,
    text: "Do not run",
  });

  assert.deepEqual(result, {
    ok: false,
    type: "session.prompt",
    commandId: "command-unsupported",
    error: { code: "unsupported", message: "The Agent does not support prompting" },
  });
  assert.equal(agent.runCount, 0);
  await client.close();
});

test("closing a client ends active update iterators", async () => {
  const agent = new ScriptedAgentSession(models);
  const client = createDirectGroveClient(agent);
  const initial = await client.sync();
  const iterator = client.watch({ after: initial.cursor })[Symbol.asyncIterator]();

  await client.execute({
    type: "session.select-model",
    commandId: "command-before-close",
    sessionId: initial.session.id,
    model: models[1]!.ref,
  });
  assert.equal((await iterator.next()).done, false);

  const pending = iterator.next();
  await client.close();
  assert.equal((await pending).done, true);
  assert.equal(agent.shutdownCount, 1);
});

class ScriptedAgentSession implements AgentSessionService {
  readonly capabilities: AgentSessionCapabilities;
  readonly #models: readonly ModelSummary[];
  #model: ModelSummary;
  #thinkingLevel: ThinkingLevel = "low";
  readonly #waitForAbort: boolean;
  #resolveAbort: (() => void) | undefined;
  runCount = 0;
  shutdownCount = 0;

  constructor(
    availableModels: readonly ModelSummary[],
    waitForAbort = false,
    capabilities: Partial<AgentSessionCapabilities> = {},
  ) {
    this.capabilities = {
      prompt: true,
      abort: true,
      selectModel: true,
      setThinkingLevel: true,
      ...capabilities,
    };
    this.#models = availableModels;
    this.#model = availableModels[0]!;
    this.#waitForAbort = waitForAbort;
  }

  get models(): Effect.Effect<readonly ModelSummary[]> {
    return Effect.succeed(this.#models);
  }

  get model(): Effect.Effect<ModelSummary> {
    return Effect.sync(() => this.#model);
  }

  get thinkingLevel(): Effect.Effect<ThinkingLevel> {
    return Effect.sync(() => this.#thinkingLevel);
  }

  run(sink: AgentSessionSink, _text: string): Effect.Effect<AgentRunResult> {
    const start = Effect.sync(() => {
      this.runCount += 1;
      sink.progress({ type: "message.text-delta", delta: "Hello from the scripted agent." });
    });
    const waitForAbort = this.#waitForAbort
      ? Effect.callback<void>((resume) => {
          this.#resolveAbort = () => resume(Effect.void);
        })
      : Effect.void;
    const outcome = this.#waitForAbort ? ("aborted" as const) : ("completed" as const);
    return start.pipe(
      Effect.andThen(waitForAbort),
      Effect.as({
        outcome,
        parts: [{ type: "text" as const, text: "Hello from the scripted agent." }],
      }),
    );
  }

  get abort(): Effect.Effect<boolean> {
    return Effect.sync(() => {
      if (!this.#resolveAbort) return false;
      this.#resolveAbort();
      this.#resolveAbort = undefined;
      return true;
    });
  }

  selectModel(ref: ModelSummary["ref"]): Effect.Effect<boolean> {
    return Effect.sync(() => {
      const selected = this.#models.find(
        (candidate) =>
          candidate.ref.providerId === ref.providerId && candidate.ref.modelId === ref.modelId,
      );
      if (!selected) return false;
      this.#model = selected;
      return true;
    });
  }

  setThinkingLevel(level: ThinkingLevel): Effect.Effect<boolean> {
    return Effect.sync(() => {
      if (!this.#model.thinkingLevels.includes(level)) return false;
      this.#thinkingLevel = level;
      return true;
    });
  }

  get shutdown(): Effect.Effect<void> {
    return Effect.sync(() => {
      this.shutdownCount += 1;
      this.#resolveAbort?.();
      this.#resolveAbort = undefined;
    });
  }
}

async function waitForIdle(client: ReturnType<typeof createDirectGroveClient>): Promise<void> {
  for (let attempt = 0; attempt < 20; attempt += 1) {
    if ((await client.sync()).session.phase.type === "idle") return;
    await new Promise<void>((resolve) => setImmediate(resolve));
  }
  assert.fail("Session did not settle");
}

function model(providerId: string, modelId: string, name: string): ModelSummary {
  return {
    ref: { agentId: "pi", providerId, modelId },
    name,
    contextWindow: 200_000,
    input: ["text"],
    thinkingLevels: ["off", "low", "medium", "high"],
  };
}
