# WinPods Backend (Rust)

`crates/desktop/src-tauri` is the Rust backend service binary used by the Electron app.

## Build backend

From repository root:

```powershell
cargo build -p desktop --bin desktop
```

## Run backend directly

```powershell
cargo run -p desktop --bin desktop
```

The frontend UI lives in `apps/desktop-ui`.
