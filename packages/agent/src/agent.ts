import {
  type AgentSessionRuntime,
  createAgentSessionFromServices,
  createAgentSessionRuntime,
  createAgentSessionServices,
  type CreateAgentSessionFromServicesOptions,
  type CreateAgentSessionRuntimeFactory,
  type CreateAgentSessionServicesOptions,
  getAgentDir,
  SessionManager,
} from "@earendil-works/pi-coding-agent";
import { resolve } from "node:path";

import { localExtension } from "./extensions/local/index.ts";

type AgentServicesOptions = Omit<CreateAgentSessionServicesOptions, "agentDir" | "cwd">;
type AgentSessionOptions = Omit<
  CreateAgentSessionFromServicesOptions,
  "services" | "sessionManager" | "sessionStartEvent"
>;

export interface AgentOptions {
  cwd?: string;
  agentDir?: string;
  sessionManager?: SessionManager;
  services?: AgentServicesOptions;
  session?: AgentSessionOptions;
}

/** Creates Grove's configured Pi agent and returns Pi's runtime unchanged. */
export async function createAgent(options: AgentOptions = {}): Promise<AgentSessionRuntime> {
  const sessionCwd = options.sessionManager?.getCwd();
  if (options.cwd && sessionCwd && resolve(options.cwd) !== resolve(sessionCwd)) {
    throw new Error(
      `Agent cwd does not match its Pi session: ${resolve(options.cwd)} !== ${resolve(sessionCwd)}`,
    );
  }
  if (options.agentDir && !options.sessionManager) {
    throw new Error("A custom Pi agent directory requires an explicit Pi session manager");
  }

  const cwd = sessionCwd ?? options.cwd ?? process.cwd();
  const agentDir = options.agentDir ?? getAgentDir();
  const createRuntime: CreateAgentSessionRuntimeFactory = async ({
    cwd: sessionCwd,
    sessionManager,
    sessionStartEvent,
  }) => {
    const resourceLoaderOptions = options.services?.resourceLoaderOptions;
    const services = await createAgentSessionServices({
      ...options.services,
      cwd: sessionCwd,
      agentDir,
      resourceLoaderOptions: {
        ...resourceLoaderOptions,
        extensionFactories: [localExtension, ...(resourceLoaderOptions?.extensionFactories ?? [])],
      },
    });
    const result = await createAgentSessionFromServices({
      ...options.session,
      services,
      sessionManager,
      sessionStartEvent,
    });
    return { ...result, services, diagnostics: services.diagnostics };
  };
  const runtime = await createAgentSessionRuntime(createRuntime, {
    cwd,
    agentDir,
    sessionManager: options.sessionManager ?? SessionManager.create(cwd),
  });

  if (!runtime.session.model) {
    await runtime.dispose();
    throw new Error("Agent has no models available");
  }
  try {
    const bindExtensions = (session: AgentSessionRuntime["session"]) => session.bindExtensions({});
    runtime.setRebindSession(bindExtensions);
    await bindExtensions(runtime.session);
  } catch (error) {
    await runtime.dispose();
    throw error;
  }

  return runtime;
}
