# Knobset desktop app

Tauri 2 + React + TypeScript + shadcn/ui. The Rust side (`src-tauri`) is a
thin layer over `crates/g5-core`.

```sh
npm install
npm run tauri dev     # desktop app against a real pager
npx vite              # UI only, in a browser, against the mock backend (src/lib/mock.ts)
```

Settings live in `~/Library/Application Support/com.harpchad.knobset/settings.json`
on macOS: the Unication PPS `DataBase` folder (templates, never bundled) and the
backups folder.
