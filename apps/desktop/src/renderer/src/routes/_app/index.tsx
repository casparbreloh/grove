import { createFileRoute } from "@tanstack/react-router";

import { TabContent } from "@/components/layout/tabs/tab-content";
import { useMockGrove } from "@/lib/mocks/grove";

export const Route = createFileRoute("/_app/")({ component: App });

function App() {
  const { activeTabId, tabs } = useMockGrove();

  return (
    <div className="min-h-0 flex-1">
      {tabs.map((tab) => (
        <div
          aria-hidden={tab.tabId !== activeTabId}
          className={tab.tabId === activeTabId ? "h-full" : "hidden"}
          id={`${tab.tabId}-panel`}
          inert={tab.tabId !== activeTabId}
          key={tab.tabId}
        >
          <TabContent tab={tab} />
        </div>
      ))}
    </div>
  );
}
