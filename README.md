![CI](https://img.shields.io/github/actions/workflow/status/your-org/tauri-app/ci.yml?branch=main&label=CI)
![License](https://img.shields.io/badge/license-MIT-green)
![Tauri](https://img.shields.io/badge/Tauri-2.10.3-24C8DB)
![Rust](https://img.shields.io/badge/Rust-1.94%2B-orange)

## ✅ Modern stable scaffold

This template is configured around current stable releases and modern defaults:

- **Vite 7** + **Vitest 4** + **TypeScript 5.9**
- Tauri Vite integration settings for host/HMR/build behavior
- Tauri capability-based command permissions (`src-tauri/capabilities/default.json`)
- Two-phase auto-updater (staged install-on-exit, release channels, settings card) — setup in `docs/UPDATER.md`; ships with placeholders guarded by `pnpm run check:updater`
- Strict TypeScript and typed linting checks (`pnpm check` includes `typecheck`)

## Prerequisites

- Node.js **20.19+** or **22.12+**
- `pnpm`
- Rust stable (MSRV **1.94+** for this template)
- Platform dependencies for Tauri: https://v2.tauri.app/start/prerequisites/

## Setup

```bash
pnpm install
pnpm tauri dev
pnpm tauri build
```

`src/routeTree.gen.ts` is generated on first dev server startup. Run `pnpm tauri dev` once before TypeScript is fully happy.

## Project structure

| Path                     | Purpose                                              |
| ------------------------ | ---------------------------------------------------- |
| `src/assets`             | Static assets                                        |
| `src/components/ui`      | Shared primitive UI components                       |
| `src/routes`             | TanStack Router file-based route modules             |
| `src/stores`             | Zustand stores (app + persisted settings)            |
| `src/hooks`              | Reusable hooks (typed Tauri event listener)          |
| `src/lib`                | Typed Tauri invoke wrappers                          |
| `scripts`                | Optional release/version/checksum automation scripts |
| `src/styles`             | Global Tailwind import and CSS variables             |
| `src-tauri/src/commands` | Tauri command functions                              |
| `tests`                  | Frontend tests                                       |
| `.github/workflows`      | CI pipelines                                         |

## Add a new Tauri command

1. Define the command function (and any serializable structs) in `src-tauri/src/commands/mod.rs`.
2. Register the command in `src-tauri/src/lib.rs` via `tauri::generate_handler!`.
3. Add the command name to `src-tauri/build.rs` in `AppManifest::commands(&[...])` so `allow-*` / `deny-*` permissions are generated.
4. Reference the generated `allow-*` permission in `src-tauri/capabilities/default.json`.
5. Add a typed frontend wrapper in `src/lib/tauri.ts` using `invoke()`.
6. Import and call the wrapper from a route or component.

## Stack

- Tauri 2 - https://v2.tauri.app/
- Rust 2024 edition - https://doc.rust-lang.org/edition-guide/rust-2024/index.html
- Vite 7 - https://vite.dev/
- React 19 - https://react.dev/
- TypeScript 5.9 - https://www.typescriptlang.org/docs/
- `@vitejs/plugin-react` - https://www.npmjs.com/package/@vitejs/plugin-react
- Tailwind CSS v4 (`@tailwindcss/vite`) - https://tailwindcss.com/docs/installation/using-vite
- Zustand 5 - https://zustand.docs.pmnd.rs/
- TanStack Router v1 - https://tanstack.com/router/latest
- Vitest - https://vitest.dev/
- Testing Library React - https://testing-library.com/docs/react-testing-library/intro/
- Oxlint / Oxfmt / Oxlint tsgolint backend - https://oxc.rs/
- Knip - https://knip.dev/
- Lefthook - https://lefthook.dev/

## Security defaults

- Capability files are enabled and scoped to the `main` window in `src-tauri/capabilities/default.json`.
- App command permissions are generated from `build.rs` (`allow-greet`, `allow-get-app-info`) via `tauri_build::AppManifest::commands`.
- `freezePrototype` is enabled in `src-tauri/tauri.conf.json`.

## Optional release tooling

The scaffold includes optional release automation scripts under `scripts/`.

```bash
# validate release prerequisites on the current machine
pnpm release:preflight

# build OS-specific bundles via platform config overlays
pnpm tauri:build:linux
pnpm tauri:build:macos
pnpm tauri:build:windows

# synchronize version across package.json, tauri.conf.json and Cargo.toml
pnpm version:bump 0.2.0

# increment version automatically via build counter
pnpm version:increment
pnpm version:build
pnpm version:patch
pnpm version:info

# after running a release build, generate .sha256 files + CHECKSUMS.txt
pnpm release:checksums

# verify one artifact against its checksum file
pnpm release:verify src-tauri/target/release/bundle/appimage/tauri-app.AppImage
```

Platform config overlays live in:

- `src-tauri/tauri.linux.conf.json`
- `src-tauri/tauri.macos.conf.json`
- `src-tauri/tauri.windows.conf.json`

These are merged with `src-tauri/tauri.conf.json` at build-time when using `--config`.

### Advanced platform workflows (optional)

```bash
# macOS: signed + notarized DMG (requires Apple signing/notary env vars and explicit paths)
pnpm release:macos:dmg-notarized -- --app src-tauri/target/release/bundle/macos/tauri-app.app --out build/tauri-app-v1.0.0-macos-aarch64-signed.dmg

# Windows: build with Microsoft Store config and collect installers into build/
pnpm release:windows:msstore

# Windows: sign latest installers (or pass explicit -Input/-Output args)
pnpm release:windows:sign

# Android: generate release keystore and encode for GitHub Actions secrets
pnpm release:android:keystore
pnpm release:android:keystore:encode -- src-tauri/gen/android/keystore/release-key.jks
```

macOS notarization script requires these environment variables:

- `APPLE_SIGN_ID`
- `APPLE_NOTARY_KEY_ID`
- `APPLE_NOTARY_ISSUER_ID`
- `APPLE_NOTARY_KEY_PATH`

## License

MIT. See `LICENSE`.
