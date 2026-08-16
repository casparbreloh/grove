import {
  createAgentSession,
  DefaultResourceLoader,
  getAgentDir,
  ModelRuntime,
  SessionManager,
  SettingsManager,
  type AgentSession,
} from "@earendil-works/pi-coding-agent";
import type { ChatEvent } from "../shared/chat-ipc";

const provider = "openai-codex";
const modelId = "gpt-5.6-luna";

type PiChatOptions = {
  onEvent: (event: ChatEvent) => void;
};

export function createPiChat({ onEvent }: PiChatOptions) {
  const cwd = process.cwd();
  let cancelling = false;
  let disposed = false;
  let sessionPromise: Promise<AgentSession> | undefined;
  let unsubscribe: (() => void) | undefined;

  const emit = (event: ChatEvent) => {
    if (!disposed) onEvent(event);
  };

  const getSession = () => {
    if (sessionPromise) return sessionPromise;

    const pending = (async () => {
      const modelRuntime = await ModelRuntime.create();
      const model = modelRuntime.getModel(provider, modelId);
      if (!model) throw new Error(`Pi model ${provider}/${modelId} is unavailable`);

      const settingsManager = SettingsManager.inMemory();
      const resourceLoader = new DefaultResourceLoader({
        cwd,
        agentDir: getAgentDir(),
        settingsManager,
        noContextFiles: true,
        noExtensions: true,
        noPromptTemplates: true,
        noSkills: true,
        noThemes: true,
      });
      await resourceLoader.reload();

      const { session } = await createAgentSession({
        cwd,
        model,
        modelRuntime,
        noTools: "all",
        resourceLoader,
        sessionManager: SessionManager.inMemory(cwd),
        settingsManager,
      });

      if (disposed) {
        session.dispose();
        throw new Error("Pi chat was disposed");
      }

      unsubscribe = session.subscribe((event) => {
        if (event.type === "message_update" && event.assistantMessageEvent.type === "text_delta")
          emit({ type: "text-delta", delta: event.assistantMessageEvent.delta });
      });

      return session;
    })();

    sessionPromise = pending;
    void pending.catch(() => {
      if (sessionPromise === pending) sessionPromise = undefined;
    });
    return pending;
  };

  return {
    async send(text: string) {
      cancelling = false;
      const session = await getSession();
      if (disposed) return;
      if (cancelling) return;
      if (session.isStreaming) throw new Error("Pi is already responding");

      try {
        await session.prompt(text);
      } catch (error) {
        if (cancelling) return;
        throw error;
      }
      if (disposed) return;

      const lastAssistantMessage = session.messages
        .slice()
        .reverse()
        .find((message) => message.role === "assistant");
      if (lastAssistantMessage?.role === "assistant" && lastAssistantMessage.stopReason === "error")
        throw new Error(lastAssistantMessage.errorMessage ?? "Pi failed to respond");
    },
    async cancel() {
      if (disposed) return;
      cancelling = true;
      if (sessionPromise) await (await sessionPromise).abort();
    },
    dispose() {
      if (disposed) return;
      disposed = true;
      const pending = sessionPromise;
      sessionPromise = undefined;
      unsubscribe?.();
      unsubscribe = undefined;
      if (pending)
        void pending
          .then(
            async (session) => {
              try {
                await session.abort();
              } finally {
                session.dispose();
              }
            },
            () => {},
          )
          .catch(() => {});
    },
  };
}
