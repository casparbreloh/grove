import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { chmod, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

const extensionPath = process.argv[2];
assert(extensionPath, "usage: node pi-extension.mjs <extension.ts>");
const source = await readFile(extensionPath, "utf8");
const moduleUrl = `data:text/javascript;base64,${Buffer.from(source).toString("base64")}`;
const { default: grove } = await import(moduleUrl);

const temporary = await mkdtemp(join(tmpdir(), "grove-extension-test-"));
const invocation = join(temporary, "invocation");
const executable = join(temporary, "title-generator");
await writeFile(
  executable,
  `#!/bin/sh
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
    getSessionName() {
      return name;
    },
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
  grove(pi, (...args) => {
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
      customType: "grove",
      data: { schema: 1, changeId: "change-123" },
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
  assert.equal(resume.entries.length, 1, "resume repairs a missing Grove link");
  assert.deepEqual(resume.input("Do not rename resumed session"), {
    action: "continue",
  });
  assert.equal(spawnCount, 1, "resume does not arm title inference");
  assert.equal(resume.name(), undefined);

  const linked = runtime("startup", startup.entries, "Existing Session Name");
  assert.equal(linked.entries.length, 1, "an existing Grove link is not duplicated");
  linked.input("Do not replace existing name");
  assert.equal(spawnCount, 1, "an existing name is not replaced");
  assert.equal(linked.name(), "Existing Session Name");
} finally {
  await rm(temporary, { recursive: true, force: true });
}
