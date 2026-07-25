import { spawn, spawnSync } from "node:child_process";

const LINK_TYPE = "grove.change";
const NAMING_LIFECYCLES = new Set(["startup", "new", "fork"]);
const RETRY_DELAYS = [1_000, 5_000];
const ATTEMPT_TIMEOUT = 60_000;

export default function changeSession(
  pi,
  spawnProcess = spawn,
  retryDelays = RETRY_DELAYS,
  applyTitle = spawnSync,
) {
  let currentSessionId;
  let naming;

  function terminate(child) {
    if (!child || child.exitCode !== null) return;
    const grouped = process.platform !== "win32" && child.pid;
    try {
      if (grouped) process.kill(-child.pid, "SIGTERM");
      else child.kill();
    } catch {}
    if (child.groveKillTimer) return;
    child.groveKillTimer = setTimeout(() => {
      if (child.exitCode !== null) return;
      try {
        if (grouped) process.kill(-child.pid, "SIGKILL");
        else child.kill("SIGKILL");
      } catch {}
    }, 1_000);
    child.groveKillTimer.unref?.();
    child.once("close", () => clearTimeout(child.groveKillTimer));
  }

  function stopNaming() {
    if (!naming) return;
    clearTimeout(naming.retryTimer);
    clearTimeout(naming.timeoutTimer);
    terminate(naming.child);
    naming = undefined;
  }

  pi.on("session_start", (event, ctx) => {
    stopNaming();
    const sessionId = ctx.sessionManager.getSessionId();
    currentSessionId = sessionId;

    const changeId = process.env.GROVE_CHANGE_ID;
    if (!changeId) return;

    const linked = ctx.sessionManager.getEntries().some(
      (entry) =>
        entry.type === "custom" &&
        entry.customType === LINK_TYPE &&
        entry.data?.changeId === changeId,
    );
    if (!linked) {
      pi.appendEntry(LINK_TYPE, { changeId });
    }
    if (NAMING_LIFECYCLES.has(event.reason) && !pi.getSessionName()) {
      naming = { sessionId, changeId, attempts: 0 };
    }
  });

  pi.on("session_info_changed", (event) => {
    if (event.name) stopNaming();
  });

  pi.on("session_shutdown", () => {
    currentSessionId = undefined;
    stopNaming();
  });

  function warn(ctx) {
    naming = undefined;
    ctx.ui?.notify?.(
      "Grove could not name this session after repeated attempts",
      "warning",
    );
  }

  function apply(ctx, request, executable, title) {
    if (
      naming !== request ||
      currentSessionId !== request.sessionId ||
      pi.getSessionName()
    ) {
      return;
    }
    request.applyAttempts = (request.applyAttempts ?? 0) + 1;
    try {
      const applied = applyTitle(
        executable,
        [
          "__title",
          "--change",
          request.changeId,
          "--session",
          request.sessionId,
          "--apply",
        ],
        {
          cwd: process.cwd(),
          env: process.env,
          input: title,
          stdio: ["pipe", "ignore", "ignore"],
          timeout: 5_000,
          killSignal: "SIGKILL",
        },
      );
      if (applied.status === 0) {
        pi.setSessionName(title);
        naming = undefined;
        return;
      }
    } catch {}

    const delay = retryDelays[request.applyAttempts - 1];
    if (delay === undefined) {
      warn(ctx);
    } else {
      request.retryTimer = setTimeout(
        () => apply(ctx, request, executable, title),
        delay,
      );
    }
  }

  function attempt(ctx) {
    const request = naming;
    const executable = process.env.GROVE_EXECUTABLE;
    if (
      !request ||
      !request.prompt ||
      request.child ||
      !executable ||
      currentSessionId !== request.sessionId ||
      pi.getSessionName()
    ) {
      return;
    }

    request.attempts += 1;
    let stdout = "";
    let finished = false;
    let child;

    const fail = () => {
      if (finished) return;
      finished = true;
      clearTimeout(request.timeoutTimer);
      if (naming !== request) return;
      request.child = undefined;
      const delay = retryDelays[request.attempts - 1];
      if (delay === undefined) {
        warn(ctx);
        return;
      }
      request.retryTimer = setTimeout(() => attempt(ctx), delay);
    };

    try {
      child = spawnProcess(
        executable,
        ["__title", "--change", request.changeId, "--session", request.sessionId],
        {
          cwd: process.cwd(),
          env: process.env,
          stdio: ["pipe", "pipe", "ignore"],
          detached: process.platform !== "win32",
        },
      );
    } catch {
      fail();
      return;
    }

    request.child = child;
    child.stdout.setEncoding("utf8");
    child.stdout.on("data", (chunk) => {
      stdout += chunk;
      if (stdout.length > 256) terminate(child);
    });
    child.once("close", (code) => {
      if (finished) return;
      clearTimeout(request.timeoutTimer);
      request.child = undefined;
      const title = stdout.trim();
      if (
        code === 0 &&
        title.length > 0 &&
        !title.includes("\n") &&
        !title.includes("\r") &&
        naming === request &&
        currentSessionId === request.sessionId
      ) {
        try {
          if (
            ctx.sessionManager.getSessionId() === request.sessionId &&
            !pi.getSessionName()
          ) {
            finished = true;
            apply(ctx, request, executable, title);
            return;
          }
        } catch {}
      }
      fail();
    });
    child.once("error", fail);
    child.stdin.once("error", () => terminate(child));
    child.stdin.end(request.prompt);
    request.timeoutTimer = setTimeout(() => terminate(child), ATTEMPT_TIMEOUT);
  }

  pi.on("input", (event, ctx) => {
    if (event.source !== "interactive") return { action: "continue" };
    const prompt = String(event.text ?? "").trim();
    if (
      !naming ||
      naming.prompt ||
      prompt.length < 3 ||
      prompt.startsWith("/")
    ) {
      return { action: "continue" };
    }

    naming.prompt = prompt;
    attempt(ctx);
    return { action: "continue" };
  });
}
