import { useSyncExternalStore } from "react";
import type { GroveViewState, Project, Task, Workspace } from "@/lib/grove";

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
