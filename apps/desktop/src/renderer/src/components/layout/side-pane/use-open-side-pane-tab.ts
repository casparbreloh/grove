import { useCallback } from "react";

import type { OpenableSidePaneTab } from "@/lib/mock-side-pane-state";
import { openMockSidePaneTab } from "@/lib/mock-side-pane-state";
import { useSidePaneLayout } from "./side-pane-layout";

export function useOpenSidePaneTab() {
  const { openSidePane } = useSidePaneLayout();

  return useCallback(
    (sidePaneTab: OpenableSidePaneTab) => {
      openMockSidePaneTab(sidePaneTab);
      openSidePane();
    },
    [openSidePane],
  );
}
