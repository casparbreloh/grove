import { randomUUID } from "node:crypto";

import type { AgentRunResult, AgentSessionProgress, AgentSessionService } from "./agent-session.ts";
import type {
  AbortCommandResult,
  CommandRejected,
  GroveClient,
  GroveCommand,
  GroveCommandResult,
  GroveMessage,
  GroveProgress,
  GroveSync,
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

interface RuntimeState {
  readonly task: TaskSummary;
  readonly session: SessionSnapshot;
  readonly sequence: number;
  readonly history: readonly Extract<GroveUpdate, { kind: "event" }>[];
  readonly receipts: ReadonlyMap<string, CommandReceipt>;
}

interface CommandReceipt {
  readonly fingerprint: string;
  readonly result: GroveCommandResult;
}

interface Subscriber {
  readonly queue: AsyncQueue<GroveUpdate>;
  readonly liveAfter: number;
}

export function createDirectGroveClient(
  agent: AgentSessionService,
  options: DirectGroveClientOptions = {},
): GroveClient {
  return new DirectGroveClient(agent, options);
}

class DirectGroveClient implements GroveClient {
  readonly #agent: AgentSessionService;
  readonly #gate = new Mutex();
  readonly #subscribers = new Set<Subscriber>();
  readonly #activeRuns = new Set<Promise<void>>();
  #closePromise: Promise<void> | undefined;
  #closed = false;
  #state: RuntimeState;

  constructor(agent: AgentSessionService, options: DirectGroveClientOptions) {
    this.#agent = agent;
    const taskId = options.taskId ?? randomUUID();
    const sessionId = options.sessionId ?? randomUUID();
    this.#state = {
      task: {
        id: taskId,
        projectId: options.projectId ?? "grove-harness",
        environment: { id: options.environmentId ?? "local", kind: "local" },
      },
      session: {
        id: sessionId,
        taskId,
        phase: { type: "idle" },
        model: agent.model.ref,
        thinkingLevel: agent.thinkingLevel,
        messages: [],
        capabilities: clone(agent.capabilities),
      },
      sequence: 0,
      history: [],
      receipts: new Map(),
    };
  }

  async sync(): Promise<GroveSync> {
    return this.#gate.run(() => {
      this.#ensureOpen();
      return clone({
        task: this.#state.task,
        session: this.#state.session,
        models: this.#agent.models,
        cursor: { [streamId(this.#state.session.id)]: this.#state.sequence },
      });
    });
  }

  async execute<TCommand extends GroveCommand>(command: TCommand): Promise<ResultFor<TCommand>> {
    return this.#gate.run(async () => {
      this.#ensureOpen();
      const current = this.#state;
      if (command.sessionId !== current.session.id) {
        return reject(
          command,
          "not-found",
          `Session not found: ${command.sessionId}`,
        ) as ResultFor<TCommand>;
      }

      const fingerprint = commandFingerprint(command);
      const receipt = current.receipts.get(command.commandId);
      if (receipt) {
        return clone(
          receipt.fingerprint === fingerprint
            ? receipt.result
            : reject(
                command,
                "invalid-input",
                `Command ID was already used with different input: ${command.commandId}`,
              ),
        ) as ResultFor<TCommand>;
      }

      const result = await this.#executeNew(command);
      this.#state = {
        ...this.#state,
        receipts: new Map(this.#state.receipts).set(command.commandId, {
          fingerprint,
          result: clone(result),
        }),
      };
      return clone(result) as ResultFor<TCommand>;
    });
  }

  watch(options: GroveWatchOptions = {}): AsyncIterable<GroveUpdate> {
    const queue = new AsyncQueue<GroveUpdate>();
    const snapshot = this.#state;
    const after = options.after?.[streamId(snapshot.session.id)] ?? 0;
    const subscriber: Subscriber = { queue, liveAfter: snapshot.sequence };

    if (this.#closed) {
      queue.end();
    } else {
      for (const update of snapshot.history) {
        if (update.sequence > after) queue.push(clone(update));
      }
      this.#subscribers.add(subscriber);
    }

    return {
      [Symbol.asyncIterator]: () => ({
        next: () => queue.next(),
        return: async () => {
          this.#subscribers.delete(subscriber);
          queue.end();
          return { done: true, value: undefined };
        },
        throw: async (error?: unknown) => {
          this.#subscribers.delete(subscriber);
          queue.end();
          throw error;
        },
      }),
    };
  }

  close(): Promise<void> {
    this.#closePromise ??= this.#close();
    return this.#closePromise;
  }

  async #close(): Promise<void> {
    const activeRuns = await this.#gate.run(() => {
      this.#closed = true;
      for (const subscriber of this.#subscribers) subscriber.queue.end();
      this.#subscribers.clear();
      return [...this.#activeRuns];
    });
    try {
      await this.#agent.shutdown();
    } finally {
      await Promise.allSettled(activeRuns);
    }
  }

  async #executeNew(command: GroveCommand): Promise<GroveCommandResult> {
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

  async #prompt(
    command: Extract<GroveCommand, { type: "session.prompt" }>,
  ): Promise<PromptCommandResult | CommandRejected> {
    const current = this.#state;
    if (!current.session.capabilities.prompt) {
      return reject(command, "unsupported", "The Agent does not support prompting");
    }
    if (current.session.phase.type === "running") {
      return reject(command, "busy", "Session already has an active Turn");
    }
    const text = command.text.trim();
    if (!text) return reject(command, "invalid-input", "Prompt text is empty");

    const turnId = randomUUID();
    const assistantMessageId = randomUUID();
    const userMessage: GroveMessage = {
      id: randomUUID(),
      role: "user",
      createdAt: Date.now(),
      parts: [{ type: "text", text }],
    };
    this.#state = {
      ...current,
      session: {
        ...current.session,
        phase: { type: "running", turnId },
        messages: [...current.session.messages, userMessage],
        lastTurn: undefined,
      },
    };
    this.#publishSession();

    const activeRun = this.#runTurn(text, turnId, assistantMessageId);
    this.#activeRuns.add(activeRun);
    void activeRun.finally(() => this.#activeRuns.delete(activeRun));

    return {
      ok: true,
      type: "session.prompt",
      commandId: command.commandId,
      turnId,
      status: "accepted",
    };
  }

  async #abort(
    command: Extract<GroveCommand, { type: "session.abort" }>,
  ): Promise<AbortCommandResult | CommandRejected> {
    const current = this.#state;
    if (!current.session.capabilities.abort) {
      return reject(command, "unsupported", "The Agent does not support aborting");
    }
    const requested = current.session.phase.type === "running" && (await this.#agent.abort());
    return {
      ok: true,
      type: "session.abort",
      commandId: command.commandId,
      outcome: requested ? "requested" : "idle",
    };
  }

  async #selectModel(
    command: Extract<GroveCommand, { type: "session.select-model" }>,
  ): Promise<SelectModelCommandResult | CommandRejected> {
    const current = this.#state;
    if (!current.session.capabilities.selectModel) {
      return reject(command, "unsupported", "The Agent does not support model selection");
    }
    if (current.session.phase.type === "running") {
      return reject(command, "busy", "Cannot change model during an active Turn");
    }
    if (!(await this.#agent.selectModel(command.model))) {
      return reject(
        command,
        "unsupported",
        `Model is unavailable: ${command.model.providerId}/${command.model.modelId}`,
      );
    }
    const selected = this.#agent.model;
    this.#state = {
      ...this.#state,
      session: {
        ...this.#state.session,
        model: selected.ref,
        thinkingLevel: this.#agent.thinkingLevel,
      },
    };
    this.#publishSession();
    return {
      ok: true,
      type: "session.select-model",
      commandId: command.commandId,
      model: selected.ref,
    };
  }

  async #setThinkingLevel(
    command: Extract<GroveCommand, { type: "session.set-thinking-level" }>,
  ): Promise<SetThinkingLevelCommandResult | CommandRejected> {
    const current = this.#state;
    if (!current.session.capabilities.setThinkingLevel) {
      return reject(
        command,
        "unsupported",
        "The Agent does not support changing its thinking level",
      );
    }
    if (current.session.phase.type === "running") {
      return reject(command, "busy", "Cannot change thinking level during an active Turn");
    }
    if (!(await this.#agent.setThinkingLevel(command.thinkingLevel))) {
      return reject(
        command,
        "unsupported",
        `Thinking level is unavailable: ${command.thinkingLevel}`,
      );
    }
    const thinkingLevel = this.#agent.thinkingLevel;
    this.#state = {
      ...this.#state,
      session: { ...this.#state.session, thinkingLevel },
    };
    this.#publishSession();
    return {
      ok: true,
      type: "session.set-thinking-level",
      commandId: command.commandId,
      thinkingLevel,
    };
  }

  async #runTurn(text: string, turnId: string, assistantMessageId: string): Promise<void> {
    let result: AgentRunResult;
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
        error: { code: "agent-failure", message: errorMessage(error) },
      };
    }

    await this.#gate.run(() => this.#settleTurn(turnId, assistantMessageId, result));
  }

  #settleTurn(turnId: string, assistantMessageId: string, result: AgentRunResult): void {
    if (this.#closed) return;
    const current = this.#state;
    const assistantMessage: GroveMessage = {
      id: assistantMessageId,
      role: "assistant",
      createdAt: Date.now(),
      model: current.session.model,
      parts: result.parts,
    };
    const lastTurn: TurnSummary = {
      id: turnId,
      outcome: result.outcome,
      ...(result.error ? { error: result.error } : {}),
    };
    this.#state = {
      ...current,
      session: {
        ...current.session,
        phase: { type: "idle" },
        messages:
          result.parts.length > 0
            ? [...current.session.messages, assistantMessage]
            : current.session.messages,
        lastTurn,
      },
    };
    this.#publishSession();
  }

  #publishSession(): void {
    const update: Extract<GroveUpdate, { kind: "event" }> = {
      kind: "event",
      streamId: streamId(this.#state.session.id),
      sequence: this.#state.sequence + 1,
      occurredAt: Date.now(),
      event: { type: "session.updated", session: clone(this.#state.session) },
    };
    this.#state = {
      ...this.#state,
      sequence: update.sequence,
      history: [...this.#state.history, update],
    };
    this.#publish(update);
  }

  #publishProgress(turnId: string, messageId: string, progress: AgentSessionProgress): void {
    if (this.#closed) return;
    const common = { sessionId: this.#state.session.id, turnId, messageId };
    let projected: GroveProgress;
    switch (progress.type) {
      case "message.text-delta":
      case "message.reasoning-delta":
      case "tool.started":
      case "tool.settled":
        projected = { ...common, ...progress };
        break;
    }
    this.#publish({ kind: "progress", progress: projected });
  }

  #publish(update: GroveUpdate): void {
    for (const subscriber of this.#subscribers) {
      if (update.kind === "progress" || update.sequence > subscriber.liveAfter) {
        subscriber.queue.push(clone(update));
      }
    }
  }

  #ensureOpen(): void {
    if (this.#closed) throw new Error("Grove client is closed");
  }
}

class Mutex {
  #tail = Promise.resolve();

  async run<T>(operation: () => T | Promise<T>): Promise<T> {
    const previous = this.#tail;
    let release: () => void = () => undefined;
    const current = new Promise<void>((resolve) => {
      release = resolve;
    });
    this.#tail = previous.then(() => current);
    await previous;
    try {
      return await operation();
    } finally {
      release();
    }
  }
}

class AsyncQueue<T> {
  readonly #values: T[] = [];
  readonly #waiters: ((result: IteratorResult<T>) => void)[] = [];
  #ended = false;

  push(value: T): void {
    if (this.#ended) return;
    const waiter = this.#waiters.shift();
    if (waiter) waiter({ done: false, value });
    else this.#values.push(value);
  }

  next(): Promise<IteratorResult<T>> {
    if (this.#values.length > 0) {
      return Promise.resolve({ done: false, value: this.#values.shift() as T });
    }
    if (this.#ended) return Promise.resolve({ done: true, value: undefined });
    return new Promise((resolve) => this.#waiters.push(resolve));
  }

  end(): void {
    if (this.#ended) return;
    this.#ended = true;
    for (const waiter of this.#waiters.splice(0)) waiter({ done: true, value: undefined });
  }
}

function streamId(sessionId: string): string {
  return `session:${sessionId}`;
}

function reject(
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

function clone<T>(value: T): T {
  return structuredClone(value);
}

function errorMessage(cause: unknown): string {
  return cause instanceof Error ? cause.message : "Agent failed";
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
