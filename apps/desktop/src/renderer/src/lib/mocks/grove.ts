import { useSyncExternalStore } from "react";

type Workspace = { id: string; name: string };
export type Project = { id: string; workspaceId: string; name: string };
export type Task = { id: string; projectId: string; title: string; updatedAt: string };

type ChatTab = {
  tabId: string;
  kind: "chat";
  sessionId: string;
  title: string;
};

type TerminalTab = {
  tabId: string;
  kind: "terminal";
  terminalId: string;
  title: string;
};

type UnavailableTab = {
  tabId: string;
  kind: "unavailable";
  originalKind: string;
  title: string;
};

export type Tab = ChatTab | TerminalTab | UnavailableTab;

export type PaneId = "pane_main" | "pane_split";
type SplitOrientation = "horizontal" | "vertical";
export type SplitEdge = "left" | "right" | "top" | "bottom";

export type TabPane = {
  paneId: PaneId;
  tabIds: readonly string[];
  activeTabId: string;
};

export type TabLayout =
  | { kind: "single"; activePaneId: PaneId; panes: readonly [TabPane] }
  | {
      kind: "split";
      activePaneId: PaneId;
      orientation: SplitOrientation;
      panes: readonly [TabPane, TabPane];
    };

export function getPaneTabs(pane: TabPane, allTabs: readonly Tab[]) {
  return pane.tabIds.flatMap((tabId) => {
    const tab = allTabs.find((candidate) => candidate.tabId === tabId);
    return tab ? [tab] : [];
  });
}

type GroveViewState = {
  workspaces: readonly Workspace[];
  activeWorkspaceId: string | undefined;
  projects: readonly Project[];
  tasks: readonly Task[];
  draftProjectId: string | undefined;
  taskProjectFilterId: string | undefined;
  tabs: readonly Tab[];
  activeTabId: string;
  tabLayout: TabLayout;
};

const workspaces: readonly Workspace[] = [
  { id: "workspace_grove", name: "Grove" },
  { id: "workspace_studio", name: "Studio" },
];

let projects: readonly Project[] = [
  { id: "project_permalux", workspaceId: "workspace_grove", name: "permalux-service" },
  { id: "project_grove", workspaceId: "workspace_grove", name: "grove" },
  {
    id: "project_consulting",
    workspaceId: "workspace_grove",
    name: "consulting-partners",
  },
  { id: "project_studio", workspaceId: "workspace_studio", name: "studio-site" },
];

const tasks: readonly Task[] = [
  {
    id: "task_sidebar",
    projectId: "project_grove",
    title: "Refine the task sidebar overflow behavior for unusually long task titles",
    updatedAt: "2026-08-18T14:30:00Z",
  },
  {
    id: "task_revisions",
    projectId: "project_grove",
    title: "Design revision checkpoints",
    updatedAt: "2026-08-17T09:15:00Z",
  },
  {
    id: "task_auth",
    projectId: "project_permalux",
    title: "Investigate authentication recovery across expired and interrupted sessions",
    updatedAt: "2026-08-16T18:20:00Z",
  },
  {
    id: "task_release",
    projectId: "project_permalux",
    title: "Prepare the next release",
    updatedAt: "2026-08-15T11:00:00Z",
  },
  {
    id: "task_side_pane",
    projectId: "project_grove",
    title: "Polish the side pane transitions",
    updatedAt: "2026-08-14T16:45:00Z",
  },
  {
    id: "task_keyboard_navigation",
    projectId: "project_grove",
    title: "Review keyboard navigation",
    updatedAt: "2026-08-13T10:20:00Z",
  },
  {
    id: "task_empty_states",
    projectId: "project_grove",
    title: "Tighten empty states",
    updatedAt: "2026-08-12T15:10:00Z",
  },
  {
    id: "task_terminal_resize",
    projectId: "project_grove",
    title: "Test terminal resizing",
    updatedAt: "2026-08-11T13:40:00Z",
  },
  {
    id: "task_task_filters",
    projectId: "project_grove",
    title: "Refine project filters",
    updatedAt: "2026-08-10T09:35:00Z",
  },
  {
    id: "task_shortcuts",
    projectId: "project_grove",
    title: "Document desktop shortcuts",
    updatedAt: "2026-08-09T17:05:00Z",
  },
  {
    id: "task_translucency",
    projectId: "project_grove",
    title: "Check translucent menus",
    updatedAt: "2026-08-08T11:55:00Z",
  },
  {
    id: "task_sidebar_width",
    projectId: "project_grove",
    title: "Verify remembered sidebar width",
    updatedAt: "2026-08-07T14:25:00Z",
  },
  {
    id: "task_panel_restore",
    projectId: "project_grove",
    title: "Exercise panel restore behavior",
    updatedAt: "2026-08-06T08:50:00Z",
  },
  {
    id: "task_focus_states",
    projectId: "project_grove",
    title: "Audit focus states",
    updatedAt: "2026-08-05T16:15:00Z",
  },
  {
    id: "task_task_archiving",
    projectId: "project_grove",
    title: "Prototype task archiving",
    updatedAt: "2026-08-04T12:30:00Z",
  },
  {
    id: "task_window_controls",
    projectId: "project_grove",
    title: "Align window controls",
    updatedAt: "2026-08-03T10:05:00Z",
  },
  {
    id: "task_scroll_fades",
    projectId: "project_grove",
    title: "Tune overflow fades",
    updatedAt: "2026-08-02T18:00:00Z",
  },
  {
    id: "task_visual_regression",
    projectId: "project_grove",
    title: "Capture visual regression cases",
    updatedAt: "2026-08-01T09:45:00Z",
  },
  {
    id: "task_landing",
    projectId: "project_studio",
    title: "Polish the landing page",
    updatedAt: "2026-08-18T08:10:00Z",
  },
];

const listeners = new Set<() => void>();
let activeWorkspaceId = workspaces[0]?.id;
let draftProjectId: string | undefined;
let taskProjectFilterId: string | undefined;
let nextProject = 1;
let nextChat = 2;
let nextTerminal = 2;
let tabs: readonly Tab[] = [
  { tabId: "tab_chat", kind: "chat", sessionId: "session_mock", title: "Chat" },
  { tabId: "tab_terminal_1", kind: "terminal", terminalId: "terminal_1", title: "Terminal" },
];
let tabLayout: TabLayout = {
  kind: "single",
  activePaneId: "pane_main",
  panes: [
    {
      paneId: "pane_main",
      tabIds: tabs.map(({ tabId }) => tabId),
      activeTabId: tabs[0].tabId,
    },
  ],
};

function getPane(paneId: PaneId) {
  return tabLayout.panes.find((pane) => pane.paneId === paneId);
}

function getPaneForTab(tabId: string) {
  return tabLayout.panes.find((pane) => pane.tabIds.includes(tabId));
}

function getActiveTabId() {
  return getPane(tabLayout.activePaneId)?.activeTabId ?? tabLayout.panes[0].activeTabId;
}

function replacePane(paneId: PaneId, nextPane: TabPane): TabLayout {
  if (tabLayout.kind === "single") {
    return {
      ...tabLayout,
      panes: [tabLayout.panes[0].paneId === paneId ? nextPane : tabLayout.panes[0]],
    };
  }

  const [firstPane, secondPane] = tabLayout.panes;
  return {
    ...tabLayout,
    panes: [
      firstPane.paneId === paneId ? nextPane : firstPane,
      secondPane.paneId === paneId ? nextPane : secondPane,
    ],
  };
}

function removeTabFromPane(pane: TabPane, tabId: string) {
  const tabIndex = pane.tabIds.indexOf(tabId);
  if (tabIndex < 0) return pane;

  const tabIds = pane.tabIds.filter((candidate) => candidate !== tabId);
  if (tabIds.length === 0) return undefined;

  return {
    ...pane,
    tabIds,
    activeTabId:
      pane.activeTabId === tabId ? tabIds[Math.min(tabIndex, tabIds.length - 1)] : pane.activeTabId,
  };
}

function createState(): GroveViewState {
  const visibleProjects: Project[] = [];
  const visibleProjectIds = new Set<string>();
  for (const project of projects) {
    if (project.workspaceId !== activeWorkspaceId) continue;
    visibleProjects.push(project);
    visibleProjectIds.add(project.id);
  }
  const visibleTasks = tasks.filter(
    ({ projectId }) =>
      visibleProjectIds.has(projectId) &&
      (taskProjectFilterId === undefined || projectId === taskProjectFilterId),
  );
  const byLastUpdated = (left: Task, right: Task) => right.updatedAt.localeCompare(left.updatedAt);

  return {
    workspaces,
    activeWorkspaceId,
    projects: visibleProjects,
    tasks: visibleTasks.sort(byLastUpdated),
    draftProjectId,
    taskProjectFilterId,
    tabs,
    activeTabId: getActiveTabId(),
    tabLayout,
  };
}

let state = createState();

function emitChange() {
  state = createState();
  for (const listener of listeners) listener();
}

export function useMockGrove() {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => state,
    () => state,
  );
}

function addMockTab(tab: Tab, paneId: PaneId = tabLayout.activePaneId) {
  const targetPane = getPane(paneId) ?? tabLayout.panes[0];
  tabs = [...tabs, tab];
  tabLayout = {
    ...replacePane(targetPane.paneId, {
      ...targetPane,
      tabIds: [...targetPane.tabIds, tab.tabId],
      activeTabId: tab.tabId,
    }),
    activePaneId: targetPane.paneId,
  };
  emitChange();
}

export function selectMockTab(tabId: string) {
  const pane = getPaneForTab(tabId);
  if (!pane || (pane.activeTabId === tabId && tabLayout.activePaneId === pane.paneId)) return;
  tabLayout = {
    ...replacePane(pane.paneId, { ...pane, activeTabId: tabId }),
    activePaneId: pane.paneId,
  };
  emitChange();
}

export function createMockChatTab(paneId?: PaneId) {
  const chatNumber = nextChat++;
  const tab: Tab = {
    tabId: `tab_chat_${chatNumber}`,
    kind: "chat",
    sessionId: `session_${chatNumber}`,
    title: `Chat ${chatNumber}`,
  };
  addMockTab(tab, paneId);
}

export function createMockTerminalTab(paneId?: PaneId) {
  const terminalNumber = nextTerminal++;
  const tab: Tab = {
    tabId: `tab_terminal_${terminalNumber}`,
    kind: "terminal",
    terminalId: `terminal_${terminalNumber}`,
    title: `Terminal ${terminalNumber}`,
  };
  addMockTab(tab, paneId);
}

export function closeMockTab(tabId: string) {
  const tabIndex = tabs.findIndex((tab) => tab.tabId === tabId);
  if (tabIndex < 0 || tabs.length === 1) return;

  const sourcePane = getPaneForTab(tabId);
  if (!sourcePane) return;
  const nextSourcePane = removeTabFromPane(sourcePane, tabId);

  tabs = tabs.filter((tab) => tab.tabId !== tabId);
  if (!nextSourcePane) {
    if (tabLayout.kind !== "split") return;
    const remainingPane = tabLayout.panes.find((pane) => pane.paneId !== sourcePane.paneId);
    if (!remainingPane) return;
    tabLayout = {
      kind: "single",
      activePaneId: remainingPane.paneId,
      panes: [remainingPane],
    };
  } else {
    tabLayout = replacePane(sourcePane.paneId, nextSourcePane);
  }
  emitChange();
  return getActiveTabId();
}

export function moveMockTab(tabId: string, targetPaneId: PaneId, overTabId?: string) {
  const sourcePane = getPaneForTab(tabId);
  const targetPane = getPane(targetPaneId);
  if (!sourcePane || !targetPane) return;

  if (sourcePane.paneId === targetPane.paneId) {
    const sourceIndex = sourcePane.tabIds.indexOf(tabId);
    const targetIndex = overTabId
      ? sourcePane.tabIds.indexOf(overTabId)
      : sourcePane.tabIds.length - 1;
    if (sourceIndex < 0 || targetIndex < 0 || sourceIndex === targetIndex) return;

    const nextTabIds = [...sourcePane.tabIds];
    nextTabIds.splice(sourceIndex, 1);
    nextTabIds.splice(targetIndex, 0, tabId);
    tabLayout = replacePane(sourcePane.paneId, { ...sourcePane, tabIds: nextTabIds });
    emitChange();
    return;
  }

  if (tabLayout.kind !== "split") return;

  const nextSourcePane = removeTabFromPane(sourcePane, tabId);
  const targetIndex = overTabId ? targetPane.tabIds.indexOf(overTabId) : targetPane.tabIds.length;
  if (targetIndex < 0) return;
  const targetTabIds = [...targetPane.tabIds];
  targetTabIds.splice(targetIndex, 0, tabId);
  const nextTargetPane = { ...targetPane, tabIds: targetTabIds, activeTabId: tabId };

  if (!nextSourcePane) {
    tabLayout = {
      kind: "single",
      activePaneId: targetPane.paneId,
      panes: [nextTargetPane],
    };
  } else {
    const [firstPane] = tabLayout.panes;
    tabLayout = {
      ...tabLayout,
      activePaneId: targetPane.paneId,
      panes:
        firstPane.paneId === sourcePane.paneId
          ? [nextSourcePane, nextTargetPane]
          : [nextTargetPane, nextSourcePane],
    };
  }
  emitChange();
}

export function splitMockTab(tabId: string, sourcePaneId: PaneId, edge: SplitEdge) {
  const sourcePane = getPane(sourcePaneId);
  if (tabLayout.kind !== "single" || !sourcePane || sourcePane.tabIds.length < 2) return;
  if (!sourcePane.tabIds.includes(tabId)) return;

  const nextSourcePane = removeTabFromPane(sourcePane, tabId);
  if (!nextSourcePane) return;
  const splitPaneId: PaneId = sourcePane.paneId === "pane_main" ? "pane_split" : "pane_main";
  const splitPane: TabPane = { paneId: splitPaneId, tabIds: [tabId], activeTabId: tabId };

  tabLayout = {
    kind: "split",
    activePaneId: splitPaneId,
    orientation: edge === "left" || edge === "right" ? "horizontal" : "vertical",
    panes:
      edge === "left" || edge === "top" ? [splitPane, nextSourcePane] : [nextSourcePane, splitPane],
  };
  emitChange();
}

export function selectMockDraftProject(projectId: string | undefined) {
  if (
    projectId &&
    !projects.some(({ id, workspaceId }) => id === projectId && workspaceId === activeWorkspaceId)
  )
    return;
  draftProjectId = projectId;
  emitChange();
}

export function createMockProject(name: string) {
  const normalizedName = name.trim();
  if (!normalizedName || !activeWorkspaceId) return;

  const project = {
    id: `mock_project_${nextProject++}`,
    workspaceId: activeWorkspaceId,
    name: normalizedName,
  };
  projects = [...projects, project];
  draftProjectId = project.id;
  emitChange();
}

export function filterMockTasksByProject(projectId: string | undefined) {
  if (
    projectId &&
    !projects.some(({ id, workspaceId }) => id === projectId && workspaceId === activeWorkspaceId)
  )
    return;
  taskProjectFilterId = projectId;
  emitChange();
}
