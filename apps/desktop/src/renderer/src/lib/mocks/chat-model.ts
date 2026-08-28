import type { ChatModelAdapter } from "@assistant-ui/react";

function waitForChunk(abortSignal: AbortSignal) {
  if (abortSignal.aborted) return Promise.resolve(false);

  return new Promise<boolean>((resolve) => {
    let settled = false;
    let timeout: ReturnType<typeof setTimeout>;
    const finish = (shouldContinue: boolean) => {
      if (settled) return;
      settled = true;
      clearTimeout(timeout);
      abortSignal.removeEventListener("abort", onAbort);
      resolve(shouldContinue);
    };
    const onAbort = () => finish(false);

    timeout = setTimeout(() => finish(true), 24);
    abortSignal.addEventListener("abort", onAbort, { once: true });
    if (abortSignal.aborted) onAbort();
  });
}

export const mockChatModel: ChatModelAdapter = {
  async *run({ messages, abortSignal }) {
    const userMessage = messages.findLast((message) => message.role === "user");
    const prompt = userMessage?.content
      .filter((part) => part.type === "text")
      .map((part) => part.text)
      .join("\n\n")
      .trim();
    if (!prompt) throw new Error("A message is required");

    let text = "";
    for (const character of `You said: ${prompt}`) {
      if (!(await waitForChunk(abortSignal))) return;
      text += character;
      yield { content: [{ type: "text", text }] };
    }
  },
};
