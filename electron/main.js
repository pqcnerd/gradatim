const path = require('node:path');
const fs = require('node:fs');
const { promises: fsPromises } = require('node:fs');
const { app, BrowserWindow, ipcMain } = require('electron');
const { createConfigStore, DEFAULT_SETTINGS } = require('./config-store');

const isDev = process.env.NODE_ENV === 'development' || !app.isPackaged;
const devServerURL = process.env.VITE_DEV_SERVER_URL || 'http://127.0.0.1:5173';
const fallbackRustEndpoint = process.env.RUST_CORE_URL || DEFAULT_SETTINGS.rustCoreUrl;
const workspaceRoot = process.cwd();
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

  ipcMain.handle('fs:list', async () => buildDirectorySnapshot(workspaceRoot));

  ipcMain.handle('fs:read-file', async (_event, relativePath) => {
    const target = resolveWorkspacePath(relativePath);
    const content = await fsPromises.readFile(target, 'utf8');
    const stat = await fsPromises.stat(target);
    return {
      path: normalizeRelative(target),
      content,
      modified: stat.mtimeMs
    };
  });

  ipcMain.handle('fs:new-file', async (_event, relativePath) => {
    const target = resolveWorkspacePath(relativePath);
    await fsPromises.mkdir(path.dirname(target), { recursive: true });
    await fsPromises.writeFile(target, '', { flag: 'wx' }).catch(error => {
      if (error.code !== 'EEXIST') {
        throw error;
      }
    });
    return { path: normalizeRelative(target) };
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
  ipcMain.removeHandler('fs:list');
  ipcMain.removeHandler('fs:read-file');
  ipcMain.removeHandler('fs:new-file');
});

const IGNORED_ENTRIES = new Set(['node_modules', '.git', '.cursor', 'dist']);
const MAX_DEPTH = 4;

function buildDirectorySnapshot(dirPath, depth = 0) {
  const node = {
    type: 'folder',
    name: path.basename(dirPath) || path.basename(workspaceRoot),
    path: normalizeRelative(dirPath),
    children: []
  };

  if (depth >= MAX_DEPTH) {
    return node;
  }

  const entries = fs.readdirSync(dirPath, { withFileTypes: true });
  for (const entry of entries) {
    if (entry.name.startsWith('.')) {
      continue;
    }
    if (IGNORED_ENTRIES.has(entry.name)) {
      continue;
    }
    const fullPath = path.join(dirPath, entry.name);
    if (entry.isDirectory()) {
      node.children.push(buildDirectorySnapshot(fullPath, depth + 1));
    } else {
      node.children.push({
        type: 'file',
        name: entry.name,
        path: normalizeRelative(fullPath)
      });
    }
  }

  return node;
}

function resolveWorkspacePath(relativePath = '.') {
  const normalized = path.normalize(relativePath);
  const targetPath = path.resolve(workspaceRoot, normalized);
  if (!targetPath.startsWith(workspaceRoot)) {
    throw new Error('Path escapes workspace boundary');
  }
  return targetPath;
}

function normalizeRelative(targetPath) {
  const relative = path.relative(workspaceRoot, targetPath) || '.';
  return relative.replace(/\\/g, '/');
}

