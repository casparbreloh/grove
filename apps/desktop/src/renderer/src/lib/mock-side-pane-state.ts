import { useSyncExternalStore } from "react";

export type TerminalSidePaneTab = Readonly<{
  kind: "terminal";
  tabId: string;
  terminalId: string;
  title: string;
}>;

export type TestSidePaneTab = Readonly<{
  kind: "test";
  tabId: string;
  title: string;
}>;

type UnavailableSidePaneTab = Readonly<{
  kind: "unavailable";
  originalKind: string;
  tabId: string;
  title: string;
}>;

export type SidePaneTab = TerminalSidePaneTab | TestSidePaneTab | UnavailableSidePaneTab;
export type OpenableSidePaneTab = TerminalSidePaneTab | TestSidePaneTab;

type MockSidePaneTabsSnapshot = Readonly<{
  activeSidePaneTabId: string | undefined;
  sidePaneTabs: readonly SidePaneTab[];
}>;

const mockSidePaneTabListeners = new Set<() => void>();
let sidePaneTabs: readonly SidePaneTab[] = [];
let activeSidePaneTabId: string | undefined;
let nextSidePaneTabNumber = 1;
let nextSidePaneTerminalNumber = 1;
let nextSidePaneTestNumber = 1;

function createMockSidePaneTabsSnapshot(): MockSidePaneTabsSnapshot {
  return { activeSidePaneTabId, sidePaneTabs };
}

let mockSidePaneTabsSnapshot = createMockSidePaneTabsSnapshot();

function emitMockSidePaneTabsChange() {
  mockSidePaneTabsSnapshot = createMockSidePaneTabsSnapshot();
  for (const listener of mockSidePaneTabListeners) listener();
}

export function useMockSidePaneTabs() {
  return useSyncExternalStore(
    (listener) => {
      mockSidePaneTabListeners.add(listener);
      return () => mockSidePaneTabListeners.delete(listener);
    },
    () => mockSidePaneTabsSnapshot,
    () => mockSidePaneTabsSnapshot,
  );
}

export function selectMockSidePaneTab(tabId: string) {
  if (
    tabId === activeSidePaneTabId ||
    !sidePaneTabs.some((sidePaneTab) => sidePaneTab.tabId === tabId)
  ) {
    return;
  }
  activeSidePaneTabId = tabId;
  emitMockSidePaneTabsChange();
}

export function createMockSidePaneTerminalTab(): TerminalSidePaneTab {
  const terminalNumber = nextSidePaneTerminalNumber++;
  const terminalId = `side_terminal_${terminalNumber}`;

  return {
    kind: "terminal",
    tabId: `side_tab_${nextSidePaneTabNumber++}`,
    terminalId,
    title: terminalNumber === 1 ? "Terminal" : `Terminal ${terminalNumber}`,
  };
}

export function createMockSidePaneTestTab(): TestSidePaneTab {
  const testNumber = nextSidePaneTestNumber++;
  return {
    kind: "test",
    tabId: `side_tab_${nextSidePaneTabNumber++}`,
    title: testNumber === 1 ? "Test" : `Test ${testNumber}`,
  };
}

export function openMockSidePaneTab(sidePaneTab: OpenableSidePaneTab) {
  sidePaneTabs = [...sidePaneTabs, sidePaneTab];
  activeSidePaneTabId = sidePaneTab.tabId;
  emitMockSidePaneTabsChange();
}

// Closing removes only the Tab view descriptor. Resource lifetime belongs to Grove-owned
// durable state, which this frontend-only renderer mock does not implement.
export function closeMockSidePaneTab(tabId: string) {
  const closingTabIndex = sidePaneTabs.findIndex((sidePaneTab) => sidePaneTab.tabId === tabId);
  if (closingTabIndex < 0) return;

  sidePaneTabs = sidePaneTabs.filter((sidePaneTab) => sidePaneTab.tabId !== tabId);
  if (activeSidePaneTabId === tabId) {
    activeSidePaneTabId = sidePaneTabs[Math.min(closingTabIndex, sidePaneTabs.length - 1)]?.tabId;
  }
  emitMockSidePaneTabsChange();
}
