import { Outlet, createRootRoute } from "@tanstack/react-router";
import { ThemeProvider } from "../components/theme-provider";

export const Route = createRootRoute({
  component: Root,
  notFoundComponent: () => (
    <main className="container mx-auto p-4 pt-16">
      <h1>404</h1>
      <p>The requested page could not be found.</p>
    </main>
  ),
});

function Root() {
  return (
    <ThemeProvider attribute="class" defaultTheme="system" storageKey="theme">
      <Outlet />
    </ThemeProvider>
  );
}
