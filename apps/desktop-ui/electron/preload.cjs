const { contextBridge, ipcRenderer } = require("electron");

contextBridge.exposeInMainWorld("backend", {
  status: () => ipcRenderer.invoke("backend:status"),
  restart: () => ipcRenderer.invoke("backend:restart"),
  data: () => ipcRenderer.invoke("backend:data")
});
