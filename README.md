# RRRex — a radial application launcher for Windows

Hold the middle mouse button anywhere in Windows, move toward an application, release to launch. RRRex lives in the system tray and appears around your cursor in a fraction of a second.

![RRRex](src-tauri/app-icon.png)

## Features

- **Global trigger** — press-and-hold the middle mouse button (or X1/X2/right) anywhere, even when another application has focus. Implemented with a native Windows low-level mouse hook (`WH_MOUSE_LL`), not browser events.
- **Radial menu** — glassy, dark translucent circular menu with smooth open animation, selection ring + glow, minimal labels and a dead-zone in the center. Any number of items, all positions computed from the sector math.
- **Native selection** — the sector containing the cursor is computed by the Rust hook (`atan2`-based, with dead zone and configurable hysteresis to stop boundary flicker).
- **Click-through, focus-free overlay** — the overlay never steals focus, never shows in the taskbar/alt-tab, and closes instantly after selection.
- **Native launching** — apps are started via Win32 process APIs (`CreateProcess`/`ShellExecuteEx` for `.lnk`), with arguments and working directories. Missing or permission-restricted executables are reported gracefully.
- **Real icons** — icons are extracted from the executables/shortcuts with `IShellItemImageFactory` and cached.
- **Settings window** — General, Appearance (with live preview), Applications (add/remove/reorder/edit/enable), Behavior and About.
- **System tray** — open settings, enable/disable, reload configuration, about, quit.
- **Start with Windows** — per-user Run key (HKCU), no admin rights.
- **Multi-monitor & DPI aware** — Per-Monitor V2; all coordinates are physical pixels; the menu is clamped to the monitor work area.

## Install

Run the NSIS installer produced by the Tauri bundler (`RRRex_1.0.0_x64-setup.exe`) or see **Build** below. Windows 10/11 x64 with WebView2 Runtime (installed on demand by the installer).

## Usage

1. RRRex starts in the tray. Open **Settings** from the tray (left-click the tray icon) to add applications.
2. Press and hold the **middle mouse button** anywhere.
3. Move toward an item and release to launch. Release over the center (or outside the ring) to dismiss.

While the launcher is enabled, the trigger button is captured globally — other applications won't see it. Disable the launcher from the tray or Settings to give the button back. This is by design: it guarantees the underlying app never receives stray half-clicks when the menu opens/closes.

Closing the settings window keeps the app running in the tray; use **Quit** to exit.

## Build

Prerequisites:

- [Rust](https://rustup.rs) (stable, `x86_64-pc-windows-msvc`) — includes MSVC linker via Visual Studio Build Tools
- [Node.js](https://nodejs.org) 20+
- WebView2 Runtime (preinstalled on Windows 11, common on 10)

```bash
npm install
npm run tauri dev     # development
npm run tauri build   # production build + NSIS installer (src-tauri/target/release/bundle/nsis/)
```

Other checks:

```bash
npm run typecheck     # tsc --noEmit (strict)
npm run lint          # eslint
npm run test          # vitest unit tests (frontend geometry + config validation)
cargo test            # Rust unit tests (selection, geometry, config, menu state machine, launcher)
cargo fmt && cargo clippy
npm run verify        # end-to-end test on Windows (see below)
```

## Architecture

```
├─ src/                          # Frontend (React + TypeScript, strict)
│  ├─ components/                # RadialMenu, RadialItem, AppIcon, SelectionIndicator,
│  │                             # ApplicationPicker, SettingsWindow, controls
│  ├─ pages/Settings.tsx         # General / Appearance / Applications / Behavior
│  ├─ pages/About.tsx
│  ├─ overlay/Overlay.tsx        # renderer for the transparent overlay window
│  ├─ lib/geometry.ts            # radial math + selection (unit tested)
│  ├─ lib/configValidation.ts    # config sanitization (unit tested)
│  ├─ lib/ipc.ts, lib/events.ts # typed IPC surface
│  └─ hooks/                     # useConfig (debounced save), useAppIcons
└─ src-tauri/src/
   ├─ main.rs                    # assembly: windows, tray, listeners, plugins
   ├─ mouse.rs                   # WH_MOUSE_LL hook thread + input state machine
   ├─ menu.rs                    # menu runtime, release/click decisions (unit tested)
   ├─ selection.rs               # native sector math — twin of lib/geometry.ts (unit tested)
   ├─ geometry.rs                # work-area clamping (unit tested)
   ├─ overlay.rs                 # overlay window control (main thread only)
   ├─ win32.rs                   # DPI awareness, monitor/work-area, no-activate helpers
   ├─ launcher.rs                # CreateProcess / ShellExecuteEx + structured errors
   ├─ icons.rs                   # IShellItemImageFactory icon extraction + PNG cache
   ├─ config.rs                  # config model, sanitization, atomic JSON persistence
   ├─ commands.rs                # Tauri IPC commands (narrow surface)
   ├─ tray.rs                    # tray icon + context menu
   ├─ state.rs                   # process-wide shared state
   └─ logger.rs                  # file logging (AppData/com.rrrex.launcher/rrrex.log)
```

### Input pipeline (the important part)

1. The hook thread owns the `WH_MOUSE_LL` hook and its message pump.
2. On trigger **down** the hook consumes the event (the underlying app never sees it), computes the clamped menu center in physical pixels using the monitor's work area and DPI scale, snapshots the enabled apps + geometry, and emits the render payload to the overlay webview.
3. The webview commits the payload and **acks**; only then does the main thread reveal the window via `SetWindowPos(SWP_SHOWWINDOW|SWP_NOACTIVATE)` — no stale-frame flash, no focus steal (`WS_EX_NOACTIVATE`).
4. While the button is held, every `WM_MOUSEMOVE` is turned into a sector selection (dead zone → none, adjacent-sector hysteresis → no flicker). Selection changes are emitted to the webview at most once per change (cursor dot throttled to ~60 Hz).
5. On trigger **up** the hook resolves the selection and either launches (worker thread, Win32 API) or enters sticky mode. The overlay is hidden immediately and all events are consumed.

The overlay is **click-through** (`WS_EX_TRANSPARENT`) — mouse input physically passes through to the app underneath, and the native hook remains the single owner of pointer routing. In sticky mode, clicks are routed by the hook's own hit-testing.

### Coordinates & DPI

The process is forced to Per-Monitor-V2 DPI awareness at startup, so hook coordinates, `GetCursorPos`, monitor rectangles and window positions are all **physical pixels** and mutually consistent at 100–200% scaling. Configuration values (radius, icon size, dead zone) are logical CSS pixels; the Rust layer converts with the monitor scale factor when the menu opens. The menu clamps its center into the containing monitor's work area, so it stays fully visible near screen edges and works on any monitor layout.

### Persistence

`%APPDATA%\com.rrrex.launcher\config.json` — validated and sanitized on load (corrupt files are backed up as `config.corrupt.json` and defaults are used), written atomically (temp file + rename). Extracted icons are cached under `%APPDATA%\com.rrrex.launcher\icons\`.

### Security

- Strict CSP; no `withGlobalTauri`; capability file limited to `core:default` for both windows.
- The webview can only call the narrow IPC commands; executable paths are never launched from webview input — only from the persisted, validated configuration by id.
- `.lnk` shortcuts are launched through `ShellExecuteEx` with structured parameters (never a shell string); `.exe` through `CreateProcess`.
- Process spawning, mouse hooking and window management stay in the native layer.

## Testing

### Automated checks

- **42 frontend unit tests** (vitest): angle normalization, item positioning for 4/6/8/12 items, sector selection, boundaries, dead zone, hysteresis (including wrap-around), hit testing, config sanitization/clamping/dedup, command-line splitting.
- **41 Rust unit tests** (cargo test): mirror of the selection math, work-area clamping (incl. secondary monitors, tiny work areas), config sanitize/persist/roundtrip/corruption recovery, menu open/scale/clamp/selection/throttle, release decisions, sticky click routing, launcher validation (missing executable, invalid working directory).
- `tsc --noEmit` (strict), ESLint, `cargo fmt --check`, `cargo clippy -D warnings`.
- **`npm run verify`** — end-to-end harness that drives the real release binary through its debug simulation input (RRREX_SIMULATE=1) and asserts the whole pipeline: menu open with DPI-correct geometry, overlay shown via the rendered-ack path, selection tracking, release → the selected application **actually launches** (Win32), and instant close; plus a dead-zone release that closes without launching. This harness exists because the dev workstation used for verification silently drops SendInput and never delivers low-level-hook callbacks (see below), which rules out OS-level input injection there.
- **`npm run verify:input`** — the same end-to-end flow driven by real `SendInput` mouse events, for machines that allow input injection (typical home machines). On the dev workstation this harness cannot work: `SendInput` events are accepted into the input stream (GetLastInputInfo updates) but are never applied to the cursor and never reach *any* `WH_MOUSE_LL` hook — verified independently with a known-good C# hook (`scripts/csharp-hook-test.ps1`) and a minimal Rust probe (`src-tauri/src/bin/hook_probe.rs`). The launcher's own hook code follows the canonical pattern exactly and fails identically to the reference probes on that machine, i.e. the block is environmental.

### What still needs a human hand (2 minutes, on a normal desktop)

Because low-level hook delivery could not be exercised by synthetic input on the dev machine, verify once by hand:

1. Build/start the app (`npm run tauri build` output or `target\release\rrrex.exe`).
2. Click into any other application (e.g. a browser).
3. Press and hold the **middle mouse button** → the radial menu must appear around the cursor.
4. Move toward an item → it must highlight; the underlying app must not receive the click.
5. Release over the item → the app launches and the menu closes instantly.
6. Release over the center → the menu closes, nothing launches.
7. Repeat near each screen edge and on a second monitor if available.
8. Tray: left-click toggles settings; right-click shows the menu (enable/disable, reload, about, quit).
9. Check the tray *Quit* releases the hook cleanly (Task Manager shows no lingering input lag).

Verified on the dev machine (Windows 10 22H2, x64, 125% DPI): app boots to tray, settings window renders and persists configuration, hook installs, the full simulated pipeline passes (see `npm run verify`), real process launching works (Notepad launched by the pipeline), steady-state idle CPU is 0.00% over 25 s, and clean shutdown. Multi-monitor and 100/150/200% DPI paths are unit tested but not exercised on the single-monitor dev hardware.

## Limitations & notes

- While enabled, the trigger button is captured globally system-wide.
- A low-level hook has a system timeout (`LowLevelHooksTimeout`); our callback is O(µs), but heavy third-party hooks can degrade global input delivery.
- WebView2 cannot blur the desktop behind a transparent window, so the glass effect is painted rather than a real acrylic blur.
- Sticky mode has no Escape-to-close (the overlay never takes keyboard focus by design); click outside the menu to dismiss.
- A global keyboard hotkey is not included in this release (planned; the trigger-first design doesn't depend on it).

## License

MIT
