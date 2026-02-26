const { app, BrowserWindow, Menu, Tray, ipcMain, nativeImage } = require("electron");
const fs = require("fs");
const path = require("path");
const { spawn } = require("child_process");

const isDev = !app.isPackaged;
const forceLocalBundle = process.env.WINPODS_FORCE_LOCAL_BUNDLE === "1";
let backendProcess = null;
let mainWindow = null;
let tray = null;
let isQuitting = false;

function resolveBackendCommand() {
  if (forceLocalBundle) {
    return {
      command: path.resolve(__dirname, "../.runtime/backend/desktop.exe"),
      args: [],
      cwd: undefined
    };
  }

  if (isDev) {
    return {
      command: "cargo",
      args: ["run", "-p", "desktop", "--bin", "desktop"],
      cwd: path.resolve(__dirname, "../../..")
    };
  }

  return {
    command: path.join(process.resourcesPath, "backend", "desktop.exe"),
    args: [],
    cwd: undefined
  };
}

function startBackend() {
  if (backendProcess) return;

  const { command, args, cwd } = resolveBackendCommand();
  backendProcess = spawn(command, args, {
    cwd,
    stdio: "ignore",
    windowsHide: true,
    shell: false
  });

  backendProcess.on("exit", (code) => {
    backendProcess = null;
    console.log(`[backend] exited with code ${code}`);
  });
}

function stopBackend() {
  if (!backendProcess) return;
  try {
    backendProcess.kill();
  } catch (_) {}
  backendProcess = null;
}

function resolveStatePath() {
  return path.join(app.getPath("appData"), "winpods", "state.json");
}

function createWindow() {
  mainWindow = new BrowserWindow({
    width: 1180,
    height: 760,
    minWidth: 1080,
    minHeight: 760,
    autoHideMenuBar: true,
    backgroundColor: "#111111",
    webPreferences: {
      preload: path.join(__dirname, "preload.cjs"),
      contextIsolation: true,
      nodeIntegration: false
    }
  });

  if (isDev && !forceLocalBundle) {
    mainWindow.loadURL("http://127.0.0.1:5173");
  } else {
    mainWindow.loadFile(path.join(__dirname, "../dist/index.html"));
  }

  mainWindow.on("close", (event) => {
    if (isQuitting) return;
    event.preventDefault();
    mainWindow.hide();
  });
}

function resolveTrayIconPath() {
  if (isDev) {
    return path.resolve(__dirname, "../../../crates/desktop/src-tauri/icons/icon.png");
  }

  return path.join(process.resourcesPath, "icons", "icon.png");
}

function createTray() {
  const iconPath = resolveTrayIconPath();
  const icon = nativeImage.createFromPath(iconPath);
  tray = new Tray(icon.isEmpty() ? nativeImage.createEmpty() : icon);
  tray.setToolTip("WinPods");

  const trayMenu = Menu.buildFromTemplate([
    {
      label: "Open WinPods",
      click: () => {
        if (!mainWindow) {
          createWindow();
          return;
        }

        mainWindow.show();
        mainWindow.focus();
      }
    },
    {
      label: "Quit",
      click: () => {
        isQuitting = true;
        app.quit();
      }
    }
  ]);

  tray.setContextMenu(trayMenu);
  tray.on("double-click", () => {
    if (!mainWindow) {
      createWindow();
      return;
    }
    mainWindow.show();
    mainWindow.focus();
  });
}

app.whenReady().then(() => {
  startBackend();
  createTray();
  createWindow();

  app.on("activate", () => {
    if (!mainWindow) {
      createWindow();
      return;
    }

    mainWindow.show();
    mainWindow.focus();
  });
});

ipcMain.handle("backend:status", () => ({ running: Boolean(backendProcess) }));
ipcMain.handle("backend:restart", () => {
  stopBackend();
  startBackend();
  return { running: Boolean(backendProcess) };
});
ipcMain.handle("backend:data", () => {
  const statePath = resolveStatePath();
  if (!fs.existsSync(statePath)) return null;
  try {
    const raw = fs.readFileSync(statePath, "utf-8");
    return JSON.parse(raw);
  } catch (_) {
    return null;
  }
});

app.on("before-quit", () => {
  isQuitting = true;
  stopBackend();
});

app.on("window-all-closed", () => {});
