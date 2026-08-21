import { MultiFileDiff } from "@pierre/diffs/react";
import type { FileTreeDirectoryHandle, FileTreeVisibleRow } from "@pierre/trees";
import { useFileTree, useFileTreeSelector } from "@pierre/trees/react";
import {
  ArrowDown01Icon,
  ArrowRight01Icon,
  File01Icon,
  Folder01Icon,
  ReactIcon,
  Typescript01Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useMemo } from "react";
import type { CSSProperties } from "react";
import { useTheme } from "@/components/theme-provider";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";

function createLargeChange(name: string, label: string) {
  const oldLines = Array.from(
    { length: 10_000 },
    (_, index) => `export const ${label}Line${index + 1} = "original ${index + 1}";`,
  );
  const newLines = oldLines.map((line, index) =>
    index % 20 === 0 ? `export const ${label}Line${index + 1} = "changed ${index + 1}";` : line,
  );

  return {
    oldFile: { name, contents: `${oldLines.join("\n")}\n` },
    newFile: { name, contents: `${newLines.join("\n")}\n` },
  };
}

const changes = {
  "src/components/tabs/registry.tsx": createLargeChange(
    "src/components/tabs/registry.tsx",
    "registry",
  ),
  "src/lib/mock.ts": createLargeChange("src/lib/mock.ts", "mock"),
};

const paths = Object.keys(changes);
const diffStyle = {
  "--diffs-light-bg": "var(--card)",
  "--diffs-dark-bg": "var(--card)",
  "--diffs-light": "var(--card-foreground)",
  "--diffs-dark": "var(--card-foreground)",
  "--diffs-header-font-family": "var(--font-sans)",
  "--diffs-font-size": "0.75rem",
  "--diffs-line-height": "1.25rem",
  "--diffs-gap-inline": "0.5rem",
  "--diffs-gap-block": "0.5rem",
  "--diffs-bg-context-override": "var(--muted)",
  "--diffs-bg-context-gutter-override": "var(--muted)",
  "--diffs-bg-separator-override": "var(--border)",
  "--diffs-fg-number-override": "var(--muted-foreground)",
} as CSSProperties;

function fileIcon(path: string) {
  if (path.endsWith(".tsx")) return ReactIcon;
  if (path.endsWith(".ts")) return Typescript01Icon;
  return File01Icon;
}

function rowsEqual(previous: readonly FileTreeVisibleRow[], next: readonly FileTreeVisibleRow[]) {
  return (
    previous.length === next.length &&
    previous.every((row, index) => {
      const other = next[index];
      return other?.path === row.path && other.isExpanded === row.isExpanded;
    })
  );
}

export function Diff({ diffId }: { diffId: string }) {
  const { resolvedTheme } = useTheme();
  const { model } = useFileTree({ id: diffId, initialExpansion: "open", paths });
  const rows = useFileTreeSelector(
    model,
    (tree) => tree.getVisibleRows(0, tree.getVisibleCount()),
    rowsEqual,
  );
  const diffOptions = useMemo(
    () => ({
      diffStyle: "unified" as const,
      theme: { dark: "pierre-dark", light: "pierre-light" },
      themeType: resolvedTheme === "dark" ? ("dark" as const) : ("light" as const),
    }),
    [resolvedTheme],
  );

  return (
    <div className="grid h-full min-h-0 grid-cols-[12rem_minmax(0,1fr)] gap-4 overflow-hidden bg-background p-4">
      <nav aria-label="Changed files" className="min-h-0 overflow-auto" role="tree">
        {rows.map((row) => {
          const isDirectory = row.kind === "directory";

          return (
            <Button
              aria-expanded={isDirectory ? row.isExpanded : undefined}
              className="w-full justify-start font-normal aria-expanded:bg-transparent hover:bg-muted"
              key={row.path}
              onClick={() => {
                const item = model.getItem(row.path);
                if (isDirectory && item?.isDirectory()) (item as FileTreeDirectoryHandle).toggle();
                else if (row.path in changes)
                  document
                    .getElementById(`diff-${row.path}`)
                    ?.scrollIntoView({ behavior: "smooth" });
              }}
              role="treeitem"
              style={{ paddingLeft: `${row.depth * 12 + 8}px` }}
              variant="ghost"
            >
              {isDirectory ? (
                <>
                  <HugeiconsIcon
                    className="size-3!"
                    icon={row.isExpanded ? ArrowDown01Icon : ArrowRight01Icon}
                    strokeWidth={2}
                  />
                  <HugeiconsIcon icon={Folder01Icon} strokeWidth={2} />
                </>
              ) : (
                <>
                  <span className="size-3" />
                  <HugeiconsIcon icon={fileIcon(row.path)} strokeWidth={2} />
                </>
              )}
              <span className="truncate">{row.name}</span>
            </Button>
          );
        })}
      </nav>
      <div className="min-h-0 space-y-4 overflow-auto">
        {Object.entries(changes).map(([path, change]) => (
          <Card className="gap-0 py-0" id={`diff-${path}`} key={path}>
            <MultiFileDiff
              newFile={change.newFile}
              oldFile={change.oldFile}
              options={diffOptions}
              style={diffStyle}
            />
          </Card>
        ))}
      </div>
    </div>
  );
}
