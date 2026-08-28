import { createFileRoute } from "@tanstack/react-router";

import { TabWorkspace } from "@/components/layout/tabs/tab-workspace";

export const Route = createFileRoute("/_app/")({ component: App });

function App() {
  return <TabWorkspace />;
}
