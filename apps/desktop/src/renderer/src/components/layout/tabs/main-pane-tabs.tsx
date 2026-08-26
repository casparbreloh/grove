import { ChatIcon, TerminalIcon } from "@hugeicons/core-free-icons";

import type { GroveTab } from "@/lib/mock";
import { createMockChatTab, createMockTerminalTab } from "@/lib/mock";
import { Chat } from "./chat";
import { Terminal } from "./terminal";

export const mainPaneTabDefinitions = [
  { kind: "chat", label: "Chat", icon: ChatIcon, create: createMockChatTab },
  { kind: "terminal", label: "Terminal", icon: TerminalIcon, create: createMockTerminalTab },
] as const;

export function renderMainPaneTabContent(tab: GroveTab) {
  if (tab.kind === "terminal") return <Terminal terminalId={tab.terminalId} />;
  return <Chat />;
}
