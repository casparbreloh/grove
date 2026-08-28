import { describe, expect, it, vi } from "vitest";

import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";

import { localExtension } from "./index.ts";

interface TestContext {
  ui: {
    setStatus: ReturnType<typeof vi.fn>;
    theme: { fg: ReturnType<typeof vi.fn> };
  };
}

type ExtensionHandler = (event: unknown, context: TestContext) => void;

describe("localExtension", () => {
  it("owns and clears Grove's Pi status entry", () => {
    const on = vi.fn();
    localExtension({ on } as unknown as ExtensionAPI);

    const registrations = on.mock.calls as unknown as Array<[string, ExtensionHandler]>;
    const start = registrations.find(([event]) => event === "session_start")?.[1];
    const shutdown = registrations.find(([event]) => event === "session_shutdown")?.[1];
    const context: TestContext = {
      ui: {
        setStatus: vi.fn(),
        theme: { fg: vi.fn((_tone: string, value: string) => value) },
      },
    };

    expect(start).toBeDefined();
    expect(shutdown).toBeDefined();
    start?.({}, context);
    shutdown?.({}, context);

    expect(context.ui.setStatus).toHaveBeenNthCalledWith(1, "grove", "Grove");
    expect(context.ui.setStatus).toHaveBeenNthCalledWith(2, "grove", undefined);
  });
});
