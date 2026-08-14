import { contextBridge } from "electron";

contextBridge.exposeInMainWorld("grove", {
  platform: process.platform,
});
