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
  readonly models: readonly ModelSummary[];
  readonly model: ModelSummary;
  readonly thinkingLevel: ThinkingLevel;
  run(sink: AgentSessionSink, text: string): Promise<AgentRunResult>;
  abort(): Promise<boolean>;
  selectModel(ref: ModelRef): Promise<boolean>;
  setThinkingLevel(level: ThinkingLevel): Promise<boolean>;
  shutdown(): Promise<void>;
}
