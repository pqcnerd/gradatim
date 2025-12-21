const path = require('node:path');
const fs = require('node:fs');
const { promises: fsPromises } = require('node:fs');
const { app, BrowserWindow, ipcMain, dialog, Menu } = require('electron');
const { createConfigStore, DEFAULT_SETTINGS } = require('./config-store');
const ptyManager = require('./pty-manager');

const isDev = process.env.NODE_ENV === 'development' || !app.isPackaged;
const devServerURL = process.env.VITE_DEV_SERVER_URL || 'http://127.0.0.1:5173';
const fallbackRustEndpoint = process.env.RUST_CORE_URL || DEFAULT_SETTINGS.rustCoreUrl;
let workspaceRoot = process.cwd();
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

  const win = createMainWindow();
  const menu = Menu.buildFromTemplate(buildMenuTemplate(win));
  Menu.setApplicationMenu(menu);

  // Set up terminal PTY handlers
  ptyManager.setupIpcHandlers(ipcMain, win);

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

  ipcMain.handle('fs:list', async () => ({
    rootPath: workspaceRoot,
    rootName: path.basename(workspaceRoot),
    snapshot: buildDirectorySnapshot(workspaceRoot)
  }));

  ipcMain.handle('fs:read-file', async (_event, relativePath) => {
    const target = resolveWorkspacePath(relativePath);
    const content = await fsPromises.readFile(target, 'utf8');
    const stat = await fsPromises.stat(target);
    const relative = normalizeRelative(target);
    return {
      path: relative ?? target,
      relativePath: relative,
      absolutePath: target,
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
    return { path: normalizeRelative(target), absolutePath: target };
  });

  ipcMain.handle('fs:write-file', async (_event, payload = {}) => {
    const target = resolveTargetPath(payload);
    if (!target) {
      throw new Error('Missing target path for write');
    }
    await fsPromises.mkdir(path.dirname(target), { recursive: true });
    await fsPromises.writeFile(target, payload.content ?? '', 'utf8');
    return {
      absolutePath: target,
      relativePath: normalizeRelative(target),
      name: path.basename(target)
    };
  });

  ipcMain.handle('fs:save-as', async (_event, payload = {}) => {
    const result = await dialog.showSaveDialog({
      title: 'Save As',
      defaultPath: payload.defaultPath || path.join(workspaceRoot, payload.suggestedName || 'untitled.c')
    });
    if (result.canceled || !result.filePath) {
      return { canceled: true };
    }
    await fsPromises.mkdir(path.dirname(result.filePath), { recursive: true });
    await fsPromises.writeFile(result.filePath, payload.content ?? '', 'utf8');
    return {
      canceled: false,
      absolutePath: result.filePath,
      relativePath: normalizeRelative(result.filePath),
      name: path.basename(result.filePath)
    };
  });

  ipcMain.handle('dialog:open-file', async () => {
    const result = await dialog.showOpenDialog({
      title: 'Open File',
      properties: ['openFile']
    });
    if (result.canceled || !result.filePaths.length) {
      return { canceled: true };
    }
    const filePath = result.filePaths[0];
    const content = await fsPromises.readFile(filePath, 'utf8');
    return {
      canceled: false,
      file: {
        name: path.basename(filePath),
        absolutePath: filePath,
        relativePath: normalizeRelative(filePath),
        content
      }
    };
  });

  ipcMain.handle('dialog:open-folder', async () => {
    const result = await dialog.showOpenDialog({
      title: 'Open Folder',
      properties: ['openDirectory']
    });
    if (result.canceled || !result.filePaths.length) {
      return { canceled: true };
    }
    workspaceRoot = result.filePaths[0];
    const snapshot = buildDirectorySnapshot(workspaceRoot);
    return {
      canceled: false,
      rootPath: workspaceRoot,
      rootName: path.basename(workspaceRoot),
      snapshot
    };
  });

  ipcMain.on('menu-command', (_event, command) => {
    sendMenuCommand(command);
  });

  ipcMain.handle('view:get-zoom', event => {
    const win = BrowserWindow.fromWebContents(event.sender);
    return win ? win.webContents.getZoomLevel() : 0;
  });

  ipcMain.handle('view:set-zoom', (event, level = 0) => {
    const win = BrowserWindow.fromWebContents(event.sender);
    if (!win || typeof level !== 'number' || Number.isNaN(level)) {
      return 0;
    }
    win.webContents.setZoomLevel(level);
    return win.webContents.getZoomLevel();
  });

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createMainWindow();
    }
  });
});

app.on('window-all-closed', () => {
  // Close all terminal sessions
  ptyManager.closeAllTerminals();

  if (process.platform !== 'darwin') {
    app.quit();
  }

  ipcMain.removeHandler('translate-line');
  ipcMain.removeHandler('settings:get');
  ipcMain.removeHandler('settings:save');
  ipcMain.removeHandler('fs:list');
  ipcMain.removeHandler('fs:read-file');
  ipcMain.removeHandler('fs:new-file');
  ipcMain.removeHandler('fs:write-file');
  ipcMain.removeHandler('fs:save-as');
  ipcMain.removeHandler('dialog:open-file');
  ipcMain.removeHandler('dialog:open-folder');
  ipcMain.removeHandler('view:get-zoom');
  ipcMain.removeHandler('view:set-zoom');
  ipcMain.removeHandler('terminal:create');
  ipcMain.removeHandler('terminal:write');
  ipcMain.removeHandler('terminal:resize');
  ipcMain.removeHandler('terminal:close');
  ipcMain.removeHandler('terminal:run-command');
  ipcMain.removeHandler('terminal:list');
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
  if (!targetPath) {
    return null;
  }
  const relative = path.relative(workspaceRoot, targetPath);
  if (!relative) {
    return '.';
  }
  if (!relative.startsWith('..') && !path.isAbsolute(relative)) {
    return relative.replace(/\\/g, '/');
  }
  return null;
}

function resolveTargetPath(payload) {
  if (payload.absolutePath) {
    return payload.absolutePath;
  }
  if (payload.relativePath) {
    return resolveWorkspacePath(payload.relativePath);
  }
  return null;
}

function sendMenuCommand(command) {
  const win = BrowserWindow.getFocusedWindow();
  if (win) {
    win.webContents.send('menu-command', command);
  }
}

function buildMenuTemplate() {
  return [
    {
      label: 'File',
      submenu: [
        {
          label: 'New File',
          accelerator: 'CmdOrCtrl+N',
          click: () => sendMenuCommand('file:new')
        },
        {
          label: 'Open File…',
          accelerator: 'CmdOrCtrl+O',
          click: () => sendMenuCommand('file:open')
        },
        {
          label: 'Open Folder…',
          accelerator: 'CmdOrCtrl+Shift+O',
          click: () => sendMenuCommand('file:openFolder')
        },
        { type: 'separator' },
        {
          label: 'Save',
          accelerator: 'CmdOrCtrl+S',
          click: () => sendMenuCommand('file:save')
        },
        {
          label: 'Save As…',
          accelerator: 'CmdOrCtrl+Shift+S',
          click: () => sendMenuCommand('file:saveAs')
        },
        { type: 'separator' },
        {
          label: 'Close Tab',
          accelerator: 'CmdOrCtrl+W',
          click: () => sendMenuCommand('file:closeTab')
        },
        {
          label: 'Close Others',
          click: () => sendMenuCommand('file:closeOthers')
        },
        {
          label: 'Close All',
          click: () => sendMenuCommand('file:closeAll')
        }
      ]
    },
    {
      label: 'Edit',
      submenu: [
        {
          label: 'Undo',
          accelerator: 'CmdOrCtrl+Z',
          click: () => sendMenuCommand('edit:undo')
        },
        {
          label: 'Redo',
          accelerator: 'CmdOrCtrl+Y',
          click: () => sendMenuCommand('edit:redo')
        },
        { type: 'separator' },
        {
          label: 'Cut',
          accelerator: 'CmdOrCtrl+X',
          click: () => sendMenuCommand('edit:cut')
        },
        {
          label: 'Copy',
          accelerator: 'CmdOrCtrl+C',
          click: () => sendMenuCommand('edit:copy')
        },
        {
          label: 'Paste',
          accelerator: 'CmdOrCtrl+V',
          click: () => sendMenuCommand('edit:paste')
        },
        {
          label: 'Select All',
          accelerator: 'CmdOrCtrl+A',
          click: () => sendMenuCommand('edit:selectAll')
        },
        { type: 'separator' },
        {
          label: 'Find',
          accelerator: 'CmdOrCtrl+F',
          click: () => sendMenuCommand('edit:find')
        },
        {
          label: 'Replace',
          accelerator: 'CmdOrCtrl+H',
          click: () => sendMenuCommand('edit:replace')
        },
        {
          label: 'Go to Line…',
          accelerator: 'CmdOrCtrl+G',
          click: () => sendMenuCommand('edit:goToLine')
        }
      ]
    },
    {
      label: 'View',
      submenu: [
        {
          label: 'Toggle Sidebar',
          accelerator: 'CmdOrCtrl+B',
          click: () => sendMenuCommand('view:toggleSidebar')
        },
        {
          label: 'Toggle Activity Panel',
          click: () => sendMenuCommand('view:toggleActivity')
        },
        {
          label: 'Toggle Terminal',
          accelerator: 'CmdOrCtrl+`',
          click: () => sendMenuCommand('view:toggleTerminal')
        },
        { type: 'separator' },
        {
          label: 'Zoom In',
          accelerator: 'CmdOrCtrl+=',
          click: () => sendMenuCommand('view:zoomIn')
        },
        {
          label: 'Zoom Out',
          accelerator: 'CmdOrCtrl+-',
          click: () => sendMenuCommand('view:zoomOut')
        },
        {
          label: 'Reset Zoom',
          accelerator: 'CmdOrCtrl+0',
          click: () => sendMenuCommand('view:zoomReset')
        },
        { type: 'separator' },
        {
          label: 'Toggle Minimap',
          click: () => sendMenuCommand('view:toggleMinimap')
        }
      ]
    },
    {
      label: 'Terminal',
      submenu: [
        {
          label: 'New Terminal',
          accelerator: 'CmdOrCtrl+Shift+`',
          click: () => sendMenuCommand('terminal:new')
        },
        {
          label: 'Run Current File',
          accelerator: 'F5',
          click: () => sendMenuCommand('terminal:runFile')
        },
        {
          label: 'Build Current File',
          accelerator: 'Ctrl+Shift+B',
          click: () => sendMenuCommand('terminal:buildFile')
        },
        { type: 'separator' },
        {
          label: 'Close Terminal',
          click: () => sendMenuCommand('terminal:close')
        }
      ]
    }
  ];
}

