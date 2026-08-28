import { Agent, type AgentEvent } from "@earendil-works/pi-agent-core";
import {
  clampThinkingLevel,
  getSupportedThinkingLevels,
  type Api,
  type AssistantMessage,
  type Model,
} from "@earendil-works/pi-ai";
import { ModelRuntime } from "@earendil-works/pi-coding-agent";
import { Cause, Effect } from "effect";

import {
  type AgentRunResult,
  type AgentSessionService,
  type AgentSessionSink,
} from "./agent-session.ts";
import type { JsonValue, MessagePart, ModelRef, ModelSummary, ThinkingLevel } from "./types.ts";

export interface PiAgentSessionOptions {
  model?: { providerId: string; modelId: string };
  systemPrompt?: string;
  thinkingLevel?: ThinkingLevel;
}

export const createPiAgentSession: (
  options?: PiAgentSessionOptions,
) => Effect.Effect<AgentSessionService, Cause.UnknownError> = Effect.fn("PiAgentSession.create")(
  function* (options: PiAgentSessionOptions = {}) {
    const models = yield* Effect.tryPromise(() => ModelRuntime.create());
    const preferred = options.model
      ? models.getModel(options.model.providerId, options.model.modelId)
      : models.getModel("openai-codex", "gpt-5.6-sol");
    const model = preferred ?? models.getAvailableSnapshot()[0] ?? models.getModels()[0];

    if (!model) {
      return yield* Effect.fail(new Cause.UnknownError(undefined, "Pi has no models available"));
    }

    return new PiAgentSession(models, model, options);
  },
);

class PiAgentSession implements AgentSessionService {
  readonly capabilities = {
    prompt: true,
    abort: true,
    selectModel: true,
    setThinkingLevel: true,
  };
  readonly #models: ModelRuntime;
  readonly #agent: Agent;

  constructor(models: ModelRuntime, model: Model<Api>, options: PiAgentSessionOptions) {
    this.#models = models;
    const requestedThinking = options.thinkingLevel ?? "low";
    this.#agent = new Agent({
      initialState: {
        model,
        systemPrompt: options.systemPrompt ?? "You are a concise software development agent.",
        thinkingLevel: clampThinkingLevel(model, requestedThinking),
        tools: [],
      },
      streamFn: models.streamSimple.bind(models),
    });
  }

  get models(): Effect.Effect<readonly ModelSummary[]> {
    return Effect.sync(() =>
      uniqueModels([this.#agent.state.model, ...this.#models.getAvailableSnapshot()]).map(
        toModelSummary,
      ),
    );
  }

  get model(): Effect.Effect<ModelSummary> {
    return Effect.sync(() => toModelSummary(this.#agent.state.model));
  }

  get thinkingLevel(): Effect.Effect<ThinkingLevel> {
    return Effect.sync(() => this.#agent.state.thinkingLevel);
  }

  run(sink: AgentSessionSink, text: string): Effect.Effect<AgentRunResult, Cause.UnknownError> {
    return Effect.scoped(runPiAgent(this.#agent, sink, text));
  }

  get abort(): Effect.Effect<boolean> {
    return Effect.sync(() => {
      if (!this.#agent.state.isStreaming) return false;
      this.#agent.abort();
      return true;
    });
  }

  selectModel(ref: ModelRef): Effect.Effect<boolean> {
    return Effect.sync(() => {
      if (ref.agentId !== "pi") return false;
      const model = this.#models.getModel(ref.providerId, ref.modelId);
      if (!model) return false;
      this.#agent.state.model = model;
      this.#agent.state.thinkingLevel = clampThinkingLevel(model, this.#agent.state.thinkingLevel);
      return true;
    });
  }

  setThinkingLevel(level: ThinkingLevel): Effect.Effect<boolean> {
    return Effect.sync(() => {
      if (!getSupportedThinkingLevels(this.#agent.state.model).includes(level)) return false;
      this.#agent.state.thinkingLevel = level;
      return true;
    });
  }

  get shutdown(): Effect.Effect<void> {
    return Effect.sync(() => this.#agent.abort());
  }
}

const runPiAgent = Effect.fn("PiAgentSession.run")(function* (
  agent: Agent,
  sink: AgentSessionSink,
  text: string,
) {
  yield* Effect.acquireRelease(
    Effect.sync(() => agent.subscribe((event) => publishProgress(sink, event))),
    (unsubscribe) => Effect.sync(unsubscribe),
  );
  yield* Effect.callback<void, Cause.UnknownError>((resume) => {
    void agent.prompt(text).then(
      () => resume(Effect.void),
      (cause) => resume(Effect.fail(new Cause.UnknownError(cause, "Pi Agent request failed"))),
    );
    return Effect.sync(() => agent.abort());
  });
  return toRunResult(agent.state.messages);
});

function publishProgress(sink: AgentSessionSink, event: AgentEvent): void {
  if (event.type === "message_update") {
    const update = event.assistantMessageEvent;
    if (update.type === "text_delta")
      sink.progress({ type: "message.text-delta", delta: update.delta });
    if (update.type === "thinking_delta") {
      sink.progress({ type: "message.reasoning-delta", delta: update.delta });
    }
  }
  if (event.type === "tool_execution_start") {
    sink.progress({ type: "tool.started", callId: event.toolCallId, name: event.toolName });
  }
  if (event.type === "tool_execution_end") {
    sink.progress({
      type: "tool.settled",
      callId: event.toolCallId,
      name: event.toolName,
      isError: event.isError,
    });
  }
}

function toRunResult(messages: readonly unknown[]): AgentRunResult {
  const message = findLastAssistantMessage(messages);
  if (!message) {
    return {
      outcome: "failed",
      parts: [],
      error: {
        code: "missing-response",
        message: "Pi completed without an assistant response",
      },
    };
  }
  if (message.stopReason === "error") {
    return {
      outcome: "failed",
      parts: toMessageParts(message),
      error: {
        code: "provider-error",
        message: message.errorMessage ?? "Model request failed",
      },
    };
  }
  if (message.stopReason === "aborted") {
    return { outcome: "aborted", parts: toMessageParts(message) };
  }
  return { outcome: "completed", parts: toMessageParts(message) };
}

function toModelSummary(model: Model<Api>): ModelSummary {
  return {
    ref: { agentId: "pi", providerId: model.provider, modelId: model.id },
    name: model.name,
    contextWindow: model.contextWindow,
    input: model.input,
    thinkingLevels: getSupportedThinkingLevels(model),
  };
}

function uniqueModels(models: readonly Model<Api>[]): Model<Api>[] {
  return models.filter(
    (model, index) =>
      models.findIndex((candidate) => modelKey(candidate) === modelKey(model)) === index,
  );
}

function modelKey(model: Model<Api>): string {
  return `${model.provider}:${model.id}`;
}

function isAssistantMessage(message: unknown): message is AssistantMessage {
  return (
    typeof message === "object" &&
    message !== null &&
    "role" in message &&
    message.role === "assistant"
  );
}

function findLastAssistantMessage(messages: readonly unknown[]): AssistantMessage | undefined {
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    const message = messages[index];
    if (isAssistantMessage(message)) return message;
  }
  return undefined;
}

function toMessageParts(message: AssistantMessage): MessagePart[] {
  return message.content.map((part) => {
    switch (part.type) {
      case "text":
        return { type: "text", text: part.text };
      case "thinking":
        return { type: "reasoning", text: part.thinking };
      case "toolCall":
        return {
          type: "tool-call",
          callId: part.id,
          name: part.name,
          input: toJsonValue(part.arguments),
        };
    }
  });
}

function toJsonValue(value: unknown): JsonValue {
  if (value === null || typeof value === "string" || typeof value === "boolean") return value;
  if (typeof value === "number") return Number.isFinite(value) ? value : String(value);
  if (Array.isArray(value)) return value.map(toJsonValue);
  if (typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value).map(([key, entry]) => [key, toJsonValue(entry)]),
    );
  }
  return String(value);
}
