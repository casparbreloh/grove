const { ipcRenderer } = require("electron");

function applyFullscreenState(isFullscreen) {
  document.documentElement.dataset.nativeFullscreen = String(isFullscreen);
}

window.addEventListener("DOMContentLoaded", () => {
  if (process.platform === "darwin") document.documentElement.dataset.platform = "macos";
  ipcRenderer.invoke("window:is-fullscreen").then(applyFullscreenState);
});

ipcRenderer.on("window:fullscreen-changed", (_event, isFullscreen) => {
  applyFullscreenState(isFullscreen);
});
