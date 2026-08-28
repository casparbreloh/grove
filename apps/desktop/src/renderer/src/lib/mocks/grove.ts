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

type GroveViewState = {
  workspaces: readonly Workspace[];
  activeWorkspaceId: string | undefined;
  projects: readonly Project[];
  tasks: readonly Task[];
  draftProjectId: string | undefined;
  taskProjectFilterId: string | undefined;
  tabs: readonly Tab[];
  activeTabId: string;
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
let activeTabId = tabs[0].tabId;

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
    activeTabId,
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

function addMockTab(tab: Tab) {
  tabs = [...tabs, tab];
  activeTabId = tab.tabId;
  emitChange();
}

export function selectMockTab(tabId: string) {
  if (tabId === activeTabId || !tabs.some((tab) => tab.tabId === tabId)) return;
  activeTabId = tabId;
  emitChange();
}

export function createMockChatTab() {
  const chatNumber = nextChat++;
  const tab: Tab = {
    tabId: `tab_chat_${chatNumber}`,
    kind: "chat",
    sessionId: `session_${chatNumber}`,
    title: `Chat ${chatNumber}`,
  };
  addMockTab(tab);
}

export function createMockTerminalTab() {
  const terminalNumber = nextTerminal++;
  const tab: Tab = {
    tabId: `tab_terminal_${terminalNumber}`,
    kind: "terminal",
    terminalId: `terminal_${terminalNumber}`,
    title: `Terminal ${terminalNumber}`,
  };
  addMockTab(tab);
}

export function closeMockTab(tabId: string) {
  const tabIndex = tabs.findIndex((tab) => tab.tabId === tabId);
  if (tabIndex < 0 || tabs.length === 1) return;

  tabs = tabs.filter((tab) => tab.tabId !== tabId);
  if (activeTabId === tabId) activeTabId = tabs[Math.min(tabIndex, tabs.length - 1)].tabId;
  emitChange();
  return activeTabId;
}

export function reorderMockTab(tabId: string, overTabId?: string) {
  const sourceIndex = tabs.findIndex((tab) => tab.tabId === tabId);
  const targetIndex = overTabId
    ? tabs.findIndex((tab) => tab.tabId === overTabId)
    : tabs.length - 1;
  if (sourceIndex < 0 || targetIndex < 0 || sourceIndex === targetIndex) return;

  const nextTabs = [...tabs];
  const [movedTab] = nextTabs.splice(sourceIndex, 1);
  nextTabs.splice(targetIndex, 0, movedTab);
  tabs = nextTabs;
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
