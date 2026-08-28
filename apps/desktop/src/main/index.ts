import {
  app,
  BrowserWindow,
  Menu,
  nativeTheme,
  shell,
  type MenuItemConstructorOptions,
} from "electron";
import path from "node:path";
import { pathToFileURL } from "node:url";

const isMac = process.platform === "darwin";

let mainWindow: BrowserWindow | undefined;

function opaqueWindowBackground() {
  return nativeTheme.shouldUseDarkColors ? "#121212" : "#ffffff";
}

function rendererUrl() {
  return (
    process.env.ELECTRON_RENDERER_URL ??
    pathToFileURL(path.join(__dirname, "../renderer/index.html")).toString()
  );
}

function isRendererUrl(url: string) {
  const expectedUrl = rendererUrl();
  if (!process.env.ELECTRON_RENDERER_URL) return url === expectedUrl;
  return new URL(url).origin === new URL(expectedUrl).origin;
}

function openExternal(url: string) {
  const protocol = new URL(url).protocol;
  if (protocol === "https:" || protocol === "http:" || protocol === "mailto:")
    void shell.openExternal(url);
}

function configureApplicationMenu() {
  const template: MenuItemConstructorOptions[] = [
    ...(isMac ? ([{ role: "appMenu" }] satisfies MenuItemConstructorOptions[]) : []),
    { role: "fileMenu" },
    { role: "editMenu" },
    {
      label: "View",
      submenu: [
        { role: "toggleDevTools" },
        { type: "separator" },
        { role: "resetZoom" },
        { role: "zoomIn" },
        { role: "zoomOut" },
        { type: "separator" },
        { role: "togglefullscreen" },
      ],
    },
    { role: "windowMenu" },
  ];

  Menu.setApplicationMenu(Menu.buildFromTemplate(template));
}

function createWindow() {
  const useSidebarVibrancy = isMac && !nativeTheme.prefersReducedTransparency;
  const window = new BrowserWindow({
    width: 1280,
    height: 840,
    minWidth: 960,
    minHeight: 640,
    show: false,
    ...(!useSidebarVibrancy ? { backgroundColor: opaqueWindowBackground() } : {}),
    titleBarStyle: isMac ? "hiddenInset" : "hidden",
    titleBarOverlay: true,
    ...(isMac
      ? {
          trafficLightPosition: { x: 12, y: 13 },
          ...(useSidebarVibrancy
            ? {
                vibrancy: "sidebar" as const,
                visualEffectState: "followWindow" as const,
              }
            : {}),
        }
      : {}),
    webPreferences: {
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
    },
  });

  window.once("ready-to-show", () => window.show());
  window.webContents.setWindowOpenHandler(({ url }) => {
    openExternal(url);
    return { action: "deny" };
  });
  window.webContents.on("will-navigate", (event, url) => {
    if (!isRendererUrl(url)) {
      event.preventDefault();
      openExternal(url);
    }
  });

  const updateWindowMaterial = () => {
    if (window.isDestroyed()) return;

    if (!isMac) {
      window.setBackgroundColor(opaqueWindowBackground());
      return;
    }

    if (nativeTheme.prefersReducedTransparency) {
      window.setVibrancy(null);
      window.setBackgroundColor(opaqueWindowBackground());
      return;
    }

    window.setVibrancy("sidebar");
    window.setBackgroundColor("#00000000");
  };

  nativeTheme.on("updated", updateWindowMaterial);

  void window.loadURL(rendererUrl());

  mainWindow = window;
  window.on("closed", () => {
    nativeTheme.off("updated", updateWindowMaterial);
    mainWindow = undefined;
  });
}

if (!app.requestSingleInstanceLock()) {
  app.quit();
} else {
  app.on("second-instance", () => {
    if (!mainWindow) return;
    if (mainWindow.isMinimized()) mainWindow.restore();
    mainWindow.focus();
  });

  app.whenReady().then(() => {
    configureApplicationMenu();
    createWindow();
    app.on("activate", () => {
      if (BrowserWindow.getAllWindows().length === 0) createWindow();
    });
  });
}

app.on("window-all-closed", () => {
  if (process.platform !== "darwin") app.quit();
});
