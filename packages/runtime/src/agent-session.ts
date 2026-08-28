import { Cause, Effect } from "effect";

import type { MessagePart, ModelRef, ModelSummary, ThinkingLevel } from "./types.ts";

export type AgentSessionProgress =
  | { type: "message.text-delta"; delta: string }
  | { type: "message.reasoning-delta"; delta: string }
  | { type: "tool.started"; callId: string; name: string }
  | { type: "tool.settled"; callId: string; name: string; isError: boolean };

export interface AgentSessionSink {
  progress(progress: AgentSessionProgress): void;
}

export interface AgentRunResult {
  outcome: "completed" | "aborted" | "failed";
  parts: readonly MessagePart[];
  error?: { code: string; message: string };
}

export interface AgentSessionCapabilities {
  prompt: boolean;
  abort: boolean;
  selectModel: boolean;
  setThinkingLevel: boolean;
}

export interface AgentSessionService {
  readonly capabilities: AgentSessionCapabilities;
  readonly models: Effect.Effect<readonly ModelSummary[]>;
  readonly model: Effect.Effect<ModelSummary>;
  readonly thinkingLevel: Effect.Effect<ThinkingLevel>;
  run(sink: AgentSessionSink, text: string): Effect.Effect<AgentRunResult, Cause.UnknownError>;
  readonly abort: Effect.Effect<boolean>;
  selectModel(ref: ModelRef): Effect.Effect<boolean>;
  setThinkingLevel(level: ThinkingLevel): Effect.Effect<boolean>;
  readonly shutdown: Effect.Effect<void>;
}
