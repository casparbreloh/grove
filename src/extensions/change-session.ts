import { spawn, spawnSync } from "node:child_process";

const LINK_TYPE = "grove.change";
const NAMING_LIFECYCLES = new Set(["startup", "new", "fork"]);

export default function changeSession(pi, spawnProcess = spawn, applyTitle = spawnSync) {
  let currentSessionId;
  let request;

  function stop() {
    const active = request;
    const child = active?.child;
    request = undefined;
    clearTimeout(active?.timer);
    if (!child || child.exitCode !== null) return;
    try {
      if (process.platform !== "win32" && child.pid) process.kill(-child.pid);
      else child.kill();
    } catch {}
  }

  function failed(ctx, active) {
    if (request !== active) return;
    request = undefined;
    ctx.ui?.notify?.("Grove could not name this session", "warning");
  }

  pi.on("session_start", (event, ctx) => {
    stop();
    currentSessionId = ctx.sessionManager.getSessionId();
    const changeId = process.env.GROVE_CHANGE_ID;
    if (!changeId) return;

    const linked = ctx.sessionManager.getEntries().some(
      (entry) =>
        entry.type === "custom" &&
        entry.customType === LINK_TYPE &&
        entry.data?.changeId === changeId,
    );
    if (!linked) pi.appendEntry(LINK_TYPE, { changeId });
    if (NAMING_LIFECYCLES.has(event.reason) && !pi.getSessionName()) {
      request = { changeId, sessionId: currentSessionId };
    }
  });

  pi.on("session_info_changed", (event) => {
    if (event.name) stop();
  });

  pi.on("session_shutdown", () => {
    currentSessionId = undefined;
    stop();
  });

  pi.on("input", (event, ctx) => {
    if (event.source !== "interactive") return { action: "continue" };
    const prompt = String(event.text ?? "").trim();
    const executable = process.env.GROVE_EXECUTABLE;
    const active = request;
    if (
      !active ||
      active.child ||
      !executable ||
      prompt.length < 3 ||
      prompt.startsWith("/")
    ) {
      return { action: "continue" };
    }

    let stdout = "";
    const child = spawnProcess(
      executable,
      ["__title", "--change", active.changeId, "--session", active.sessionId],
      {
        cwd: process.cwd(),
        env: process.env,
        stdio: ["pipe", "pipe", "ignore"],
        detached: process.platform !== "win32",
      },
    );
    active.child = child;
    active.timer = setTimeout(() => {
      if (request !== active) return;
      stop();
      ctx.ui?.notify?.("Grove could not name this session", "warning");
    }, 60_000);
    child.stdout.setEncoding("utf8");
    child.stdout.on("data", (chunk) => {
      stdout += chunk;
      if (stdout.length > 256) stop();
    });
    child.once("close", (code) => {
      clearTimeout(active.timer);
      const title = stdout.trim();
      if (
        code !== 0 ||
        !title ||
        title.includes("\n") ||
        title.includes("\r") ||
        request !== active ||
        currentSessionId !== active.sessionId
      ) {
        failed(ctx, active);
        return;
      }
      try {
        if (
          ctx.sessionManager.getSessionId() !== active.sessionId ||
          pi.getSessionName()
        ) {
          return;
        }
        const applied = applyTitle(
          executable,
          [
            "__title",
            "--change",
            active.changeId,
            "--session",
            active.sessionId,
            "--apply",
          ],
          {
            cwd: process.cwd(),
            env: process.env,
            input: title,
            stdio: ["pipe", "ignore", "ignore"],
            timeout: 5_000,
          },
        );
        if (applied.status === 0) pi.setSessionName(title);
        else ctx.ui?.notify?.("Grove could not save the session title", "warning");
      } finally {
        if (request === active) request = undefined;
      }
    });
    child.once("error", () => failed(ctx, active));
    child.stdin.once("error", stop);
    child.stdin.end(prompt);
    return { action: "continue" };
  });
}
