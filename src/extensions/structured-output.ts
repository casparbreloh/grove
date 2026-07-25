export default function structuredOutput(pi) {
  pi.registerFlag("structured-output-schema", {
    description: "JSON schema for the structured_output tool",
    type: "string",
  });

  const serializedSchema = pi.getFlag("structured-output-schema");
  if (typeof serializedSchema !== "string") return;

  const schema = JSON.parse(serializedSchema);
  if (!schema || typeof schema !== "object" || Array.isArray(schema)) {
    throw new Error("--structured-output-schema must contain a JSON object");
  }

  pi.registerTool({
    name: "structured_output",
    label: "Structured Output",
    description:
      "Return the final answer in the requested structure. Use this as the last action.",
    parameters: schema,
    async execute(_toolCallId, params) {
      return {
        content: [{ type: "text", text: "Structured output received" }],
        details: params,
        terminate: true,
      };
    },
  });
}
