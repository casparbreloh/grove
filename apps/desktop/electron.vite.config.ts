import { defineConfig } from "electron-vite";
import renderer from "./vite.renderer.config";

export default defineConfig({
  main: {},
  preload: {
    build: {
      rollupOptions: {
        output: { entryFileNames: "[name].cjs", format: "cjs" },
      },
    },
  },
  renderer,
});
