import { createFileRoute } from "@tanstack/react-router";
import { Chat } from "@/components/chat";

export const Route = createFileRoute("/_app/")({ component: App });

function App() {
  return (
    <div className="min-h-0 flex-1">
      <Chat />
    </div>
  );
}
