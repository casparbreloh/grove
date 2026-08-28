export type EnvironmentKind = "local" | "development-machine" | "cloudflare";
export type ThinkingLevel = "off" | "minimal" | "low" | "medium" | "high" | "xhigh" | "max";

export interface ModelRef {
  agentId: string;
  providerId: string;
  modelId: string;
}

export interface ModelSummary {
  ref: ModelRef;
  name: string;
  contextWindow: number;
  input: readonly ("text" | "image")[];
  thinkingLevels: readonly ThinkingLevel[];
}

export type MessagePart =
  | { type: "text"; text: string }
  | { type: "reasoning"; text: string }
  | { type: "tool-call"; callId: string; name: string; input: JsonValue }
  | { type: "tool-result"; callId: string; name: string; output: string; isError: boolean };

export type GroveMessage =
  | {
      id: string;
      role: "user";
      createdAt: number;
      parts: readonly MessagePart[];
    }
  | {
      id: string;
      role: "assistant";
      createdAt: number;
      model: ModelRef;
      parts: readonly MessagePart[];
    };

export interface TurnSummary {
  id: string;
  outcome: "completed" | "aborted" | "failed";
  error?: { code: string; message: string };
}

export interface SessionSnapshot {
  id: string;
  taskId: string;
  phase: { type: "idle" } | { type: "running"; turnId: string };
  model: ModelRef;
  thinkingLevel: ThinkingLevel;
  messages: readonly GroveMessage[];
  lastTurn?: TurnSummary;
  capabilities: {
    prompt: boolean;
    abort: boolean;
    selectModel: boolean;
    setThinkingLevel: boolean;
  };
}

export interface TaskSummary {
  id: string;
  projectId: string;
  environment: { id: string; kind: EnvironmentKind };
}

export type GroveCursor = Readonly<Record<string, number>>;

export interface GroveBootstrap {
  task: TaskSummary;
  session: SessionSnapshot;
  models: readonly ModelSummary[];
  cursor: GroveCursor;
}

export type GroveCommand =
  | { type: "session.prompt"; commandId: string; sessionId: string; text: string }
  | { type: "session.abort"; commandId: string; sessionId: string }
  | { type: "session.select-model"; commandId: string; sessionId: string; model: ModelRef }
  | {
      type: "session.set-thinking-level";
      commandId: string;
      sessionId: string;
      thinkingLevel: ThinkingLevel;
    };

export interface CommandRejected {
  ok: false;
  type: GroveCommand["type"];
  commandId: string;
  error: {
    code: "busy" | "invalid-input" | "not-found" | "unsupported";
    message: string;
  };
}

export interface PromptCommandResult {
  ok: true;
  type: "session.prompt";
  commandId: string;
  turnId: string;
  status: "accepted";
}

export interface AbortCommandResult {
  ok: true;
  type: "session.abort";
  commandId: string;
  outcome: "requested" | "idle";
}

export interface SelectModelCommandResult {
  ok: true;
  type: "session.select-model";
  commandId: string;
  model: ModelRef;
}

export interface SetThinkingLevelCommandResult {
  ok: true;
  type: "session.set-thinking-level";
  commandId: string;
  thinkingLevel: ThinkingLevel;
}

export interface CommandResults {
  "session.prompt": PromptCommandResult | CommandRejected;
  "session.abort": AbortCommandResult | CommandRejected;
  "session.select-model": SelectModelCommandResult | CommandRejected;
  "session.set-thinking-level": SetThinkingLevelCommandResult | CommandRejected;
}

export type ResultFor<TCommand extends GroveCommand> = CommandResults[TCommand["type"]];
export type GroveCommandResult = CommandResults[keyof CommandResults];

export type GroveDurableEvent = { type: "session.updated"; session: SessionSnapshot };

export type GroveProgress =
  | {
      type: "message.text-delta";
      sessionId: string;
      turnId: string;
      messageId: string;
      delta: string;
    }
  | {
      type: "message.reasoning-delta";
      sessionId: string;
      turnId: string;
      messageId: string;
      delta: string;
    }
  | {
      type: "tool.started";
      sessionId: string;
      turnId: string;
      messageId: string;
      callId: string;
      name: string;
    }
  | {
      type: "tool.settled";
      sessionId: string;
      turnId: string;
      messageId: string;
      callId: string;
      name: string;
      isError: boolean;
    };

export type GroveUpdate =
  | {
      kind: "event";
      streamId: string;
      sequence: number;
      occurredAt: number;
      event: GroveDurableEvent;
    }
  | { kind: "progress"; progress: GroveProgress };

export interface GroveWatchOptions {
  after?: GroveCursor;
}

export interface GroveClient {
  bootstrap(): Promise<GroveBootstrap>;
  execute<TCommand extends GroveCommand>(command: TCommand): Promise<ResultFor<TCommand>>;
  watch(options?: GroveWatchOptions): AsyncIterable<GroveUpdate>;
}

export type JsonValue =
  | null
  | boolean
  | number
  | string
  | JsonValue[]
  | { [key: string]: JsonValue };
