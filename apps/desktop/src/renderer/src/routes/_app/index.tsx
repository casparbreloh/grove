import { createFileRoute } from "@tanstack/react-router";
import { renderTab } from "@/components/layout/tabs/registry";
import { useMockGrove } from "@/lib/mock";

export const Route = createFileRoute("/_app/")({ component: App });

function App() {
  const { activeTabId, tabs } = useMockGrove();

  return (
    <div className="min-h-0 flex-1">
      {tabs.map((tab) => (
        <div
          className={tab.tabId === activeTabId ? "h-full" : "hidden"}
          key={tab.tabId}
          role="tabpanel"
        >
          {renderTab(tab)}
        </div>
      ))}
    </div>
  );
}
