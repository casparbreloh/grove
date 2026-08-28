import { createPiAgentSession, type PiAgentSessionOptions } from "./pi-agent-session.ts";
import { createEffectGroveClient, type DirectGroveClientOptions } from "./runtime.ts";
import type { GroveClient } from "./types.ts";

export type {
  GroveClient,
  GroveCommand,
  GroveCommandResult,
  GroveCursor,
  GroveMessage,
  GroveProgress,
  GroveSync,
  GroveUpdate,
  GroveWatchOptions,
  MessagePart,
  ModelRef,
  ModelSummary,
  ResultFor,
  SessionSnapshot,
  ThinkingLevel,
} from "./types.ts";

export interface CreateGroveClientOptions extends PiAgentSessionOptions, DirectGroveClientOptions {}

export async function createGroveClient(
  options: CreateGroveClientOptions = {},
): Promise<GroveClient> {
  return createEffectGroveClient(createPiAgentSession(options), options);
}
