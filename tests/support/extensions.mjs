import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { chmod, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

const [changeSessionPath, structuredOutputPath] = process.argv.slice(2);
assert(
  changeSessionPath && structuredOutputPath,
  "usage: node extensions.mjs <change-session.ts> <structured-output.ts>",
);

async function importExtension(path) {
  const source = await readFile(path, "utf8");
  const moduleUrl = `data:text/javascript;base64,${Buffer.from(source).toString("base64")}`;
  return (await import(moduleUrl)).default;
}

async function testChangeSession() {
  const changeSession = await importExtension(changeSessionPath);
  const temporary = await mkdtemp(join(tmpdir(), "grove-extension-test-"));
  const invocation = join(temporary, "invocation");
  const executable = join(temporary, "title-generator");
  await writeFile(
    executable,
    `#!/bin/sh
case " $* " in
  *" --apply "*) cat >/dev/null; exit 0 ;;
esac
printf 'args=%s\\nprompt=' "$*" > "${invocation}"
cat >> "${invocation}"
printf 'Generated Session Title\\n'
`,
  );
  await chmod(executable, 0o700);

  process.env.GROVE_EXECUTABLE = executable;
  process.env.GROVE_CHANGE_ID = "change-123";
  let spawnCount = 0;

  function runtime(
    reason,
    initialEntries = [],
    initialName,
    spawnProcess,
    retryDelays = [0, 0],
    applyTitle,
  ) {
    const handlers = new Map();
    const entries = [...initialEntries];
    let name = initialName;
    const pi = {
      on(event, handler) {
        handlers.set(event, handler);
      },
      appendEntry(customType, data) {
        entries.push({ type: "custom", customType, data });
      },
      getSessionName() {
        return name;
      },
      setSessionName(value) {
        name = value;
      },
    };
    const notifications = [];
    const ctx = {
      sessionManager: {
        getSessionId: () => "session-456",
        getEntries: () => entries,
      },
      ui: {
        notify(message, level) {
          notifications.push({ message, level });
        },
      },
    };
    changeSession(
      pi,
      (...args) => {
        spawnCount += 1;
        return (spawnProcess ?? spawn)(...args);
      },
      retryDelays,
      applyTitle,
    );
    handlers.get("session_start")({ reason }, ctx);
    return {
      entries,
      input: (text) =>
        handlers.get("input")({ source: "interactive", text }, ctx),
      name: () => name,
      notifications,
      setName: (value) => {
        name = value;
        handlers.get("session_info_changed")?.({ name: value }, ctx);
      },
      shutdown: () => handlers.get("session_shutdown")?.({}, ctx),
    };
  }

  async function waitFor(predicate) {
    const deadline = Date.now() + 2000;
    while (!(await predicate())) {
      assert(Date.now() < deadline, "extension action did not complete");
      await new Promise((resolve) => setTimeout(resolve, 10));
    }
  }

  try {
    const startup = runtime("startup");
    assert.deepEqual(startup.entries, [
      {
        type: "custom",
        customType: "grove.change",
        data: { changeId: "change-123" },
      },
    ]);
    assert.deepEqual(startup.input("  Implement extension coverage  "), {
      action: "continue",
    });
    await waitFor(() => startup.name() === "Generated Session Title");
    assert.equal(
      await readFile(invocation, "utf8"),
      "args=__title --change change-123 --session session-456\nprompt=Implement extension coverage",
    );

    assert.equal(spawnCount, 1);

    let applyCount = 0;
    const applyRetry = runtime(
      "startup",
      [],
      undefined,
      undefined,
      [0, 0],
      () => ({ status: ++applyCount === 3 ? 0 : 1 }),
    );
    applyRetry.input("Retry only local title application");
    await waitFor(() => applyRetry.name() === "Generated Session Title");
    assert.equal(applyCount, 3, "local title application is retried");
    assert.equal(spawnCount, 2, "application retries do not repeat inference");

    const retryCounter = join(temporary, "retry-count");
    const flakyExecutable = join(temporary, "flaky-title-generator");
    await writeFile(
      flakyExecutable,
      `#!/bin/sh
case " $* " in
  *" --apply "*) cat >/dev/null; exit 0 ;;
esac
count=0
[ ! -f "${retryCounter}" ] || count=$(cat "${retryCounter}")
count=$((count + 1))
printf '%s' "$count" > "${retryCounter}"
cat >/dev/null
if [ "$count" -eq 1 ]; then
  printf 'Malformed\\nTitle\\n'
  exit 0
fi
[ "$count" -ge 3 ] || exit 17
printf 'Generated Session Title\\n'
`,
    );
    await chmod(flakyExecutable, 0o700);
    process.env.GROVE_EXECUTABLE = flakyExecutable;
    const retry = runtime("startup");
    retry.input("Retry transient title failures");
    await waitFor(() => retry.name() === "Generated Session Title");
    assert.equal(await readFile(retryCounter, "utf8"), "3");
    assert.deepEqual(retry.notifications, []);

    process.env.GROVE_EXECUTABLE = "/usr/bin/false";
    const exhausted = runtime("startup");
    exhausted.input("Bound persistent title failures");
    await waitFor(() => exhausted.notifications.length === 1);
    assert.deepEqual(exhausted.notifications, [
      {
        message: "Grove could not name this session after repeated attempts",
        level: "warning",
      },
    ]);
    assert.equal(spawnCount, 8, "naming has a session-wide attempt bound");
    exhausted.input("Do not start another naming cycle");
    await new Promise((resolve) => setTimeout(resolve, 20));
    assert.equal(spawnCount, 8, "later prompts do not create unbounded retries");

    const cancelled = runtime("startup", [], undefined, undefined, [100, 100]);
    cancelled.input("Cancel retries during shutdown");
    await waitFor(() => spawnCount === 9);
    cancelled.shutdown();
    await new Promise((resolve) => setTimeout(resolve, 150));
    assert.equal(spawnCount, 9, "shutdown cancels pending naming retries");

    const started = join(temporary, "long-running-started");
    const longRunning = join(temporary, "long-running-title-generator");
    await writeFile(
      longRunning,
      `#!/bin/sh
cat >/dev/null
printf '%s' $$ > "${started}"
trap '' TERM
sleep 10
printf 'Late Generated Title\\n'
`,
    );
    await chmod(longRunning, 0o700);
    process.env.GROVE_EXECUTABLE = longRunning;
    const manuallyNamed = runtime("startup");
    manuallyNamed.input("Do not replace a manual title");
    await waitFor(async () => {
      try {
        return /^\d+$/.test(await readFile(started, "utf8"));
      } catch {
        return false;
      }
    });
    const longRunningPid = Number(await readFile(started, "utf8"));
    manuallyNamed.setName("Manual Session Name");
    await new Promise((resolve) => setTimeout(resolve, 1_200));
    assert.throws(
      () => process.kill(longRunningPid, 0),
      (error) => error?.code === "ESRCH",
      "cancellation escalates when a worker ignores SIGTERM",
    );
    assert.equal(manuallyNamed.name(), "Manual Session Name");
    assert.deepEqual(manuallyNamed.notifications, []);
    assert.equal(spawnCount, 10, "manual naming cancels the in-flight worker");

    process.env.GROVE_EXECUTABLE = executable;
    const resume = runtime("resume");
    assert.equal(resume.entries.length, 1, "resume repairs a missing Change link");
    assert.deepEqual(resume.input("Do not rename resumed session"), {
      action: "continue",
    });
    assert.equal(spawnCount, 10, "resume does not arm title inference");
    assert.equal(resume.name(), undefined);

    const linked = runtime("startup", startup.entries, "Existing Session Name");
    assert.equal(linked.entries.length, 1, "an existing Change link is not duplicated");
    linked.input("Do not replace existing name");
    assert.equal(spawnCount, 10, "an existing name is not replaced");
    assert.equal(linked.name(), "Existing Session Name");
  } finally {
    await rm(temporary, { recursive: true, force: true });
  }
}

async function testStructuredOutput() {
  const structuredOutput = await importExtension(structuredOutputPath);
  const schema = {
    type: "object",
    properties: { change: { type: "string" } },
    required: ["change"],
    additionalProperties: false,
  };
  let flag;
  let tool;
  structuredOutput({
    registerFlag(name, options) {
      flag = { name, options };
    },
    getFlag(name) {
      assert.equal(name, "structured-output-schema");
      return JSON.stringify(schema);
    },
    registerTool(definition) {
      tool = definition;
    },
  });

  assert.equal(flag.name, "structured-output-schema");
  assert.equal(flag.options.type, "string");
  assert.equal(tool.name, "structured_output");
  assert.deepEqual(tool.parameters, schema);
  const result = await tool.execute("call-1", { change: "Name This Change" });
  assert.deepEqual(result.details, { change: "Name This Change" });
  assert.equal(result.terminate, true);
}

await testChangeSession();
await testStructuredOutput();
