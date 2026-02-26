# winpods <img src=".github/images/icon.png" alt="winpods icon" width="30"/>

winpods now uses:

- Electron + Vite/React/TypeScript UI in `apps/desktop-ui`
- Rust backend service in `crates/desktop/src-tauri`

## Run desktop UI (dev)

```powershell
cd apps/desktop-ui
npm install
npm run dev
```

## Backend-only run

```powershell
cargo run -p desktop --bin desktop
```

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE).
