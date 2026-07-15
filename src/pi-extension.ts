import { spawn } from "node:child_process";
import { unlinkSync, writeFileSync } from "node:fs";

export default function grove(pi: any): void {
  let named = false;

  pi.on("input", (event: any, ctx: any) => {
    const executable = process.env.GROVE_EXECUTABLE;
    if (event.source !== "interactive") {
      return { action: "continue" };
    }

    const claim = process.env.GROVE_NAMING_CLAIM;
    if (named || !executable || !claim) {
      return { action: "continue" };
    }

    try {
      writeFileSync(claim, "");
    } catch {
      ctx.ui.notify("Grove could not begin naming this worktree", "error");
      return { action: "continue" };
    }
    named = true;
    let settled = false;
    const failed = (): void => {
      if (settled) return;
      settled = true;
      named = false;
      try {
        unlinkSync(claim);
      } catch {}
      ctx.ui.notify("Grove could not name this worktree", "error");
    };
    const child = spawn(executable, ["__name"], {
      cwd: process.cwd(),
      env: process.env,
      stdio: ["pipe", "ignore", "ignore"],
    });
    child.once("error", failed);
    child.once("exit", (code) => {
      if (code !== 0) failed();
      else settled = true;
    });
    child.stdin.on("error", () => {
      child.kill();
    });
    child.stdin.end(event.text);
    child.unref();
    return { action: "continue" };
  });
}
