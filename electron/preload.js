const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('electronAPI', {
  translateLine: payload => ipcRenderer.invoke('translate-line', payload),
  loadSettings: () => ipcRenderer.invoke('settings:get'),
  saveSettings: payload => ipcRenderer.invoke('settings:save', payload),
  listDirectory: () => ipcRenderer.invoke('fs:list'),
  readFile: relativePath => ipcRenderer.invoke('fs:read-file', relativePath),
  createFile: relativePath => ipcRenderer.invoke('fs:new-file', relativePath)
});

