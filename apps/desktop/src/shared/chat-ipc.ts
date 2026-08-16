export type ChatEvent = { type: "text-delta"; delta: string };

export type ChatApi = {
  send: (text: string) => Promise<void>;
  cancel: () => Promise<void>;
  onEvent: (listener: (event: ChatEvent) => void) => () => void;
};

export const chatIpc = {
  cancel: "chat:cancel",
  event: "chat:event",
  send: "chat:send",
} as const;
