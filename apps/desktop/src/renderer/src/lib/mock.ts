import { useSyncExternalStore } from "react";

export type Workspace = { id: string; name: string };
export type Project = { id: string; workspaceId: string; name: string };
export type Task = { id: string; projectId: string; title: string; updatedAt: string };

export type AgentTab = {
  tabId: string;
  kind: "agent";
  sessionId: string;
  title: string;
};

export type TerminalTab = {
  tabId: string;
  kind: "terminal";
  terminalId: string;
  title: string;
};

export type DiffTab = {
  tabId: string;
  kind: "diff";
  diffId: string;
  title: string;
};

export type GroveTab = AgentTab | TerminalTab | DiffTab;

export type GroveViewState = {
  workspaces: readonly Workspace[];
  activeWorkspaceId: string | undefined;
  projects: readonly Project[];
  tasks: readonly Task[];
  draftProjectId: string | undefined;
  taskProjectFilterId: string | undefined;
  tabs: readonly GroveTab[];
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
    title: "Refine the task sidebar",
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
    title: "Investigate authentication flow",
    updatedAt: "2026-08-16T18:20:00Z",
  },
  {
    id: "task_release",
    projectId: "project_permalux",
    title: "Prepare the next release",
    updatedAt: "2026-08-15T11:00:00Z",
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
let nextAgent = 2;
let nextTerminal = 2;
let nextDiff = 1;
let tabs: readonly GroveTab[] = [
  { tabId: "tab_agent", kind: "agent", sessionId: "session_mock", title: "Agent" },
  { tabId: "tab_terminal_1", kind: "terminal", terminalId: "terminal_1", title: "Terminal" },
];
let activeTabId = tabs[0].tabId;

function createState(): GroveViewState {
  const visibleProjects = projects.filter(({ workspaceId }) => workspaceId === activeWorkspaceId);
  const projectIds = new Set(visibleProjects.map(({ id }) => id));
  const visibleTasks = tasks.filter(
    ({ projectId }) =>
      projectIds.has(projectId) &&
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

export function selectMockTab(tabId: string) {
  if (tabId === activeTabId || !tabs.some((tab) => tab.tabId === tabId)) return;
  activeTabId = tabId;
  emitChange();
}

export function createMockAgentTab() {
  const agentNumber = nextAgent++;
  const tab: GroveTab = {
    tabId: `tab_agent_${agentNumber}`,
    kind: "agent",
    sessionId: `session_${agentNumber}`,
    title: `Agent ${agentNumber}`,
  };
  tabs = [...tabs, tab];
  activeTabId = tab.tabId;
  emitChange();
}

export function createMockTerminalTab() {
  const terminalNumber = nextTerminal++;
  const tab: GroveTab = {
    tabId: `tab_terminal_${terminalNumber}`,
    kind: "terminal",
    terminalId: `terminal_${terminalNumber}`,
    title: terminalNumber === 1 ? "Terminal" : `Terminal ${terminalNumber}`,
  };
  tabs = [...tabs, tab];
  activeTabId = tab.tabId;
  emitChange();
}

export function createMockDiffTab() {
  const diffNumber = nextDiff++;
  const tab: GroveTab = {
    tabId: `tab_diff_${diffNumber}`,
    kind: "diff",
    diffId: `diff_${diffNumber}`,
    title: diffNumber === 1 ? "Diff" : `Diff ${diffNumber}`,
  };
  tabs = [...tabs, tab];
  activeTabId = tab.tabId;
  emitChange();
}

export function closeMockTab(tabId: string) {
  const index = tabs.findIndex((tab) => tab.tabId === tabId);
  if (index < 0 || tabs.length === 1) return;

  tabs = tabs.filter((tab) => tab.tabId !== tabId);
  if (activeTabId === tabId) activeTabId = tabs[Math.min(index, tabs.length - 1)].tabId;
  emitChange();
}

export function selectMockWorkspace(workspaceId: string) {
  if (workspaceId === activeWorkspaceId) return;
  if (!workspaces.some(({ id }) => id === workspaceId)) return;
  activeWorkspaceId = workspaceId;
  draftProjectId = undefined;
  taskProjectFilterId = undefined;
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

import type { ChatModelAdapter } from "@assistant-ui/react";

function waitForChunk(abortSignal: AbortSignal) {
  if (abortSignal.aborted) return Promise.resolve(false);

  return new Promise<boolean>((resolve) => {
    let settled = false;
    let timeout: ReturnType<typeof setTimeout>;
    const finish = (shouldContinue: boolean) => {
      if (settled) return;
      settled = true;
      clearTimeout(timeout);
      abortSignal.removeEventListener("abort", onAbort);
      resolve(shouldContinue);
    };
    const onAbort = () => finish(false);

    timeout = setTimeout(() => finish(true), 24);
    abortSignal.addEventListener("abort", onAbort, { once: true });
    if (abortSignal.aborted) onAbort();
  });
}

export const mockChatModel: ChatModelAdapter = {
  async *run({ messages, abortSignal }) {
    const userMessage = messages.findLast((message) => message.role === "user");
    const prompt = userMessage?.content
      .filter((part) => part.type === "text")
      .map((part) => part.text)
      .join("\n\n")
      .trim();
    if (!prompt) throw new Error("A message is required");

    let text = "";
    for (const character of `You said: ${prompt}`) {
      if (!(await waitForChunk(abortSignal))) return;
      text += character;
      yield { content: [{ type: "text", text }] };
    }
  },
};
