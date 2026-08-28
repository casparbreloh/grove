import { createFileRoute } from "@tanstack/react-router";

import { PaneWorkspace } from "@/components/layout/tabs/pane-workspace";

export const Route = createFileRoute("/_app/")({ component: App });

function App() {
  return <PaneWorkspace />;
}
