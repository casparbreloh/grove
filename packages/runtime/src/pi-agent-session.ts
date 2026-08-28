import {
  type AgentSession,
  type AgentSessionEvent,
  type AgentSessionRuntime,
  createAgentSessionFromServices,
  createAgentSessionRuntime,
  createAgentSessionServices,
  type CreateAgentSessionRuntimeFactory,
  getAgentDir,
  type ModelRuntime,
  SessionManager,
  type ToolDefinition,
} from "@earendil-works/pi-coding-agent";
import {
  getSupportedThinkingLevels,
  type Api,
  type AssistantMessage,
  type Model,
  type ToolResultMessage,
} from "@earendil-works/pi-ai";
import { grovePiExtension } from "@grove/pi-extension";

import {
  type AgentRunResult,
  type AgentSessionService,
  type AgentSessionSink,
} from "./agent-session.ts";
import type { JsonValue, MessagePart, ModelRef, ModelSummary, ThinkingLevel } from "./types.ts";

export interface PiAgentSessionOptions {
  cwd?: string;
  agentDir?: string;
  model?: { providerId: string; modelId: string };
  systemPrompt?: string;
  thinkingLevel?: ThinkingLevel;
  tools?: string[];
  excludeTools?: string[];
  noTools?: "all" | "builtin";
  customTools?: ToolDefinition[];
}

export async function createPiAgentSession(
  options: PiAgentSessionOptions = {},
): Promise<AgentSessionService> {
  const cwd = options.cwd ?? process.cwd();
  const agentDir = options.agentDir ?? getAgentDir();
  const createRuntime: CreateAgentSessionRuntimeFactory = async ({
    cwd: sessionCwd,
    sessionManager,
    sessionStartEvent,
  }) => {
    const services = await createAgentSessionServices({
      cwd: sessionCwd,
      agentDir,
      resourceLoaderOptions: {
        extensionFactories: [grovePiExtension],
        ...(options.systemPrompt ? { systemPrompt: options.systemPrompt } : {}),
      },
    });
    const model = options.model
      ? services.modelRuntime.getModel(options.model.providerId, options.model.modelId)
      : undefined;
    if (options.model && !model) {
      throw new Error(
        `Pi model is unavailable: ${options.model.providerId}/${options.model.modelId}`,
      );
    }
    const result = await createAgentSessionFromServices({
      services,
      sessionManager,
      sessionStartEvent,
      ...(model ? { model } : {}),
      ...(options.thinkingLevel ? { thinkingLevel: options.thinkingLevel } : {}),
      ...(options.tools ? { tools: options.tools } : {}),
      ...(options.excludeTools ? { excludeTools: options.excludeTools } : {}),
      ...(options.noTools ? { noTools: options.noTools } : {}),
      ...(options.customTools ? { customTools: options.customTools } : {}),
    });
    return { ...result, services, diagnostics: services.diagnostics };
  };
  const runtime = await createAgentSessionRuntime(createRuntime, {
    cwd,
    agentDir,
    sessionManager: SessionManager.inMemory(cwd),
  });

  if (!runtime.session.model) {
    await runtime.dispose();
    throw new Error("Pi has no models available");
  }
  try {
    await runtime.session.bindExtensions({});
  } catch (error) {
    await runtime.dispose();
    throw error;
  }

  return new PiAgentSession(runtime);
}

class PiAgentSession implements AgentSessionService {
  readonly capabilities = {
    prompt: true,
    abort: true,
    selectModel: true,
    setThinkingLevel: true,
  };
  readonly #runtime: AgentSessionRuntime;

  constructor(runtime: AgentSessionRuntime) {
    this.#runtime = runtime;
  }

  get models(): readonly ModelSummary[] {
    return uniqueModels([this.#requireModel(), ...this.#models.getAvailableSnapshot()]).map(
      toModelSummary,
    );
  }

  get model(): ModelSummary {
    return toModelSummary(this.#requireModel());
  }

  get thinkingLevel(): ThinkingLevel {
    return this.#session.thinkingLevel;
  }

  async run(sink: AgentSessionSink, text: string): Promise<AgentRunResult> {
    const startIndex = this.#session.messages.length;
    const unsubscribe = this.#session.subscribe((event) => publishProgress(sink, event));
    try {
      await this.#session.prompt(text);
      return toRunResult(this.#session.messages.slice(startIndex));
    } finally {
      unsubscribe();
    }
  }

  async abort(): Promise<boolean> {
    if (!this.#session.isStreaming) return false;
    await this.#session.abort();
    return true;
  }

  async selectModel(ref: ModelRef): Promise<boolean> {
    if (ref.agentId !== "pi") return false;
    const model = this.#models.getModel(ref.providerId, ref.modelId);
    if (!model) return false;
    try {
      await this.#session.setModel(model);
      return true;
    } catch {
      return false;
    }
  }

  async setThinkingLevel(level: ThinkingLevel): Promise<boolean> {
    if (!this.#session.getAvailableThinkingLevels().includes(level)) return false;
    this.#session.setThinkingLevel(level);
    return true;
  }

  async shutdown(): Promise<void> {
    if (this.#session.isStreaming) await this.#session.abort();
    await this.#runtime.dispose();
  }

  get #models(): ModelRuntime {
    return this.#runtime.services.modelRuntime;
  }

  get #session(): AgentSession {
    return this.#runtime.session;
  }

  #requireModel(): Model<Api> {
    const model = this.#session.model;
    if (!model) throw new Error("Pi session has no model");
    return model;
  }
}

function publishProgress(sink: AgentSessionSink, event: AgentSessionEvent): void {
  if (event.type === "message_update") {
    const update = event.assistantMessageEvent;
    if (update.type === "text_delta") {
      sink.progress({ type: "message.text-delta", delta: update.delta });
    }
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
  const parts = toTurnParts(messages);
  if (!message) {
    return {
      outcome: "failed",
      parts,
      error: {
        code: "missing-response",
        message: "Pi completed without an assistant response",
      },
    };
  }
  if (message.stopReason === "error") {
    return {
      outcome: "failed",
      parts,
      error: {
        code: "provider-error",
        message: message.errorMessage ?? "Model request failed",
      },
    };
  }
  if (message.stopReason === "aborted") {
    return { outcome: "aborted", parts };
  }
  return { outcome: "completed", parts };
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

function isToolResultMessage(message: unknown): message is ToolResultMessage {
  return (
    typeof message === "object" &&
    message !== null &&
    "role" in message &&
    message.role === "toolResult"
  );
}

function findLastAssistantMessage(messages: readonly unknown[]): AssistantMessage | undefined {
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    const message = messages[index];
    if (isAssistantMessage(message)) return message;
  }
  return undefined;
}

function toTurnParts(messages: readonly unknown[]): MessagePart[] {
  return messages.flatMap((message) => {
    if (isAssistantMessage(message)) return toMessageParts(message);
    if (isToolResultMessage(message)) {
      return [
        {
          type: "tool-result" as const,
          callId: message.toolCallId,
          name: message.toolName,
          output: message.content
            .map((part) => (part.type === "text" ? part.text : "[image]"))
            .join("\n"),
          isError: message.isError,
        },
      ];
    }
    return [];
  });
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
