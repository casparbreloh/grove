export type Workspace = { id: string; name: string };
export type Project = { id: string; workspaceId: string; name: string };
export type Task = { id: string; projectId: string; title: string; updatedAt: string };

export type GroveViewState = {
  workspaces: readonly Workspace[];
  activeWorkspaceId: string | undefined;
  projects: readonly Project[];
  tasks: readonly Task[];
  draftProjectId: string | undefined;
  taskProjectFilterId: string | undefined;
};
