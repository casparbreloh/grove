import { contextBridge, ipcRenderer } from "electron";
import { chatIpc, type ChatApi, type ChatEvent } from "../shared/chat-ipc";

const listeners = new Set<(event: ChatEvent) => void>();

ipcRenderer.on(chatIpc.event, (_event, value: unknown) => {
  if (typeof value !== "object" || value === null || !("type" in value)) return;
  for (const listener of listeners) listener(value as ChatEvent);
});

const chat = {
  send: (text) => ipcRenderer.invoke(chatIpc.send, text),
  cancel: () => ipcRenderer.invoke(chatIpc.cancel),
  onEvent(listener) {
    listeners.add(listener);
    return () => listeners.delete(listener);
  },
} satisfies ChatApi;

contextBridge.exposeInMainWorld("grove", {
  chat,
  platform: process.platform,
});
