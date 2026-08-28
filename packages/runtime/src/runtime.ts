import { randomUUID } from "node:crypto";

import type { AgentSessionDriver, AgentSessionProgress } from "./agent-session.ts";
import type {
  CommandRejected,
  GroveBootstrap,
  GroveClient,
  GroveCommand,
  GroveCommandResult,
  GroveMessage,
  GroveProgress,
  GroveUpdate,
  GroveWatchOptions,
  PromptCommandResult,
  ResultFor,
  SelectModelCommandResult,
  SessionSnapshot,
  SetThinkingLevelCommandResult,
  TaskSummary,
  TurnSummary,
} from "./types.ts";

export interface DirectGroveClientOptions {
  environmentId?: string;
  projectId?: string;
  taskId?: string;
  sessionId?: string;
}

export function createDirectGroveClient(
  agent: AgentSessionDriver,
  options: DirectGroveClientOptions = {},
): GroveClient {
  return new DirectGroveClient(agent, options);
}

class DirectGroveClient implements GroveClient {
  readonly #agent: AgentSessionDriver;
  readonly #streamId: string;
  readonly #task: TaskSummary;
  readonly #watchers = new Set<UpdateQueue>();
  readonly #history: Extract<GroveUpdate, { kind: "event" }>[] = [];
  readonly #receipts = new Map<
    string,
    { fingerprint: string; result: Promise<GroveCommandResult> }
  >();
  #sequence = 0;
  #session: SessionSnapshot;

  constructor(agent: AgentSessionDriver, options: DirectGroveClientOptions) {
    this.#agent = agent;
    const taskId = options.taskId ?? randomUUID();
    const sessionId = options.sessionId ?? randomUUID();
    this.#streamId = `session:${sessionId}`;
    this.#task = {
      id: taskId,
      projectId: options.projectId ?? "grove-harness",
      environment: { id: options.environmentId ?? "local", kind: "local" },
    };
    this.#session = {
      id: sessionId,
      taskId,
      phase: { type: "idle" },
      model: agent.model.ref,
      thinkingLevel: agent.thinkingLevel,
      messages: [],
      capabilities: clone(agent.capabilities),
    };
  }

  async bootstrap(): Promise<GroveBootstrap> {
    return clone({
      task: this.#task,
      session: this.#session,
      models: this.#agent.models,
      cursor: { [this.#streamId]: this.#sequence },
    });
  }

  async execute<TCommand extends GroveCommand>(command: TCommand): Promise<ResultFor<TCommand>> {
    const fingerprint = commandFingerprint(command);
    const receipt = this.#receipts.get(command.commandId);
    if (receipt) {
      if (receipt.fingerprint !== fingerprint) {
        return this.#reject(
          command,
          "invalid-input",
          `Command ID was already used with different input: ${command.commandId}`,
        ) as ResultFor<TCommand>;
      }
      return clone(await receipt.result) as ResultFor<TCommand>;
    }

    const result = this.#execute(command);
    this.#receipts.set(command.commandId, { fingerprint, result });
    return clone(await result) as ResultFor<TCommand>;
  }

  watch(options: GroveWatchOptions = {}): AsyncIterable<GroveUpdate> {
    return {
      [Symbol.asyncIterator]: () => this.#createWatcher(options),
    };
  }

  async #execute(command: GroveCommand): Promise<GroveCommandResult> {
    if (command.sessionId !== this.#session.id) {
      return this.#reject(command, "not-found", `Session not found: ${command.sessionId}`);
    }

    switch (command.type) {
      case "session.prompt":
        return this.#prompt(command);
      case "session.abort":
        return this.#abort(command);
      case "session.select-model":
        return this.#selectModel(command);
      case "session.set-thinking-level":
        return this.#setThinkingLevel(command);
    }
  }

  #prompt(command: Extract<GroveCommand, { type: "session.prompt" }>): GroveCommandResult {
    if (!this.#session.capabilities.prompt) {
      return this.#reject(command, "unsupported", "The Agent does not support prompting");
    }
    if (this.#session.phase.type === "running") {
      return this.#reject(command, "busy", "Session already has an active Turn");
    }
    const text = command.text.trim();
    if (!text) return this.#reject(command, "invalid-input", "Prompt text is empty");

    const turnId = randomUUID();
    const assistantMessageId = randomUUID();
    const userMessage: GroveMessage = {
      id: randomUUID(),
      role: "user",
      createdAt: Date.now(),
      parts: [{ type: "text", text }],
    };
    this.#session = {
      ...this.#session,
      phase: { type: "running", turnId },
      messages: [...this.#session.messages, userMessage],
      lastTurn: undefined,
    };
    this.#publishSession();

    void this.#runTurn(text, turnId, assistantMessageId);

    return {
      ok: true,
      type: "session.prompt",
      commandId: command.commandId,
      turnId,
      status: "accepted",
    } satisfies PromptCommandResult;
  }

  async #runTurn(text: string, turnId: string, assistantMessageId: string): Promise<void> {
    let result: Awaited<ReturnType<AgentSessionDriver["run"]>>;
    try {
      result = await this.#agent.run(
        {
          progress: (progress) => this.#publishProgress(turnId, assistantMessageId, progress),
        },
        text,
      );
    } catch (error) {
      result = {
        outcome: "failed",
        parts: [],
        error: {
          code: "agent-failure",
          message: error instanceof Error ? error.message : "Agent failed",
        },
      };
    }

    const assistantMessage: GroveMessage = {
      id: assistantMessageId,
      role: "assistant",
      createdAt: Date.now(),
      model: this.#session.model,
      parts: result.parts,
    };
    const lastTurn: TurnSummary = {
      id: turnId,
      outcome: result.outcome,
      ...(result.error ? { error: result.error } : {}),
    };
    this.#session = {
      ...this.#session,
      phase: { type: "idle" },
      messages:
        result.parts.length > 0
          ? [...this.#session.messages, assistantMessage]
          : this.#session.messages,
      lastTurn,
    };
    this.#publishSession();
  }

  #abort(command: Extract<GroveCommand, { type: "session.abort" }>): GroveCommandResult {
    if (!this.#session.capabilities.abort) {
      return this.#reject(command, "unsupported", "The Agent does not support aborting");
    }
    const requested = this.#session.phase.type === "running" && this.#agent.abort();
    return {
      ok: true,
      type: "session.abort",
      commandId: command.commandId,
      outcome: requested ? "requested" : "idle",
    };
  }

  #selectModel(
    command: Extract<GroveCommand, { type: "session.select-model" }>,
  ): GroveCommandResult {
    if (!this.#session.capabilities.selectModel) {
      return this.#reject(command, "unsupported", "The Agent does not support model selection");
    }
    if (this.#session.phase.type === "running") {
      return this.#reject(command, "busy", "Cannot change model during an active Turn");
    }
    if (!this.#agent.selectModel(command.model)) {
      return this.#reject(
        command,
        "unsupported",
        `Model is unavailable: ${command.model.providerId}/${command.model.modelId}`,
      );
    }
    this.#session = {
      ...this.#session,
      model: this.#agent.model.ref,
      thinkingLevel: this.#agent.thinkingLevel,
    };
    this.#publishSession();
    return {
      ok: true,
      type: "session.select-model",
      commandId: command.commandId,
      model: this.#session.model,
    } satisfies SelectModelCommandResult;
  }

  #setThinkingLevel(
    command: Extract<GroveCommand, { type: "session.set-thinking-level" }>,
  ): GroveCommandResult {
    if (!this.#session.capabilities.setThinkingLevel) {
      return this.#reject(
        command,
        "unsupported",
        "The Agent does not support changing its thinking level",
      );
    }
    if (this.#session.phase.type === "running") {
      return this.#reject(command, "busy", "Cannot change thinking level during an active Turn");
    }
    if (!this.#agent.setThinkingLevel(command.thinkingLevel)) {
      return this.#reject(
        command,
        "unsupported",
        `Thinking level is unavailable: ${command.thinkingLevel}`,
      );
    }
    this.#session = { ...this.#session, thinkingLevel: this.#agent.thinkingLevel };
    this.#publishSession();
    return {
      ok: true,
      type: "session.set-thinking-level",
      commandId: command.commandId,
      thinkingLevel: this.#session.thinkingLevel,
    } satisfies SetThinkingLevelCommandResult;
  }

  #publishSession(): void {
    const update: Extract<GroveUpdate, { kind: "event" }> = {
      kind: "event",
      streamId: this.#streamId,
      sequence: ++this.#sequence,
      occurredAt: Date.now(),
      event: { type: "session.updated", session: clone(this.#session) },
    };
    this.#history.push(update);
    for (const watcher of this.#watchers) watcher.push(clone(update));
  }

  #publishProgress(turnId: string, messageId: string, progress: AgentSessionProgress): void {
    const common = { sessionId: this.#session.id, turnId, messageId };
    let projected: GroveProgress;
    switch (progress.type) {
      case "message.text-delta":
      case "message.reasoning-delta":
        projected = { ...common, ...progress };
        break;
      case "tool.started":
      case "tool.settled":
        projected = { ...common, ...progress };
        break;
    }
    const update: GroveUpdate = { kind: "progress", progress: projected };
    for (const watcher of this.#watchers) watcher.push(clone(update));
  }

  #createWatcher(options: GroveWatchOptions): AsyncIterator<GroveUpdate> {
    let queue: UpdateQueue;
    queue = new UpdateQueue(() => this.#watchers.delete(queue));
    const after = options.after?.[this.#streamId] ?? 0;
    for (const update of this.#history) {
      if (update.sequence > after) queue.push(clone(update));
    }
    this.#watchers.add(queue);
    return queue;
  }

  #reject(
    command: GroveCommand,
    code: CommandRejected["error"]["code"],
    message: string,
  ): CommandRejected {
    return {
      ok: false,
      type: command.type,
      commandId: command.commandId,
      error: { code, message },
    };
  }
}

class UpdateQueue implements AsyncIterator<GroveUpdate> {
  readonly #onClose: () => void;
  readonly #updates: GroveUpdate[] = [];
  readonly #waiters: ((result: IteratorResult<GroveUpdate>) => void)[] = [];
  #closed = false;

  constructor(onClose: () => void) {
    this.#onClose = onClose;
  }

  push(update: GroveUpdate): void {
    if (this.#closed) return;
    const waiter = this.#waiters.shift();
    if (waiter) waiter({ done: false, value: update });
    else this.#updates.push(update);
  }

  next(): Promise<IteratorResult<GroveUpdate>> {
    const update = this.#updates.shift();
    if (update) return Promise.resolve({ done: false, value: update });
    if (this.#closed) return Promise.resolve({ done: true, value: undefined });
    return new Promise((resolve) => this.#waiters.push(resolve));
  }

  return(): Promise<IteratorResult<GroveUpdate>> {
    this.close();
    return Promise.resolve({ done: true, value: undefined });
  }

  close(): void {
    if (this.#closed) return;
    this.#closed = true;
    this.#onClose();
    for (const waiter of this.#waiters.splice(0)) waiter({ done: true, value: undefined });
    this.#updates.length = 0;
  }
}

function clone<T>(value: T): T {
  return structuredClone(value);
}

function commandFingerprint(command: GroveCommand): string {
  switch (command.type) {
    case "session.prompt":
      return `${command.type}\u0000${command.sessionId}\u0000${command.text}`;
    case "session.abort":
      return `${command.type}\u0000${command.sessionId}`;
    case "session.select-model":
      return `${command.type}\u0000${command.sessionId}\u0000${command.model.agentId}\u0000${command.model.providerId}\u0000${command.model.modelId}`;
    case "session.set-thinking-level":
      return `${command.type}\u0000${command.sessionId}\u0000${command.thinkingLevel}`;
  }
}
