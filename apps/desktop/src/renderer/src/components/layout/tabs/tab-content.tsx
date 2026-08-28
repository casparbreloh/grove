import type { Tab } from "@/lib/mocks/grove";
import { Chat } from "./chat";
import { Terminal } from "./terminal";

export function TabContent({ tab }: { tab: Tab }) {
  switch (tab.kind) {
    case "chat":
      return <Chat />;
    case "terminal":
      return <Terminal terminalId={tab.terminalId} />;
    case "unavailable":
      return (
        <div className="flex size-full items-center justify-center text-sm text-muted-foreground">
          {tab.originalKind} tabs are unavailable in this version of Grove.
        </div>
      );
  }
}
