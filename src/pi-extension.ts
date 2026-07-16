import { spawn } from "node:child_process";

const LINK_TYPE = "grove";
const LINK_SCHEMA = 1;
const NAMING_LIFECYCLES = new Set(["startup", "new", "fork"]);

export default function grove(pi: any): void {
  let currentSessionId: string | undefined;
  let armedSessionId: string | undefined;

  pi.on("session_start", (event: any, ctx: any) => {
    const sessionId = ctx.sessionManager.getSessionId();
    currentSessionId = sessionId;
    armedSessionId = undefined;
    if (!NAMING_LIFECYCLES.has(event.reason)) return;

    const changeId = process.env.GROVE_CHANGE_ID;
    if (!changeId) return;

    const linked = ctx.sessionManager.getEntries().some(
      (entry: any) =>
        entry.type === "custom" &&
        entry.customType === LINK_TYPE &&
        entry.data?.schema === LINK_SCHEMA &&
        entry.data?.changeId === changeId,
    );
    if (!linked) {
      pi.appendEntry(LINK_TYPE, { schema: LINK_SCHEMA, changeId });
    }
    if (!pi.getSessionName()) {
      armedSessionId = sessionId;
    }
  });

  pi.on("session_shutdown", () => {
    currentSessionId = undefined;
    armedSessionId = undefined;
  });

  pi.on("input", (event: any, ctx: any) => {
    if (event.source !== "interactive") return { action: "continue" };
    const prompt = String(event.text ?? "").trim();
    const executable = process.env.GROVE_EXECUTABLE;
    const changeId = process.env.GROVE_CHANGE_ID;
    const capturedSessionId = armedSessionId;
    if (
      !executable ||
      !changeId ||
      !capturedSessionId ||
      prompt.length < 3 ||
      prompt.startsWith("/")
    ) {
      return { action: "continue" };
    }

    armedSessionId = undefined;
    let stdout = "";
    const child = spawn(
      executable,
      ["__title", "--change", changeId, "--session", capturedSessionId],
      {
        cwd: process.cwd(),
        env: process.env,
        stdio: ["pipe", "pipe", "ignore"],
      },
    );
    child.stdout.setEncoding("utf8");
    child.stdout.on("data", (chunk: string) => {
      stdout += chunk;
      if (stdout.length > 256) child.kill();
    });
    child.once("close", (code) => {
      const title = stdout.trim();
      if (
        code === 0 &&
        title.length > 0 &&
        !title.includes("\n") &&
        !title.includes("\r") &&
        currentSessionId === capturedSessionId
      ) {
        try {
          if (
            ctx.sessionManager.getSessionId() === capturedSessionId &&
            !pi.getSessionName()
          ) {
            pi.setSessionName(title);
          }
        } catch {}
      }
    });
    child.on("error", () => {});
    child.stdin.on("error", () => child.kill());
    child.stdin.end(prompt);
    (child.stdout as any).unref?.();
    child.unref();
    return { action: "continue" };
  });
}
