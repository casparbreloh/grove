import path from "node:path";
import { defineConfig } from "vite";
import { devtools } from "@tanstack/devtools-vite";
import { tanstackRouter } from "@tanstack/router-plugin/vite";
import viteReact from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  root: path.resolve(import.meta.dirname, "src/renderer"),
  resolve: { tsconfigPaths: true },
  server: { port: 5173, strictPort: true },
  plugins: [tanstackRouter({ autoCodeSplitting: true }), devtools(), tailwindcss(), viteReact()],
});
