import type { ChatApi } from "../../shared/chat-ipc";

declare global {
  interface Window {
    grove: {
      chat: ChatApi;
      platform: string;
    };
  }
}

export {};
