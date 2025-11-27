const path = require('node:path');
const { app, BrowserWindow, ipcMain } = require('electron');
const { createConfigStore, DEFAULT_SETTINGS } = require('./config-store');

const isDev = process.env.NODE_ENV === 'development' || !app.isPackaged;
const devServerURL = process.env.VITE_DEV_SERVER_URL || 'http://127.0.0.1:5173';
const fallbackRustEndpoint = process.env.RUST_CORE_URL || DEFAULT_SETTINGS.rustCoreUrl;
let configStore;
let currentSettings = { ...DEFAULT_SETTINGS };
let rustEndpoint = fallbackRustEndpoint;

function createMainWindow() {
  const win = new BrowserWindow({
    width: 1200,
    height: 800,
    minWidth: 960,
    minHeight: 600,
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false,
      spellcheck: false
    },
    title: 'Gradatim — NL to C editor'
  });

  if (isDev) {
    win.loadURL(devServerURL);
    win.webContents.openDevTools({ mode: 'detach' });
  } else {
    const indexHtml = path.join(__dirname, '..', 'renderer', 'dist', 'index.html');
    win.loadFile(indexHtml);
  }

  return win;
}

app.whenReady().then(() => {
  configStore = createConfigStore(app);
  currentSettings = configStore.get();
  rustEndpoint = currentSettings.rustCoreUrl || fallbackRustEndpoint;

  createMainWindow();

  ipcMain.handle('translate-line', async (_event, payload) => {
    const body = {
      ...payload,
      api_key: payload.api_key || currentSettings.ai?.apiKey || undefined,
      model: payload.model || currentSettings.ai?.model || undefined,
    };

    try {
      const response = await fetch(`${rustEndpoint}/translate-line`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });

      if (!response.ok) {
        const text = await response.text();
        return { kind: 'error', message: `Rust core error: ${text}` };
      }

      return response.json();
    } catch (error) {
      return { kind: 'error', message: `Failed to reach rust-core: ${error.message}` };
    }
  });

  ipcMain.handle('settings:get', () => currentSettings);
  ipcMain.handle('settings:save', (_event, payload = {}) => {
    currentSettings = configStore.set(payload);
    rustEndpoint = currentSettings.rustCoreUrl || fallbackRustEndpoint;
    return currentSettings;
  });

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createMainWindow();
    }
  });
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit();
  }

  ipcMain.removeHandler('translate-line');
  ipcMain.removeHandler('settings:get');
  ipcMain.removeHandler('settings:save');
});

