# WinPods Desktop UI (Vite + React + Electron + TypeScript)

The desktop UI is in this folder (`apps/desktop-ui`).

- Renderer: Vite + React + TypeScript
- Shell: Electron
- Backend: Rust process (`cargo run -p desktop --bin desktop`) started automatically by Electron

## Install

```powershell
npm install
```

## Development

```powershell
npm run dev
```

This starts Vite, then Electron, then launches the Rust backend process.

## Build renderer

```powershell
npm run build
```

## Run Electron

```powershell
npm run start
```

## Packaging note

For packaged builds, place backend binary at:

`resources/backend/desktop.exe`
