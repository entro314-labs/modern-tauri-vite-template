# Ludusavi → Tauri: Reusable Patterns & Code

Source project: [ludusavi](https://github.com/mtkennerly/ludusavi) — a Rust desktop app (CLI + Iced GUI) with deeply battle-tested cross-platform infrastructure code. This document catalogues the patterns extracted and the modules added under `src-tauri/src/foundations/` in this scaffold.

Each section names the **upstream file** and the **port location** in this repo.

---

## 1. `StrictPath` — typed cross-platform path wrapper

- **Upstream:** `src/path.rs` (~2000 LOC)
- **Port:** `src-tauri/src/foundations/path.rs`

Wraps raw path strings with explicit semantics:

| Method        | Purpose                                                                       |
| ------------- | ----------------------------------------------------------------------------- |
| `raw()`       | The original string, untouched.                                               |
| `render()`    | Display-friendly form (forward slashes, placeholders resolved).               |
| `interpret()` | OS-native form, canonicalized when possible. Cached behind `Arc<Mutex<...>>`. |
| `globbable()` | Glob-escaped form for use with `globset`/`globetter`.                         |
| `access()`    | OS-native form without canonicalization (used when target may not exist yet). |

Handles all the gnarly Windows edge cases: verbatim paths (`\\?\`), UNC (`\\?\UNC\server\share`), DeviceNS, mixed forward/back slashes, long-path > 260 chars.

Placeholders supported:

```
~              → home dir
<home>         → home dir
<xdgConfig>    → config dir (cross-platform)
<xdgData>      → data dir (cross-platform)
<winAppData>   → Windows roaming AppData (= xdgData on Win)
<winLocalAppData>     → Windows local AppData
<winLocalAppDataLow>  → Windows AppData\LocalLow (via known-folders crate)
<winDocuments>        → Windows Documents
<winPublic>           → C:\Users\Public
<winProgramData>      → C:\ProgramData
<winDir>              → C:\Windows
<savedGames>          → Windows %USERPROFILE%\Saved Games
<osUserName>          → current OS user
```

Cached canonicalization invalidates with `invalidate_cache()` — important for paths persisted in config across sessions.

---

## 2. `CommonPath` — standard-directory enum

- **Upstream:** `src/path.rs:27–110`
- **Port:** `src-tauri/src/foundations/path.rs` (same module)

`LazyLock`-cached lookup over `dirs::*` + Windows `known-folders` crate for `LocalAppDataLow` and `SavedGames` (which the `dirs` crate doesn't expose). Returns both plain and glob-escaped variants.

```rust
let cfg = CommonPath::Config.get();         // Option<&str>
let pattern = CommonPath::Data.get_globbable();
```

---

## 3. Resource / config trait pattern

- **Upstream:** `src/resource.rs`, `src/resource/cache.rs`
- **Port:** `src-tauri/src/foundations/resource.rs`

```rust
pub trait ResourceFile: Default + serde::de::DeserializeOwned {
    const FILE_NAME: &'static str;
    fn path() -> StrictPath { app_dir().joined(Self::FILE_NAME) }
    fn migrate(self) -> Self { self }
    fn initialize(self) -> Self { self }
    fn load() -> Result<Self, AnyError> { ... }
}

pub trait SaveableResourceFile: ResourceFile + serde::Serialize {
    fn save(&self) { /* skips write if content unchanged */ }
}
```

Why it's good:

- One struct + one trait impl + one const → versioned, migratable, typed persistence.
- `save()` diffs before writing — fewer FS events, no spurious dirty markers.
- `migrate()` hook for legacy schema migrations.
- Beats `tauri-plugin-store` if you want strong types end to end.

---

## 4. Portable-mode detection + `app_dir()`

- **Upstream:** `src/prelude.rs:161–176`
- **Port:** `src-tauri/src/foundations/app.rs`

Three-tier precedence:

1. CLI override (`--config-dir`) → stored in `CONFIG_DIR` mutex.
2. Portable flag file (`<name>.portable` next to the exe) → use exe's parent dir.
3. OS standard config dir (`dirs::config_dir() + APP_DIR_NAME`).

Critical for any app shipped both as installer **and** portable zip.

---

## 5. Logging + panic hook

- **Upstream:** `src/main.rs:18–95`
- **Port:** `src-tauri/src/foundations/logging.rs`

`flexi_logger` with:

- Log rotation (10MB × 4 files in release; verbose debug log when `debug=true`).
- UTC ISO-8601 timestamps with module-path prefix.
- `LoggerHandle` kept alive — required by `flexi_logger`.
- Custom panic hook that captures backtrace, logs it, flushes the logger, then delegates to the original hook.

Drop the handle into Tauri state via `Builder::manage(_)` so it lives for the whole app lifetime.

---

## 6. Windows console-detach pattern

- **Upstream:** `src/main.rs:97–183` (long comment block worth reading)
- **Port:** `src-tauri/src/foundations/console_win.rs`

Lets a single binary run as both CLI (with stdout) and GUI (no console flash). `FreeConsole` + reset stdin/stdout/stderr handles after CLI parsing. Avoids the `#![windows_subsystem = "windows"]` footgun where Command Prompt output bleeds into the next prompt line. Only matters if you ship a CLI alongside your Tauri binary.

---

## 7. `run_command` + `CommandError` + `Privacy`

- **Upstream:** `src/prelude.rs:227–311` (sync), `src/cloud.rs:36–200` (async event-stream)
- **Port:** `src-tauri/src/foundations/command.rs`

Synchronous wrapper around `std::process::Command` with:

- Captured stdout/stderr returned in `CommandOutput`.
- `Privacy::Private` redacts args from logs (good for tokens, paths).
- Allowed exit-code list (`success: &[0, 9]`).
- `CREATE_NO_WINDOW` on Windows so spawned children don't flash a console.
- Structured error variants: `Launched` (couldn't spawn), `Terminated` (signal-killed), `Exited { code, stdout, stderr }`.

`CommandError::command()` reconstructs a shell-pastable string for log readability.

---

## 8. OS detection — compile-time + runtime

- **Upstream:** `src/resource/manifest.rs:48–87`
- **Port:** `src-tauri/src/foundations/os.rs`

```rust
pub enum Os { Windows, Linux, Mac, Other }

impl Os {
    pub const HOST: Os = if cfg!(windows) { Os::Windows }
        else if cfg!(target_os = "macos") { Os::Mac }
        else if cfg!(target_os = "linux") { Os::Linux }
        else { Os::Other };
}
```

`Os::HOST` is a `const` — usable directly in `match` arms. Saves you `cfg!(target_os = ...)` sprinkled everywhere.

Bundled with `LazyLock` runtime detection of common deployment shapes: Steam Deck, Flatpak, Snap, AppImage.

---

## 9. HTTP client builder with user-controlled TLS

- **Upstream:** `src/prelude.rs:391–421`
- **Port:** `src-tauri/src/foundations/http.rs`

```rust
pub enum Security { Safe, Unsafe }   // serde + JsonSchema

pub fn reqwest_client(security: Security) -> reqwest::Client { ... }
```

`Unsafe` uses `danger_accept_invalid_certs(true)` — useful for users behind corporate proxies with broken cert chains. Lives in user config so you don't ship a "disable TLS" flag they have to discover.

---

## 10. ETag-aware remote resource update

- **Upstream:** `src/resource/manifest.rs:471–580`
- **Port:** See `docs/snippets/etag_update.rs` (pattern, not lifted whole)

- `If-None-Match` based update with stored ETag.
- Validates response **before** writing to disk (parse first, save second).
- 24-hour throttle via cache timestamps.
- Atomic cache + manifest updates.

Reusable for any app that pulls remote config / template / content packs.

---

## 11. SIGINT handling

- **Upstream:** `src/prelude.rs:313–350`
- **Port:** `src-tauri/src/foundations/signal.rs` (`cfg(unix)` only)

Idempotent register/unregister so GUI and CLI paths don't fight over the handler. Returns `Arc<AtomicBool>` that worker threads can poll for cancellation. Cleaner than `tokio::select! + ctrl_c()` when you also have rayon workers to interrupt.

---

## 12. Configurable parallelism

- **Upstream:** `src/prelude.rs:41–42, 193–215`
- **Port:** `src-tauri/src/foundations/parallel.rs`

Three-source precedence for thread count: env var → config → default. Sets up `rayon::ThreadPoolBuilder::new().build_global()`.

---

## 13. Windows manifest — long-path, DPI, common controls

- **Upstream:** `build.rs` + `assets/windows/manifest.xml`
- **Port:** `src-tauri/build.rs` + `src-tauri/assets/windows/manifest.{rc,xml}`

```xml
<dpiAware>true/pm</dpiAware>
<dpiAwareness>permonitorv2</dpiAwareness>
<longPathAware>true</longPathAware>
```

Three settings that fix three common Windows bugs:

- Per-monitor v2 DPI.
- Long-path > 260 chars — without this, `std::fs` silently fails on deep paths even on Windows 10+.
- Common Controls v6 — needed for `rfd` / `winapi` native dialogs to render correctly.

Tauri's installer manifest handles DPI for the WebView host process, but **not** for any extra CLI binary you ship alongside.

---

## 14. Launcher / context detection via env vars

- **Upstream:** `src/wrap.rs`, `src/wrap/heroic.rs`
- **Port:** Not in foundations (app-specific). See pattern below.

Probe a list of env vars at startup and merge the strongest signal into a `LaunchContext`. Same idea works for any app that can be opened via custom URL scheme, file association, CLI args, **and** parent process.

```rust
pub struct LaunchContext {
    pub url: Option<Url>,             // from custom URL scheme
    pub files: Vec<StrictPath>,       // from file association
    pub args: Vec<String>,            // from CLI
    pub parent: Option<String>,       // from parent process or env
}
```

---

## 15. Concrete crate choices

Pulled into `Cargo.toml`:

| Crate                 | Why                                                                    |
| --------------------- | ---------------------------------------------------------------------- |
| `dirs`                | Cross-platform standard directories.                                   |
| `known-folders` (Win) | `LocalAppDataLow`, `SavedGames` — not in `dirs`.                       |
| `typed-path`          | Parse Windows/Unix paths regardless of host OS.                        |
| `globset`             | Glob escaping + matching.                                              |
| `chrono`              | Timestamps with serde + JsonSchema.                                    |
| `flexi_logger`        | Rotating file logger.                                                  |
| `log`                 | Facade.                                                                |
| `reqwest`             | rustls-tls + blocking + gzip.                                          |
| `schemars`            | JSON Schema emission for config (frontend tooling, IDE autocomplete).  |
| `serde_yaml`          | Human-editable config persistence.                                     |
| `sha1`                | Cheap content fingerprint.                                             |
| `filetime`            | Cross-platform mtime preservation.                                     |
| `whoami`              | OS username for placeholder substitution.                              |
| `which`               | Cleaner than spawning `which`/`where`.                                 |
| `opener`              | Open URL or path in default app (works in any binary, not just Tauri). |
| `rayon`               | Parallel scanning.                                                     |
| `signal-hook` (unix)  | SIGINT.                                                                |
| `winreg` (win)        | Windows registry.                                                      |
| `windows` (win)       | Direct Win32 calls (FreeConsole, CopyFileExW, CREATE_NO_WINDOW).       |

---

## 16. Cargo profile + lints worth copying

```toml
[profile.dev]
opt-level = 1                  # debuggable
[profile.dev.package."*"]
opt-level = 3                  # but deps fully optimized

[profile.release]
lto = "thin"
strip = true

[lints.clippy]
large_enum_variant = "allow"   # large config enums are fine
new_without_default = "allow"
too_many_arguments = "allow"
```

---

## 17. Linux desktop integration

- `.desktop` file (Tauri's bundler writes one).
- AppStream `metainfo.xml` — needed for KDE Discover, GNOME Software, Flathub. Tauri does **not** generate this. See `assets/linux/` for an example.

---

## Order of leverage (copy these first)

1. `StrictPath` + `CommonPath` (foundations/path.rs).
2. `ResourceFile` / `SaveableResourceFile` traits (foundations/resource.rs).
3. Logging + panic hook (foundations/logging.rs).
4. `app_dir()` + portable detection (foundations/app.rs).
5. `Os` enum + `HOST` const (foundations/os.rs).
6. Windows manifest (build.rs + assets/windows/manifest.\*).
7. `run_command` + `Privacy` (foundations/command.rs).
8. HTTP client builder + `Security` (foundations/http.rs).
