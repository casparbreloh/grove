import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/_app/")({ component: App });

function App() {
  return (
    <div className="flex min-h-0 flex-1 items-center justify-center p-6">
      <h1 className="text-2xl font-semibold tracking-tight">Grove</h1>
    </div>
  );
}
