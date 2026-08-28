import assert from "node:assert/strict";
import test from "node:test";

import type {
  AgentRunResult,
  AgentSessionCapabilities,
  AgentSessionDriver,
  AgentSessionSink,
} from "./agent-session.ts";
import { createDirectGroveClient } from "./runtime.ts";
import type { ModelSummary, ThinkingLevel } from "./types.ts";

const models = [
  model("openai-codex", "gpt-5.6-sol", "GPT-5.6 Sol"),
  model("anthropic", "claude-sonnet-4-6", "Claude Sonnet 4.6"),
];

test("bootstrap exposes authoritative session state and model commands update it", async () => {
  const client = createDirectGroveClient(new ScriptedAgentSession(models));
  const initial = await client.bootstrap();

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
  assert.deepEqual((await client.bootstrap()).session.model, models[1]?.ref);
});

test("prompt publishes transient progress and settles an authoritative transcript", async () => {
  const client = createDirectGroveClient(new ScriptedAgentSession(models));
  const initial = await client.bootstrap();
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

  const settled = await client.bootstrap();
  assert.equal(settled.session.phase.type, "idle");
  assert.deepEqual(
    settled.session.messages.map((message) => message.role),
    ["user", "assistant"],
  );
  assert.equal(settled.session.messages[1]?.parts[0]?.type, "text");
  assert.equal(settled.session.messages[1]?.parts[0]?.text, "Hello from the scripted agent.");

  await iterator.return?.();
});

test("a second prompt is rejected while the session is running and abort settles the first", async () => {
  const agent = new ScriptedAgentSession(models, true);
  const client = createDirectGroveClient(agent);
  const { session } = await client.bootstrap();

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
  assert.equal((await client.bootstrap()).session.lastTurn?.outcome, "aborted");
});

test("redelivering a command ID returns its receipt without running the command twice", async () => {
  const agent = new ScriptedAgentSession(models);
  const client = createDirectGroveClient(agent);
  const { session } = await client.bootstrap();
  const command = {
    type: "session.prompt" as const,
    commandId: "command-retried",
    sessionId: session.id,
    text: "Only once",
  };

  const first = await client.execute(command);
  const retried = await client.execute(command);

  assert.deepEqual(retried, first);
  assert.equal(agent.runCount, 1);
  assert.equal((await client.bootstrap()).session.messages.length, 2);
});

test("watch replay closes the snapshot gap and isolates subscribers", async () => {
  const client = createDirectGroveClient(new ScriptedAgentSession(models));
  const initial = await client.bootstrap();
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
});

test("Agent capabilities are authoritative for command admission", async () => {
  const agent = new ScriptedAgentSession(models, false, { prompt: false });
  const client = createDirectGroveClient(agent);
  const { session } = await client.bootstrap();

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
});

class ScriptedAgentSession implements AgentSessionDriver {
  readonly capabilities: AgentSessionCapabilities;
  readonly models: readonly ModelSummary[];
  model: ModelSummary;
  thinkingLevel: ThinkingLevel = "low";
  readonly #waitForAbort: boolean;
  #resolveAbort: (() => void) | undefined;
  runCount = 0;

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
    this.models = availableModels;
    this.model = availableModels[0]!;
    this.#waitForAbort = waitForAbort;
  }

  async run(sink: AgentSessionSink, _text: string): Promise<AgentRunResult> {
    this.runCount += 1;
    sink.progress({ type: "message.text-delta", delta: "Hello from the scripted agent." });
    if (this.#waitForAbort) await new Promise<void>((resolve) => (this.#resolveAbort = resolve));
    return {
      outcome: this.#waitForAbort ? "aborted" : "completed",
      parts: [{ type: "text", text: "Hello from the scripted agent." }],
    };
  }

  abort(): boolean {
    if (!this.#resolveAbort) return false;
    this.#resolveAbort();
    this.#resolveAbort = undefined;
    return true;
  }

  selectModel(ref: ModelSummary["ref"]): boolean {
    const selected = this.models.find(
      (candidate) =>
        candidate.ref.providerId === ref.providerId && candidate.ref.modelId === ref.modelId,
    );
    if (!selected) return false;
    this.model = selected;
    return true;
  }

  setThinkingLevel(level: ThinkingLevel): boolean {
    if (!this.model.thinkingLevels.includes(level)) return false;
    this.thinkingLevel = level;
    return true;
  }
}

async function waitForIdle(client: ReturnType<typeof createDirectGroveClient>): Promise<void> {
  for (let attempt = 0; attempt < 10; attempt += 1) {
    if ((await client.bootstrap()).session.phase.type === "idle") return;
    await Promise.resolve();
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
