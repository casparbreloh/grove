const { app, BrowserWindow, ipcMain } = require("electron/main");

const developmentUrl = "http://localhost:5173";

function createWindow() {
  const window = new BrowserWindow({
    width: 1280,
    height: 840,
    minWidth: 960,
    minHeight: 640,
    titleBarStyle: "hiddenInset",
    trafficLightPosition: { x: 12, y: 13 },
    titleBarOverlay: true,
    vibrancy: "sidebar",
    visualEffectState: "followWindow",
    webPreferences: {
      preload: `${__dirname}/preload.cjs`,
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
    },
  });

  window.webContents.setWindowOpenHandler(() => ({ action: "deny" }));
  window.on("enter-full-screen", () => {
    window.webContents.send("window:fullscreen-changed", true);
  });
  window.on("leave-full-screen", () => {
    window.webContents.send("window:fullscreen-changed", false);
  });
  window.loadURL(developmentUrl);
}

app.whenReady().then(() => {
  ipcMain.handle(
    "window:is-fullscreen",
    (event) => BrowserWindow.fromWebContents(event.sender)?.isFullScreen() ?? false,
  );
  createWindow();

  app.on("activate", () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow();
  });
});

app.on("window-all-closed", () => {
  if (process.platform !== "darwin") app.quit();
});
