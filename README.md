![CI](https://img.shields.io/github/actions/workflow/status/your-org/tauri-app/ci.yml?branch=main&label=CI)
![License](https://img.shields.io/badge/license-MIT-green)
![Tauri](https://img.shields.io/badge/Tauri-2.10.2-24C8DB)
![Rust](https://img.shields.io/badge/Rust-1.85%2B-orange)

## ⚠️ Bleeding-edge scaffold

This template intentionally uses pre-stable tooling:

- **Vite 8 (beta)** — rolldown bundler
- **oxfmt (alpha)** — formatter
- **tsgolint (prototype)** — type-aware linting backend via `oxlint-tsgolint`

## Prerequisites

- Node.js **20+**
- `pnpm`
- Rust stable (MSRV **1.85+** for this template)
- Platform dependencies for Tauri: https://tauri.app/start/prerequisites/

## Setup

```bash
pnpm install
pnpm tauri dev
pnpm tauri build
```

`src/routeTree.gen.ts` is generated on first dev server startup. Run `pnpm tauri dev` once before TypeScript is fully happy.

## Project structure

| Path | Purpose |
| --- | --- |
| `src/assets` | Static assets |
| `src/components/ui` | Shared primitive UI components |
| `src/routes` | TanStack Router file-based route modules |
| `src/stores` | Zustand stores (app + persisted settings) |
| `src/hooks` | Reusable hooks (typed Tauri event listener) |
| `src/lib` | Typed Tauri invoke wrappers |
| `scripts` | Optional release/version/checksum automation scripts |
| `src/styles` | Global Tailwind import and CSS variables |
| `src-tauri/src/commands` | Tauri command functions |
| `tests` | Frontend tests |
| `.github/workflows` | CI pipelines |

## Add a new Tauri command

1. Define the command function (and any serializable structs) in `src-tauri/src/commands/mod.rs`.
2. Register the command in `src-tauri/src/lib.rs` via `tauri::generate_handler!`.
3. Add a typed frontend wrapper in `src/lib/tauri.ts` using `invoke()`.
4. Import and call the wrapper from a route or component.

## Stack

- Tauri 2 — https://tauri.app/
- Rust 2024 edition — https://doc.rust-lang.org/edition-guide/rust-2024/index.html
- Vite 8 beta — https://vite.dev/
- React 19 — https://react.dev/
- TypeScript 5 — https://www.typescriptlang.org/docs/
- `@vitejs/plugin-react-swc` — https://www.npmjs.com/package/@vitejs/plugin-react-swc
- Tailwind CSS v4 (`@tailwindcss/vite`) — https://tailwindcss.com/docs/installation/using-vite
- Zustand 5 — https://zustand.docs.pmnd.rs/
- TanStack Router v1 — https://tanstack.com/router/latest
- Vitest — https://vitest.dev/
- Testing Library React — https://testing-library.com/docs/react-testing-library/intro/
- Oxlint / Oxfmt / Oxlint tsgolint backend — https://oxc.rs/
- Knip — https://knip.dev/
- Lefthook — https://lefthook.dev/

## Optional release tooling

The scaffold includes optional release automation scripts under `scripts/`.

```bash
# validate release prerequisites on the current machine
pnpm release:preflight

# synchronize version across package.json, tauri.conf.json and Cargo.toml
pnpm version:bump 0.2.0

# after running a release build, generate .sha256 files + CHECKSUMS.txt
pnpm release:checksums

# verify one artifact against its checksum file
pnpm release:verify src-tauri/target/release/bundle/appimage/tauri-app.AppImage
```

### Advanced platform workflows (optional)

```bash
# macOS: signed + notarized DMG (requires Apple signing/notary env vars and explicit paths)
pnpm release:macos:dmg-notarized -- --app src-tauri/target/release/bundle/macos/tauri-app.app --out build/tauri-app-v0.1.0-macos-aarch64-signed.dmg

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