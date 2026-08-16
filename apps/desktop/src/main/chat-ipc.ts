import { ipcMain, type IpcMainInvokeEvent, type WebContents } from "electron";
import { chatIpc, type ChatEvent } from "../shared/chat-ipc";
import { createPiChat } from "./pi";

type RegisterChatIpcOptions = {
  isRendererUrl: (url: string) => boolean;
};

export function registerChatIpc({ isRendererUrl }: RegisterChatIpcOptions) {
  let activeRun: { sender: WebContents } | undefined;
  let chat: ReturnType<typeof createPiChat> | undefined;

  const isTrustedSender = (event: IpcMainInvokeEvent) => {
    const frame = event.senderFrame;
    return Boolean(frame && frame === event.sender.mainFrame && isRendererUrl(frame.url));
  };

  const sendEvent = (event: ChatEvent) => {
    const sender = activeRun?.sender;
    if (sender && !sender.isDestroyed()) sender.send(chatIpc.event, event);
  };

  const getChat = () => (chat ??= createPiChat({ onEvent: sendEvent }));

  ipcMain.handle(chatIpc.send, async (event, value: unknown) => {
    if (!isTrustedSender(event)) throw new Error("Blocked chat request from an untrusted renderer");
    if (typeof value !== "string" || !value.trim())
      throw new TypeError("message must be a non-empty string");
    if (activeRun) throw new Error("Pi is already responding");

    const currentChat = getChat();
    const run = { sender: event.sender };
    activeRun = run;
    try {
      await currentChat.send(value);
    } finally {
      if (activeRun === run) activeRun = undefined;
    }
  });

  ipcMain.handle(chatIpc.cancel, async (event) => {
    if (!isTrustedSender(event)) throw new Error("Blocked chat request from an untrusted renderer");
    await chat?.cancel();
  });

  const reset = () => {
    activeRun = undefined;
    chat?.dispose();
    chat = undefined;
  };

  return { reset };
}
