import { randomUUID } from "node:crypto";

import {
  Clock,
  Cause,
  Context,
  Deferred,
  Effect,
  Fiber,
  Layer,
  ManagedRuntime,
  PubSub,
  Queue,
  Ref,
  Semaphore,
  Stream,
} from "effect";

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

interface GroveRuntimeService {
  readonly sync: Effect.Effect<GroveSync>;
  execute(command: GroveCommand): Effect.Effect<GroveCommandResult>;
  watch(options?: GroveWatchOptions, ready?: Deferred.Deferred<void>): Stream.Stream<GroveUpdate>;
}

class AgentSession extends Context.Service<AgentSession, AgentSessionService>()(
  "@grove/runtime/AgentSession",
) {}

class GroveRuntime extends Context.Service<GroveRuntime, GroveRuntimeService>()(
  "@grove/runtime/GroveRuntime",
) {
  static readonly layerNoDeps: Layer.Layer<GroveRuntime, never, AgentSession | RuntimeOptions> =
    Layer.effect(
      GroveRuntime,
      Effect.gen(function* () {
        const agent = yield* AgentSession;
        const runtimeOptions = yield* RuntimeOptions;
        const scope = yield* Effect.scope;
        const gate = yield* Semaphore.make(1);
        const updates = yield* PubSub.unbounded<GroveUpdate>();
        yield* Effect.addFinalizer(() => PubSub.shutdown(updates));

        const taskId = runtimeOptions.taskId ?? (yield* Effect.sync(randomUUID));
        const sessionId = runtimeOptions.sessionId ?? (yield* Effect.sync(randomUUID));
        const model = yield* agent.model;
        const thinkingLevel = yield* agent.thinkingLevel;
        const state = yield* Ref.make<RuntimeState>({
          task: {
            id: taskId,
            projectId: runtimeOptions.projectId,
            environment: { id: runtimeOptions.environmentId, kind: "local" },
          },
          session: {
            id: sessionId,
            taskId,
            phase: { type: "idle" },
            model: model.ref,
            thinkingLevel,
            messages: [],
            capabilities: clone(agent.capabilities),
          },
          sequence: 0,
          history: [],
          receipts: new Map(),
        });

        const publishSession = Effect.fn("GroveRuntime.publishSession")(function* () {
          const occurredAt = yield* Clock.currentTimeMillis;
          const update = yield* Ref.modify(state, (current) => {
            const event = {
              kind: "event" as const,
              streamId: streamId(current.session.id),
              sequence: current.sequence + 1,
              occurredAt,
              event: { type: "session.updated" as const, session: clone(current.session) },
            };
            return [
              event,
              {
                ...current,
                sequence: event.sequence,
                history: [...current.history, event],
              },
            ];
          });
          yield* PubSub.publish(updates, update);
        });

        const publishProgress = (
          turnId: string,
          messageId: string,
          progress: AgentSessionProgress,
        ): void => {
          const common = { sessionId, turnId, messageId };
          let projected: GroveProgress;
          switch (progress.type) {
            case "message.text-delta":
            case "message.reasoning-delta":
            case "tool.started":
            case "tool.settled":
              projected = { ...common, ...progress };
              break;
          }
          PubSub.publishUnsafe(updates, { kind: "progress", progress: projected });
        };

        const settleTurn = Effect.fn("GroveRuntime.settleTurn")(function* (
          turnId: string,
          assistantMessageId: string,
          result: AgentRunResult,
        ) {
          const createdAt = yield* Clock.currentTimeMillis;
          yield* Ref.update(state, (current) => {
            const assistantMessage: GroveMessage = {
              id: assistantMessageId,
              role: "assistant",
              createdAt,
              model: current.session.model,
              parts: result.parts,
            };
            const lastTurn: TurnSummary = {
              id: turnId,
              outcome: result.outcome,
              ...(result.error ? { error: result.error } : {}),
            };
            return {
              ...current,
              session: {
                ...current.session,
                phase: { type: "idle" as const },
                messages:
                  result.parts.length > 0
                    ? [...current.session.messages, assistantMessage]
                    : current.session.messages,
                lastTurn,
              },
            };
          });
          yield* publishSession();
        });

        const runTurn = Effect.fn("GroveRuntime.runTurn")(function* (
          text: string,
          turnId: string,
          assistantMessageId: string,
        ) {
          const result = yield* agent
            .run(
              {
                progress: (progress) => publishProgress(turnId, assistantMessageId, progress),
              },
              text,
            )
            .pipe(
              Effect.catchTag("UnknownError", (error) =>
                Effect.succeed({
                  outcome: "failed" as const,
                  parts: [],
                  error: { code: "agent-failure", message: errorMessage(error.cause) },
                }),
              ),
            );
          yield* gate.withPermit(settleTurn(turnId, assistantMessageId, result));
        });

        const prompt = Effect.fn("GroveRuntime.prompt")(function* (
          command: Extract<GroveCommand, { type: "session.prompt" }>,
        ) {
          const current = yield* Ref.get(state);
          if (!current.session.capabilities.prompt) {
            return reject(command, "unsupported", "The Agent does not support prompting");
          }
          if (current.session.phase.type === "running") {
            return reject(command, "busy", "Session already has an active Turn");
          }
          const text = command.text.trim();
          if (!text) return reject(command, "invalid-input", "Prompt text is empty");

          const [turnId, assistantMessageId, userMessageId] = yield* Effect.all([
            Effect.sync(randomUUID),
            Effect.sync(randomUUID),
            Effect.sync(randomUUID),
          ]);
          const createdAt = yield* Clock.currentTimeMillis;
          const userMessage: GroveMessage = {
            id: userMessageId,
            role: "user",
            createdAt,
            parts: [{ type: "text", text }],
          };
          yield* Ref.update(state, (latest) => ({
            ...latest,
            session: {
              ...latest.session,
              phase: { type: "running" as const, turnId },
              messages: [...latest.session.messages, userMessage],
              lastTurn: undefined,
            },
          }));
          yield* publishSession();
          yield* Effect.forkIn(runTurn(text, turnId, assistantMessageId), scope, {
            startImmediately: true,
          });
          yield* Effect.yieldNow;

          return {
            ok: true,
            type: "session.prompt",
            commandId: command.commandId,
            turnId,
            status: "accepted",
          } satisfies PromptCommandResult;
        });

        const abort = Effect.fn("GroveRuntime.abort")(function* (
          command: Extract<GroveCommand, { type: "session.abort" }>,
        ) {
          const current = yield* Ref.get(state);
          if (!current.session.capabilities.abort) {
            return reject(command, "unsupported", "The Agent does not support aborting");
          }
          const requested = current.session.phase.type === "running" && (yield* agent.abort);
          return {
            ok: true,
            type: "session.abort",
            commandId: command.commandId,
            outcome: requested ? "requested" : "idle",
          } satisfies AbortCommandResult;
        });

        const selectModel = Effect.fn("GroveRuntime.selectModel")(function* (
          command: Extract<GroveCommand, { type: "session.select-model" }>,
        ) {
          const current = yield* Ref.get(state);
          if (!current.session.capabilities.selectModel) {
            return reject(command, "unsupported", "The Agent does not support model selection");
          }
          if (current.session.phase.type === "running") {
            return reject(command, "busy", "Cannot change model during an active Turn");
          }
          if (!(yield* agent.selectModel(command.model))) {
            return reject(
              command,
              "unsupported",
              `Model is unavailable: ${command.model.providerId}/${command.model.modelId}`,
            );
          }
          const selected = yield* agent.model;
          const selectedThinkingLevel = yield* agent.thinkingLevel;
          yield* Ref.update(state, (latest) => ({
            ...latest,
            session: {
              ...latest.session,
              model: selected.ref,
              thinkingLevel: selectedThinkingLevel,
            },
          }));
          yield* publishSession();
          return {
            ok: true,
            type: "session.select-model",
            commandId: command.commandId,
            model: selected.ref,
          } satisfies SelectModelCommandResult;
        });

        const setThinkingLevel = Effect.fn("GroveRuntime.setThinkingLevel")(function* (
          command: Extract<GroveCommand, { type: "session.set-thinking-level" }>,
        ) {
          const current = yield* Ref.get(state);
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
          if (!(yield* agent.setThinkingLevel(command.thinkingLevel))) {
            return reject(
              command,
              "unsupported",
              `Thinking level is unavailable: ${command.thinkingLevel}`,
            );
          }
          const selectedThinkingLevel = yield* agent.thinkingLevel;
          yield* Ref.update(state, (latest) => ({
            ...latest,
            session: { ...latest.session, thinkingLevel: selectedThinkingLevel },
          }));
          yield* publishSession();
          return {
            ok: true,
            type: "session.set-thinking-level",
            commandId: command.commandId,
            thinkingLevel: selectedThinkingLevel,
          } satisfies SetThinkingLevelCommandResult;
        });

        const executeNew = (command: GroveCommand): Effect.Effect<GroveCommandResult> => {
          switch (command.type) {
            case "session.prompt":
              return prompt(command);
            case "session.abort":
              return abort(command);
            case "session.select-model":
              return selectModel(command);
            case "session.set-thinking-level":
              return setThinkingLevel(command);
          }
        };

        const execute = Effect.fn("GroveRuntime.execute")((command: GroveCommand) =>
          gate.withPermit(
            Effect.gen(function* () {
              const current = yield* Ref.get(state);
              if (command.sessionId !== current.session.id) {
                return reject(command, "not-found", `Session not found: ${command.sessionId}`);
              }
              const fingerprint = commandFingerprint(command);
              const receipt = current.receipts.get(command.commandId);
              if (receipt) {
                return receipt.fingerprint === fingerprint
                  ? clone(receipt.result)
                  : reject(
                      command,
                      "invalid-input",
                      `Command ID was already used with different input: ${command.commandId}`,
                    );
              }

              const result = yield* executeNew(command);
              yield* Ref.update(state, (latest) => ({
                ...latest,
                receipts: new Map(latest.receipts).set(command.commandId, {
                  fingerprint,
                  result: clone(result),
                }),
              }));
              return clone(result);
            }),
          ),
        );

        const sync = Effect.fn("GroveRuntime.sync")(function* () {
          const current = yield* Ref.get(state);
          const models = yield* agent.models;
          return clone({
            task: current.task,
            session: current.session,
            models,
            cursor: { [streamId(current.session.id)]: current.sequence },
          });
        });

        const watch = (
          options: GroveWatchOptions = {},
          ready?: Deferred.Deferred<void>,
        ): Stream.Stream<GroveUpdate> =>
          Stream.unwrap(
            Effect.gen(function* () {
              const subscription = yield* PubSub.subscribe(updates);
              const snapshot = yield* Ref.get(state);
              const after = options.after?.[streamId(snapshot.session.id)] ?? 0;
              const replay = snapshot.history.filter((update) => update.sequence > after);
              const live = Stream.fromEffectRepeat(PubSub.take(subscription)).pipe(
                Stream.filter(
                  (update) => update.kind === "progress" || update.sequence > snapshot.sequence,
                ),
              );
              if (ready) yield* Deferred.succeed(ready, undefined);
              return Stream.concat(Stream.fromIterable(replay), live);
            }),
          ).pipe(Stream.map(clone));

        return GroveRuntime.of({ sync: sync(), execute, watch });
      }),
    );
}

export function createDirectGroveClient(
  agent: AgentSessionService,
  options: DirectGroveClientOptions = {},
): GroveClient {
  return createLayerGroveClient(agentLayer(Effect.succeed(agent)), options);
}

export async function createEffectGroveClient<E>(
  agent: Effect.Effect<AgentSessionService, E>,
  options: DirectGroveClientOptions = {},
): Promise<GroveClient> {
  const client = createLayerGroveClient(agentLayer(agent), options);
  try {
    await client.sync();
    return client;
  } catch (error) {
    await client.close();
    throw error;
  }
}

function agentLayer<E>(
  acquire: Effect.Effect<AgentSessionService, E>,
): Layer.Layer<AgentSession, E> {
  return Layer.effect(
    AgentSession,
    Effect.gen(function* () {
      const agent = yield* acquire;
      yield* Effect.addFinalizer(() => agent.shutdown);
      return AgentSession.of(agent);
    }),
  );
}

function createLayerGroveClient<E>(
  agent: Layer.Layer<AgentSession, E>,
  options: DirectGroveClientOptions,
): GroveClient {
  const layer = GroveRuntime.layerNoDeps.pipe(
    Layer.provide(agent),
    Layer.provide(
      Layer.succeed(
        RuntimeOptions,
        RuntimeOptions.of({
          environmentId: options.environmentId ?? "local",
          projectId: options.projectId ?? "grove-harness",
          ...(options.taskId ? { taskId: options.taskId } : {}),
          ...(options.sessionId ? { sessionId: options.sessionId } : {}),
        }),
      ),
    ),
  );
  const runtime = ManagedRuntime.make(layer);
  const pendingSubscriptions = new Set<Promise<void>>();

  return {
    sync: () => runtime.runPromise(GroveRuntime.use((service) => service.sync)),
    execute: async <TCommand extends GroveCommand>(command: TCommand) => {
      await Promise.all(pendingSubscriptions);
      return (await runtime.runPromise(
        GroveRuntime.use((service) => service.execute(command)),
      )) as ResultFor<TCommand>;
    },
    watch: (watchOptions = {}) => ({
      [Symbol.asyncIterator]: () => {
        const queue = Effect.runSync(Queue.unbounded<GroveUpdate, Cause.Done>());
        const ready = Effect.runSync(Deferred.make<void>());
        const readyPromise = runtime.runPromise(Deferred.await(ready));
        pendingSubscriptions.add(readyPromise);
        void readyPromise.finally(() => pendingSubscriptions.delete(readyPromise));
        const fiber = runtime.runFork(
          GroveRuntime.use((service) =>
            Stream.runForEach(service.watch(watchOptions, ready), (update) =>
              Queue.offer(queue, update).pipe(Effect.asVoid),
            ).pipe(Effect.ensuring(Queue.end(queue).pipe(Effect.asVoid))),
          ),
        );
        const iterator = Stream.toAsyncIterable(Stream.fromQueue(queue))[Symbol.asyncIterator]();
        return bridgeIterator(iterator, () => runtime.runPromise(Fiber.interrupt(fiber)));
      },
    }),
    close: () => runtime.dispose(),
  };
}

interface RuntimeOptionsService {
  readonly environmentId: string;
  readonly projectId: string;
  readonly taskId?: string;
  readonly sessionId?: string;
}

class RuntimeOptions extends Context.Service<RuntimeOptions, RuntimeOptionsService>()(
  "@grove/runtime/RuntimeOptions",
) {}

function bridgeIterator(
  source: AsyncIterator<GroveUpdate>,
  close: () => Promise<void>,
): AsyncIterator<GroveUpdate> {
  return {
    next: () => source.next(),
    return: async () => {
      await close();
      return source.return
        ? source.return()
        : ({ done: true, value: undefined } as IteratorReturnResult<undefined>);
    },
    throw: async (error?: unknown) => {
      await close();
      if (source.throw) return source.throw(error);
      throw error;
    },
  };
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
