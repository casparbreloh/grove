import { createFileRoute } from "@tanstack/react-router";
import { renderMainPaneTabContent } from "@/components/layout/tabs/main-pane-tabs";
import { useMockGrove } from "@/lib/mock";

export const Route = createFileRoute("/_app/")({ component: App });

function App() {
  const { activeTabId: activeMainPaneTabId, tabs: mainPaneTabs } = useMockGrove();

  return (
    <div className="min-h-0 flex-1">
      {mainPaneTabs.map((mainPaneTab) => (
        <div
          className={mainPaneTab.tabId === activeMainPaneTabId ? "h-full" : "hidden"}
          key={mainPaneTab.tabId}
          role="tabpanel"
        >
          {renderMainPaneTabContent(mainPaneTab)}
        </div>
      ))}
    </div>
  );
}
