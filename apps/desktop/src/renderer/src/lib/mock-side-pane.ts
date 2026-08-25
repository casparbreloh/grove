import { useSyncExternalStore } from "react";

export type NewSidePaneTab = Readonly<{
  kind: "new";
  tabId: string;
  title: "New tab";
}>;

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

export type SidePaneTab =
  | NewSidePaneTab
  | TerminalSidePaneTab
  | TestSidePaneTab
  | UnavailableSidePaneTab;
export type CreatableSidePaneTab = TerminalSidePaneTab | TestSidePaneTab;

type MockTerminalResource = Readonly<{
  terminalId: string;
}>;

type MockSidePaneState = Readonly<{
  activeTabId: string | undefined;
  tabs: readonly SidePaneTab[];
  terminalResources: readonly MockTerminalResource[];
}>;

const listeners = new Set<() => void>();
let tabs: readonly SidePaneTab[] = [];
let terminalResources: readonly MockTerminalResource[] = [];
let activeTabId: string | undefined;
let nextPlaceholder = 1;
let nextSidePaneTab = 1;
let nextTerminal = 1;
let nextTest = 1;

function createState(): MockSidePaneState {
  return { activeTabId, tabs, terminalResources };
}

let state = createState();

function emitChange() {
  state = createState();
  for (const listener of listeners) listener();
}

function createPlaceholder(): NewSidePaneTab {
  return { kind: "new", tabId: `side_tab_new_${nextPlaceholder++}`, title: "New tab" };
}

export function useMockSidePane() {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => state,
    () => state,
  );
}

export function ensureMockSidePaneTab() {
  if (tabs.length > 0) return;
  const placeholder = createPlaceholder();
  tabs = [placeholder];
  activeTabId = placeholder.tabId;
  emitChange();
}

export function selectMockSidePaneTab(tabId: string) {
  if (tabId === activeTabId || !tabs.some((tab) => tab.tabId === tabId)) return;
  activeTabId = tabId;
  emitChange();
}

export function createMockSidePaneTerminalTab(): TerminalSidePaneTab {
  const terminalNumber = nextTerminal++;
  const terminalId = `side_terminal_${terminalNumber}`;
  terminalResources = [...terminalResources, { terminalId }];

  return {
    kind: "terminal",
    tabId: `side_tab_${nextSidePaneTab++}`,
    terminalId,
    title: terminalNumber === 1 ? "Terminal" : `Terminal ${terminalNumber}`,
  };
}

export function createMockSidePaneTestTab(): TestSidePaneTab {
  const testNumber = nextTest++;
  return {
    kind: "test",
    tabId: `side_tab_${nextSidePaneTab++}`,
    title: testNumber === 1 ? "Test" : `Test ${testNumber}`,
  };
}

export function openMockSidePaneTab(tab: CreatableSidePaneTab) {
  const activeTab = tabs.find(({ tabId }) => tabId === activeTabId);
  tabs =
    activeTab?.kind === "new"
      ? tabs.map((currentTab) => (currentTab.tabId === activeTab.tabId ? tab : currentTab))
      : [...tabs, tab];
  activeTabId = tab.tabId;
  emitChange();
}

export function closeMockSidePaneTab(tabId: string) {
  const index = tabs.findIndex((tab) => tab.tabId === tabId);
  if (index < 0) return;

  if (tabs.length === 1) {
    const placeholder = createPlaceholder();
    tabs = [placeholder];
    activeTabId = placeholder.tabId;
    emitChange();
    return;
  }

  // Closing a tab removes only its view. Task resources, including terminals, keep their own
  // lifetime in the mock state.
  tabs = tabs.filter((tab) => tab.tabId !== tabId);
  if (activeTabId === tabId) activeTabId = tabs[Math.min(index, tabs.length - 1)]?.tabId;
  emitChange();
}
