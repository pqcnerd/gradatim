const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('electronAPI', {
  translateLine: payload => ipcRenderer.invoke('translate-line', payload),
  loadSettings: () => ipcRenderer.invoke('settings:get'),
  saveSettings: payload => ipcRenderer.invoke('settings:save', payload),
  listDirectory: () => ipcRenderer.invoke('fs:list'),
  readFile: relativePath => ipcRenderer.invoke('fs:read-file', relativePath),
  createFile: relativePath => ipcRenderer.invoke('fs:new-file', relativePath),
  writeFile: payload => ipcRenderer.invoke('fs:write-file', payload),
  saveFileAs: payload => ipcRenderer.invoke('fs:save-as', payload),
  openFileDialog: () => ipcRenderer.invoke('dialog:open-file'),
  openFolderDialog: () => ipcRenderer.invoke('dialog:open-folder'),
  getZoomLevel: () => ipcRenderer.invoke('view:get-zoom'),
  setZoomLevel: level => ipcRenderer.invoke('view:set-zoom', level),
  onMenuCommand: callback => {
    const listener = (_event, command) => callback(command);
    ipcRenderer.on('menu-command', listener);
    return () => ipcRenderer.removeListener('menu-command', listener);
  }
});

