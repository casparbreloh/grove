export function SidePane() {
  return (
    <aside
      aria-label="Side pane"
      className="flex size-full flex-col border-l bg-background text-foreground"
    >
      <header className="h-10 shrink-0 [-webkit-app-region:drag]" />
    </aside>
  );
}
