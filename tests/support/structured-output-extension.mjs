import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const extensionPath = process.argv[2];
assert(extensionPath, "usage: node structured-output-extension.mjs <extension.ts>");
const source = await readFile(extensionPath, "utf8");
const moduleUrl = `data:text/javascript;base64,${Buffer.from(source).toString("base64")}`;
const { default: structuredOutput } = await import(moduleUrl);

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
