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

  function runtime(reason, initialEntries = [], initialName) {
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
      getSessionName: () => name,
      setSessionName(value) {
        name = value;
      },
    };
    const ctx = {
      sessionManager: {
        getSessionId: () => "session-456",
        getEntries: () => entries,
      },
    };
    changeSession(pi, (...args) => {
      spawnCount += 1;
      return spawn(...args);
    });
    handlers.get("session_start")({ reason }, ctx);
    return {
      entries,
      input: (text) =>
        handlers.get("input")({ source: "interactive", text }, ctx),
      name: () => name,
    };
  }

  async function waitFor(predicate) {
    const deadline = Date.now() + 2000;
    while (!predicate()) {
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
    const resume = runtime("resume");
    resume.input("Do not rename resumed session");
    assert.equal(spawnCount, 1);

    const linked = runtime("startup", startup.entries, "Existing Session Name");
    assert.equal(linked.entries.length, 1);
    linked.input("Do not replace existing name");
    assert.equal(spawnCount, 1);
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
