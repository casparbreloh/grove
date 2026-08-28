import { Agent } from "@earendil-works/pi-agent-core";
import {
  clampThinkingLevel,
  getSupportedThinkingLevels,
  type Api,
  type AssistantMessage,
  type Model,
} from "@earendil-works/pi-ai";
import { ModelRuntime } from "@earendil-works/pi-coding-agent";

import type { AgentRunResult, AgentSessionDriver, AgentSessionSink } from "./agent-session.ts";
import type { JsonValue, MessagePart, ModelRef, ModelSummary, ThinkingLevel } from "./types.ts";

export interface PiAgentSessionOptions {
  model?: { providerId: string; modelId: string };
  systemPrompt?: string;
  thinkingLevel?: ThinkingLevel;
}

export async function createPiAgentSession(
  options: PiAgentSessionOptions = {},
): Promise<AgentSessionDriver> {
  const models = await ModelRuntime.create();
  const preferred = options.model
    ? models.getModel(options.model.providerId, options.model.modelId)
    : models.getModel("openai-codex", "gpt-5.6-sol");
  const model = preferred ?? models.getAvailableSnapshot()[0] ?? models.getModels()[0];

  if (!model) throw new Error("Pi has no models available");

  return new PiAgentSession(models, model, options);
}

class PiAgentSession implements AgentSessionDriver {
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

  get models(): readonly ModelSummary[] {
    return uniqueModels([this.#agent.state.model, ...this.#models.getAvailableSnapshot()]).map(
      toModelSummary,
    );
  }

  get model(): ModelSummary {
    return toModelSummary(this.#agent.state.model);
  }

  get thinkingLevel(): ThinkingLevel {
    return this.#agent.state.thinkingLevel;
  }

  async run(sink: AgentSessionSink, text: string): Promise<AgentRunResult> {
    const unsubscribe = this.#agent.subscribe((event) => {
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
    });

    try {
      await this.#agent.prompt(text);
      const message = findLastAssistantMessage(this.#agent.state.messages);
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
    } finally {
      unsubscribe();
    }
  }

  abort(): boolean {
    if (!this.#agent.state.isStreaming) return false;
    this.#agent.abort();
    return true;
  }

  selectModel(ref: ModelRef): boolean {
    if (ref.agentId !== "pi") return false;
    const model = this.#models.getModel(ref.providerId, ref.modelId);
    if (!model) return false;
    this.#agent.state.model = model;
    this.#agent.state.thinkingLevel = clampThinkingLevel(model, this.#agent.state.thinkingLevel);
    return true;
  }

  setThinkingLevel(level: ThinkingLevel): boolean {
    if (!getSupportedThinkingLevels(this.#agent.state.model).includes(level)) return false;
    this.#agent.state.thinkingLevel = level;
    return true;
  }
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
