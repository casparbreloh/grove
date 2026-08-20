import type { Configuration } from "electron-builder";

const config: Configuration = {
  appId: "com.grove.app",
  productName: "Grove",
  directories: { output: "release" },
  files: ["out/main/**/*", "out/renderer/**/*"],
  extraMetadata: { version: "0.1.0" },
  mac: { category: "public.app-category.productivity" },
};

export default config;
