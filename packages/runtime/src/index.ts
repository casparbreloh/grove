import { Agent } from "@earendil-works/pi-agent-core";
import { ModelRuntime } from "@earendil-works/pi-coding-agent";

export interface Harness {
  prompt(text: string, onText: (text: string) => void): Promise<void>;
  abort(): void;
}

export async function createHarness(): Promise<Harness> {
  const models = await ModelRuntime.create();
  const model = models.getModel("openai-codex", "gpt-5.6-sol");

  if (!model) throw new Error("Pi model not found");

  const agent = new Agent({
    initialState: {
      model,
      systemPrompt: "You are a concise software development agent.",
      thinkingLevel: "low",
      tools: [],
    },
    streamFn: models.streamSimple.bind(models),
  });

  return {
    async prompt(text, onText) {
      const unsubscribe = agent.subscribe((event) => {
        if (event.type === "message_update" && event.assistantMessageEvent.type === "text_delta") {
          onText(event.assistantMessageEvent.delta);
        }
      });

      try {
        await agent.prompt(text);
        if (agent.state.errorMessage) throw new Error(agent.state.errorMessage);
      } finally {
        unsubscribe();
      }
    },
    abort: () => agent.abort(),
  };
}
