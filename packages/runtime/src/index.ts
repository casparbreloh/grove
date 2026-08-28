import { createPiAgentSession, type PiAgentSessionOptions } from "./pi-agent-session.ts";
import { createDirectGroveClient, type DirectGroveClientOptions } from "./runtime.ts";
import type { GroveClient } from "./types.ts";

export type {
  GroveBootstrap,
  GroveClient,
  GroveCommand,
  GroveCommandResult,
  GroveCursor,
  GroveMessage,
  GroveProgress,
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
  const agent = await createPiAgentSession(options);
  return createDirectGroveClient(agent, options);
}
