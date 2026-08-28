import { beforeEach, describe, expect, it, vi } from "vitest";

import type { SessionManager } from "@earendil-works/pi-coding-agent";

const pi = vi.hoisted(() => {
  const bindExtensions = vi.fn(async () => undefined);
  const setRebindSession = vi.fn();
  const dispose = vi.fn(async () => undefined);
  const session = { bindExtensions, model: { id: "test-model" } };
  const runtime = { dispose, session, setRebindSession };

  return {
    bindExtensions,
    createAgentSessionFromServices: vi.fn(async () => ({ session })),
    createAgentSessionRuntime: vi.fn(async () => runtime),
    createAgentSessionServices: vi.fn(async () => ({
      diagnostics: [],
      modelRuntime: { getModel: vi.fn() },
    })),
    createSessionManager: vi.fn(() => ({ kind: "session-manager" })),
    dispose,
    runtime,
    setRebindSession,
  };
});

vi.mock("@earendil-works/pi-coding-agent", () => ({
  createAgentSessionFromServices: pi.createAgentSessionFromServices,
  createAgentSessionRuntime: pi.createAgentSessionRuntime,
  createAgentSessionServices: pi.createAgentSessionServices,
  getAgentDir: vi.fn(() => "/agent"),
  SessionManager: { create: pi.createSessionManager },
}));

import { createAgent } from "./agent.ts";

describe("createAgent", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("returns Pi's runtime and rebinds extensions after Pi replaces its session", async () => {
    const runtime = await createAgent({ cwd: "/project" });

    expect(runtime).toBe(pi.runtime);
    expect(pi.bindExtensions).toHaveBeenCalledWith({});
    expect(pi.setRebindSession).toHaveBeenCalledOnce();

    const rebind = pi.setRebindSession.mock.calls[0]?.[0];
    const replacement = { bindExtensions: vi.fn(async () => undefined) };
    expect(rebind).toBeTypeOf("function");
    await rebind?.(replacement);

    expect(replacement.bindExtensions).toHaveBeenCalledWith({});
  });

  it("uses Pi's session cwd and selected agent directory consistently", async () => {
    const sessionManager = { getCwd: vi.fn(() => "/task") };

    await createAgent({
      agentDir: "/grove-agent",
      sessionManager: sessionManager as unknown as SessionManager,
    });

    expect(pi.createAgentSessionRuntime).toHaveBeenCalledWith(expect.any(Function), {
      agentDir: "/grove-agent",
      cwd: "/task",
      sessionManager,
    });
  });

  it("rejects a cwd that disagrees with Pi's session", async () => {
    const sessionManager = { getCwd: vi.fn(() => "/another-task") };

    await expect(
      createAgent({
        cwd: "/task",
        sessionManager: sessionManager as unknown as SessionManager,
      }),
    ).rejects.toThrow("Agent cwd does not match its Pi session");
  });

  it("requires Pi to own session placement for a custom agent directory", async () => {
    await expect(createAgent({ agentDir: "/grove-agent", cwd: "/task" })).rejects.toThrow(
      "A custom Pi agent directory requires an explicit Pi session manager",
    );
  });
});
