import { cleanup, render, screen } from "@testing-library/react";
import { afterAll, afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { SidebarProvider } from "@/components/ui/sidebar";
import type { Project, Task } from "@/lib/mock";
import { TaskItem } from "./task-item";

const project: Project = { id: "project_grove", workspaceId: "workspace_grove", name: "grove" };
const task: Task = {
  id: "task_sidebar",
  projectId: project.id,
  title: "Refine the task sidebar",
  updatedAt: "2026-08-18T14:30:00Z",
};

afterEach(cleanup);

beforeAll(() => {
  vi.stubGlobal("matchMedia", () => ({
    addEventListener: () => undefined,
    matches: false,
    removeEventListener: () => undefined,
  }));
});

afterAll(() => vi.unstubAllGlobals());

describe("TaskItem", () => {
  it("keeps the archive button in the project metadata row", () => {
    render(
      <SidebarProvider>
        <TaskItem project={project} task={task} />
      </SidebarProvider>,
    );

    const archiveButton = screen.getByRole("button", { name: `Archive ${task.title}` });
    const metadataRow = archiveButton.parentElement;

    expect(screen.getByRole("button", { name: task.title })).toBeDefined();
    expect(metadataRow?.textContent).toContain(project.name);
    expect(metadataRow?.textContent).not.toContain(task.title);
    expect(archiveButton.getAttribute("data-slot")).toBe("button");
    expect(archiveButton.hasAttribute("data-sidebar")).toBe(false);
  });
});
