import { useMockGrove } from "@/lib/mocks/grove";
import { TabContent } from "./tab-content";

export function TabWorkspace() {
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
