/**
 * PTY Manager - Handles terminal shell processes
 * 
 * This module manages pseudo-terminal instances using node-pty,
 * allowing the renderer to spawn and interact with shell processes.
 */

const os = require('os');
const path = require('path');

// node-pty will be required dynamically to handle the case where it's not installed
let pty = null;
try {
  pty = require('node-pty');
} catch (e) {
  console.warn('node-pty not available. Terminal functionality will be disabled.');
}

/**
 * Map of terminal ID to PTY instance
 * @type {Map<string, import('node-pty').IPty>}
 */
const terminals = new Map();

/**
 * Counter for generating unique terminal IDs
 */
let terminalIdCounter = 0;

/**
 * Get the default shell for the current platform
 * @returns {string} Path to the default shell
 */
function getDefaultShell() {
  if (process.platform === 'win32') {
    return process.env.COMSPEC || 'cmd.exe';
  }
  return process.env.SHELL || '/bin/bash';
}

/**
 * Get the default shell arguments
 * @returns {string[]} Shell arguments
 */
function getDefaultShellArgs() {
  if (process.platform === 'win32') {
    return [];
  }
  // Use login shell on Unix
  return ['-l'];
}

/**
 * Create a new terminal instance
 * @param {object} options Terminal options
 * @param {string} [options.shell] Shell to use
 * @param {string[]} [options.args] Shell arguments
 * @param {string} [options.cwd] Working directory
 * @param {number} [options.cols] Number of columns
 * @param {number} [options.rows] Number of rows
 * @param {object} [options.env] Environment variables
 * @returns {{ id: string, pid: number } | null} Terminal info or null if failed
 */
function createTerminal(options = {}) {
  if (!pty) {
    console.error('node-pty is not available');
    return null;
  }

  const id = `term-${++terminalIdCounter}`;
  const shell = options.shell || getDefaultShell();
  const args = options.args || getDefaultShellArgs();
  const cwd = options.cwd || process.env.HOME || os.homedir();
  const cols = options.cols || 80;
  const rows = options.rows || 24;

  try {
    const term = pty.spawn(shell, args, {
      name: 'xterm-256color',
      cols,
      rows,
      cwd,
      env: { ...process.env, ...options.env },
    });

    terminals.set(id, term);

    console.log(`Terminal ${id} created (PID: ${term.pid})`);

    return {
      id,
      pid: term.pid,
    };
  } catch (error) {
    console.error('Failed to create terminal:', error);
    return null;
  }
}

/**
 * Get a terminal instance by ID
 * @param {string} id Terminal ID
 * @returns {import('node-pty').IPty | undefined}
 */
function getTerminal(id) {
  return terminals.get(id);
}

/**
 * Write data to a terminal
 * @param {string} id Terminal ID
 * @param {string} data Data to write
 * @returns {boolean} Success status
 */
function writeToTerminal(id, data) {
  const term = terminals.get(id);
  if (!term) {
    console.warn(`Terminal ${id} not found`);
    return false;
  }
  term.write(data);
  return true;
}

/**
 * Resize a terminal
 * @param {string} id Terminal ID
 * @param {number} cols Number of columns
 * @param {number} rows Number of rows
 * @returns {boolean} Success status
 */
function resizeTerminal(id, cols, rows) {
  const term = terminals.get(id);
  if (!term) {
    console.warn(`Terminal ${id} not found`);
    return false;
  }
  try {
    term.resize(cols, rows);
    return true;
  } catch (error) {
    console.error(`Failed to resize terminal ${id}:`, error);
    return false;
  }
}

/**
 * Close a terminal
 * @param {string} id Terminal ID
 * @returns {boolean} Success status
 */
function closeTerminal(id) {
  const term = terminals.get(id);
  if (!term) {
    console.warn(`Terminal ${id} not found`);
    return false;
  }
  try {
    term.kill();
    terminals.delete(id);
    console.log(`Terminal ${id} closed`);
    return true;
  } catch (error) {
    console.error(`Failed to close terminal ${id}:`, error);
    return false;
  }
}

/**
 * Close all terminals
 */
function closeAllTerminals() {
  for (const [id, term] of terminals) {
    try {
      term.kill();
      console.log(`Terminal ${id} closed`);
    } catch (error) {
      console.error(`Failed to close terminal ${id}:`, error);
    }
  }
  terminals.clear();
}

/**
 * Set up IPC handlers for terminal operations
 * @param {import('electron').IpcMain} ipcMain Electron IPC main
 * @param {import('electron').BrowserWindow} mainWindow Main browser window
 */
function setupIpcHandlers(ipcMain, mainWindow) {
  // Create a new terminal
  ipcMain.handle('terminal:create', async (event, options) => {
    const result = createTerminal(options);
    if (!result) {
      return { success: false, error: 'Failed to create terminal' };
    }

    const term = getTerminal(result.id);
    if (term) {
      // Forward terminal output to renderer
      term.onData((data) => {
        if (mainWindow && !mainWindow.isDestroyed()) {
          mainWindow.webContents.send('terminal:data', {
            id: result.id,
            data,
          });
        }
      });

      // Handle terminal exit
      term.onExit(({ exitCode, signal }) => {
        if (mainWindow && !mainWindow.isDestroyed()) {
          mainWindow.webContents.send('terminal:exit', {
            id: result.id,
            exitCode,
            signal,
          });
        }
        terminals.delete(result.id);
      });
    }

    return { success: true, ...result };
  });

  // Write to terminal
  ipcMain.handle('terminal:write', async (event, { id, data }) => {
    return writeToTerminal(id, data);
  });

  // Resize terminal
  ipcMain.handle('terminal:resize', async (event, { id, cols, rows }) => {
    return resizeTerminal(id, cols, rows);
  });

  // Close terminal
  ipcMain.handle('terminal:close', async (event, { id }) => {
    return closeTerminal(id);
  });

  // Run a command in a new terminal
  ipcMain.handle('terminal:run-command', async (event, { command, cwd }) => {
    const result = createTerminal({ cwd });
    if (!result) {
      return { success: false, error: 'Failed to create terminal' };
    }

    const term = getTerminal(result.id);
    if (term) {
      // Forward terminal output to renderer
      term.onData((data) => {
        if (mainWindow && !mainWindow.isDestroyed()) {
          mainWindow.webContents.send('terminal:data', {
            id: result.id,
            data,
          });
        }
      });

      // Handle terminal exit
      term.onExit(({ exitCode, signal }) => {
        if (mainWindow && !mainWindow.isDestroyed()) {
          mainWindow.webContents.send('terminal:exit', {
            id: result.id,
            exitCode,
            signal,
          });
        }
        terminals.delete(result.id);
      });

      // Write the command
      term.write(command + '\r');
    }

    return { success: true, ...result };
  });

  // Get list of active terminals
  ipcMain.handle('terminal:list', async () => {
    return Array.from(terminals.keys());
  });
}

/**
 * Check if node-pty is available
 * @returns {boolean}
 */
function isAvailable() {
  return pty !== null;
}

module.exports = {
  createTerminal,
  getTerminal,
  writeToTerminal,
  resizeTerminal,
  closeTerminal,
  closeAllTerminals,
  setupIpcHandlers,
  isAvailable,
};

