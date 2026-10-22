# CH. Clicker — Architecture (v0.1)

> Sources: the requirements prompt, the design handoff (screen specs and the HTML prototype) and the design tokens (`design/tokens.json`).
> This document contains no implementation code. Pseudocode appears only where an algorithm must be unambiguous.

---

## 1. Final stack

| Layer        | Choice                                                                                  | Why                                                                                                                                   |
| ------------ | --------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| App shell    | **Tauri 2**                                                                             | Small binary, system WebView2, NSIS/MSI bundler, official updater, autostart, notification and single-instance plugins.               |
| Native core  | **Rust (stable) + the `windows` crate (windows-rs)**                                    | Direct access to `SendInput`, high-resolution timers, `RegisterHotKey`, LL hooks, DPI, GDI. No GC, so the engine thread never pauses. |
| UI           | **React 19 + TypeScript 6 + Vite 8 + Tailwind CSS v4**                                  | Reuses the web design system. Tokens become CSS variables (`themes.css`) mapped in Tailwind's `@theme`.                               |
| UI state     | **Zustand** (one mirrored store per window)                                             | Light, no boilerplate. Rust is the source of truth, and the store only mirrors what arrives through events.                           |
| IPC bindings | **tauri-specta**                                                                        | Generates TS types from the Rust commands and events, so the section 7 contract is checked at compile time on both sides.             |
| i18n         | **i18next + react-i18next + i18next-icu**                                               | ICU plurals and numbers. Rust reads the same JSON for the tray and notifications.                                                     |
| Fonts        | **@fontsource/syne** (800) and **@fontsource/space-grotesk** (400–700)                  | Bundled locally, as the handoff requires.                                                                                             |
| Tests        | `cargo test` + `proptest`, **Vitest** + Testing Library, **Playwright** (visual parity) | See section 9.                                                                                                                        |

### Alternatives considered

| Criterion                      | Tauri 2 (Rust + WebView2)                                  | C# WinUI 3 / WPF                                                                                                                     | Electron                                                                                      |
| ------------------------------ | ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------- |
| Timing precision               | Great: dedicated native thread, no GC.                     | Good, but the .NET GC can pause the engine thread (mitigable with `GCSettings.LatencyMode` and zero allocation, which is hard work). | The engine would have to be a native addon (N-API) anyway, so it becomes Rust/C++ + Electron. |
| Installer size                 | ~5–10 MB                                                   | ~1 MB framework-dependent, 60–150 MB self-contained (WinUI 3 still needs the Windows App SDK).                                       | 80–150 MB                                                                                     |
| Memory in use                  | ~50–90 MB (shared WebView2)                                | ~40–80 MB                                                                                                                            | 150–300 MB                                                                                    |
| Themes and i18n                | Reuses the website CSS/tokens.                             | XAML ResourceDictionaries: fine, but it means rewriting the design system.                                                           | Same as Tauri.                                                                                |
| Parity with the HTML prototype | Direct (same CSS: `color-mix`, `oklch`).                   | Rebuild everything in XAML. `oklch`/`color-mix` do not exist and must be precomputed.                                                | Direct.                                                                                       |
| Packaging and signing          | Native NSIS/MSI, signed updater (minisign) + Authenticode. | MSIX/ClickOnce, good.                                                                                                                | electron-builder, good.                                                                       |

**Decision:** **Tauri 2**. The deciding point: critical timing lives in a Rust thread that does not depend on the WebView, and the UI reuses the web design system. The cost is the complexity of special windows (click-through HUD, per-monitor overlays), which need Win32 calls on Tauri's HWNDs. Sections 3 and 5 plan for that.

---

## 2. Repository layout

```
ch-auto-clicker/
├─ Cargo.toml                  # Rust workspace
├─ package.json                # pnpm workspace (ui)
├─ crates/
│  ├─ ch-core/                 # pure Rust, NO Windows dependency (testable on any OS)
│  │   ├─ model/               # Profile, Settings, Hotkey, enums, validation
│  │   ├─ schema/              # versioning, v1→vN migrations, .chprofile format
│  │   ├─ patterns/            # interval cycle, wiggle, zigzag, multi-point
│  │   ├─ engine/              # state machine, generic scheduler over a Clock trait
│  │   ├─ geometry/            # rectangles, monitor layout, 0..65535 normalization
│  │   └─ stats/               # jitter histogram (p50/p95/p99/max)
│  ├─ ch-win/                  # Windows implementations (`windows` crate)
│  │   ├─ input/               # InputBackend trait: SendInputBackend, InterceptionBackend (feature)
│  │   ├─ timing/              # QpcClock, HighResWaiter (HR waitable timer + fallback), MMCSS
│  │   ├─ display/             # EnumDisplayMonitors, DPI, work area, WM_DISPLAYCHANGE
│  │   ├─ hotkeys/             # hotkey thread: RegisterHotKey + WH_KEYBOARD_LL/WH_MOUSE_LL
│  │   ├─ capture/             # per-monitor GDI capture (BitBlt), pixel sampling
│  │   ├─ window/              # HUD styles (WS_EX_*), animated resize, "own window" guard
│  │   └─ theme/               # AppsUseLightTheme + WM_SETTINGCHANGE
│  ├─ ch-engine/               # runtime: engine thread, RunPlan, executors, telemetry
│  └─ ch-storage/              # %APPDATA%, atomic writes, backup, debounce, import/export
├─ src-tauri/                  # Tauri app: commands, events, windows, tray, updater, autostart
│   ├─ commands/               # one module per domain (profiles, settings, engine, hotkeys, overlay, window, app)
│   ├─ events.rs               # typed names and payloads (specta)
│   ├─ windows/                # main, variation, hud, overlay-N
│   ├─ tray/                   # native menu, per-state icons
│   └─ icons/                  # generated icons (16/20/24/32/48/256)
├─ ui/
│   ├─ src/
│   │   ├─ entries/            # main.tsx, variation.tsx, hud.tsx, overlay.tsx (Vite multi-page)
│   │   ├─ ipc/                # generated bindings + wrappers + test mocks
│   │   ├─ store/              # Zustand: engine, profiles, settings, hotkeys
│   │   ├─ components/         # design system primitives (Segmented, Toggle, RadioCard, Stepper, BigNumberField, Kbd, Badge, Button, Popover, Callout, Slider)
│   │   ├─ features/           # click, wiggle, scan, points, variation, profiles, hotkeys, themes, settings, overlay, hud
│   │   ├─ i18n/locales/{pt-BR,en-US,es-419}/*.json
│   │   ├─ styles/             # themes.css (generated), base.css, fonts
│   │   └─ dev/gallery/        # development route with every screen in every state (visual parity)
│   └─ tests/visual/           # Playwright + reference screenshots from the prototype
├─ design/                     # tokens.json (source of truth) + generation script
├─ tools/
│   ├─ jitter-bench/           # scheduler benchmark CLI
│   └─ gen-tokens/             # tokens.json → themes.css, tokens.ts, tokens.rs (sRGB)
├─ docs/                       # ARCHITECTURE.md, TASKS.md, bench/, checklists/
└─ .github/workflows/          # ci.yml, release.yml
```

**Dependency rule:** `ch-core` ← `ch-win` ← `ch-engine` ← `src-tauri`. The UI only talks to `src-tauri` through IPC. `ch-core` knows neither Windows nor Tauri, and it holds all the testable logic: patterns, the state machine and migrations.

---

## 3. Architecture diagram

```
┌──────────────────────────── Single process: ch-clicker.exe (PerMonitorV2) ────────────────────────────┐
│                                                                                                       │
│  WebView2 (React)                                     Rust                                            │
│  ┌──────────────┐  invoke (cmds)   ┌───────────────────────────────────────────────┐                  │
│  │ main  420×h  │ ───────────────▶ │ src-tauri: Command Router                     │                  │
│  │ variation    │ ◀─────────────── │  ├─ ProfileService ──▶ ch-storage (%APPDATA%) │                  │
│  │ hud          │   events         │  ├─ SettingsService ─▶ ch-storage             │                  │
│  │ overlay×N    │                  │  ├─ HotkeyService ◀─┐                          │                  │
│  └──────────────┘                  │  ├─ OverlayService ─┼─▶ ch-win::capture        │                  │
│        ▲                           │  ├─ WindowService ──┼─▶ ch-win::window         │                  │
│        │ 10 Hz aggregated          │  ├─ Tray / Notifications / Updater             │                  │
│        │                           │  └─ EngineController ──cmd channel──┐          │                  │
│  ┌─────┴──────────┐                └─────────────────────────────────────┼──────────┘                  │
│  │ Telemetry      │◀── SPSC ring (lateness) + atomics (clicks, state) ───┐│                             │
│  │ Emitter thread │                                                     ││                             │
│  └────────────────┘                                                     │▼                             │
│                                         ┌───────────────────────────────┴─────────────┐                │
│  ┌──────────────────────┐   HotkeyEvent │ Engine thread (TIME_CRITICAL + MMCSS)       │                │
│  │ Hotkey thread        │ ─────────────▶│  StateMachine (idle/countdown/running/      │                │
│  │  own message loop    │  + AtomicBool │                paused/stopping/error)       │                │
│  │  RegisterHotKey      │  EMERGENCY ──▶│  Scheduler (multi-timeline, QPC deadlines)  │                │
│  │  WH_KEYBOARD_LL      │               │   ├─ ClickTimeline (interval cycle)         │                │
│  │  WH_MOUSE_LL         │               │   ├─ MotionTimeline (wiggle | zigzag)       │                │
│  └──────────────────────┘               │   └─ StopTimeline (timer)                   │                │
│                                         │  HighResWaiter + hybrid spin                │                │
│                                         │  SelfWindowGuard (never clicks itself)      │                │
│                                         │        │                                    │                │
│                                         │        ▼                                    │                │
│                                         │  dyn InputBackend                           │                │
│                                         │   ├─ SendInputBackend (default)             │                │
│                                         │   └─ InterceptionBackend (experimental) ────┼─▶ kernel driver │
│                                         └─────────────────────────────────────────────┘                │
└───────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

### Threads

| Thread           | Responsibility                                                                                                       | Priority                                                                            |
| ---------------- | -------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| Main (Tauri/tao) | Event loop, windows, tray, IPC commands                                                                              | normal                                                                              |
| Engine           | Countdown, scheduler, injection. Receives commands through a channel + a Win32 `Event` that wakes it from the wait.  | `THREAD_PRIORITY_TIME_CRITICAL` + MMCSS "Games" via `AvSetMmThreadCharacteristicsW` |
| Hotkey           | Own message loop with `RegisterHotKey` (message-only hwnd) and LL hooks. Answers the hook in < 1 ms (only enqueues). | `ABOVE_NORMAL`                                                                      |
| Telemetry        | Every 100 ms reads the atomics and the ring buffer and emits aggregated Tauri events                                 | normal                                                                              |
| Window animator  | Animated resize of the main window (`SetWindowPos` every frame)                                                      | normal                                                                              |

The engine thread **never** calls Tauri or allocates on the hot path. It writes to atomics and a lock-free ring buffer. The emergency stop is an `AtomicBool` that the hook writes and the engine reads on every iteration, and the hook also signals the `Event` that wakes the wait. That guarantees the stop even when the command channel is congested.

---

## 4. Data model

Coordinates are always in **physical pixels of the virtual desktop** (they can be negative). Times are integer ms, except amplitude (float).

### Enums

```
Mode         = "click" | "wiggle" | "scan"
MouseButton  = "left" | "middle" | "right" | "x1" | "x2"
ClickKind    = "single" | "double"
PositionMode = "cursor" | "points"
WiggleSpeed  = 4 | 8 | 16                       (Hz)
ScanSpeed    = 600 | 1200 | 2400                (px/s)
ScanAction   = "move" | "move_click"
Locale       = "pt-BR" | "en-US" | "es-419"
ThemeId      = "grafite" | "blueprint" | "fosforo" | "bancada" | "papel" | "contraste"
CloseAction  = "minimize" | "quit"
InputBackend = "sendinput" | "interception"
HotkeyAction = "startStop" | "pauseResume" | "capturePoint" | "nextProfile" | "emergencyStop"
EngineState  = "idle" | "countdown" | "running" | "paused" | "stopping" | "error"
StopReason   = "user" | "timer" | "emergency" | "completed" | "error" | "profileSwitch"
```

### Hotkey

```json
{ "mods": ["ctrl", "shift"], "key": "P" }
```

`mods ⊆ {ctrl, alt, shift, win}`. `key` is the canonical name of a VK (`"F6"`, `"Escape"`, `"Right"`, `"P"`) or of a mouse button (`"MouseMiddle"`, `"MouseX1"`, `"MouseX2"`). The left and right buttons are not accepted as hotkeys.

### Profile (`schemaVersion: 1`)

Following **design decision 1**, the click settings (`click`) and the timer (`stopAfter`) live at profile level and are shared by every mode. Each mode's parameters live in `modes`, and all of them are kept when switching tabs.

```json
{
  "schemaVersion": 1,
  "id": "6f1c0e8a-3b6e-4c9a-9f3e-2a1d7c5b8e40",
  "name": "Resource farm",
  "mode": "click",
  "hotkey": { "mods": ["ctrl", "alt"], "key": "1" },
  "createdAt": "2026-10-04T20:00:00Z",
  "updatedAt": "2026-10-04T20:10:00Z",
  "click": {
    "button": "left",
    "kind": "single",
    "intervalMs": 100,
    "variation": { "enabled": true, "blocksMs": [1, 35, 70] }
  },
  "stopAfter": { "enabled": false, "seconds": 300 },
  "modes": {
    "click": {
      "position": "points",
      "points": [
        { "id": "p1", "x": 412, "y": 236, "delayMs": 0 },
        { "id": "p2", "x": 1520, "y": 400, "delayMs": 120 },
        { "id": "p3", "x": -840, "y": 610, "delayMs": 0 }
      ],
      "layoutFingerprint": "3mon:1920x1080@0,0;2560x1440@1920,-180;1680x1050@-1680,0"
    },
    "wiggle": { "amplitudePct": 0.05, "speedHz": 8, "clickWhileWiggling": false },
    "scan": {
      "area": { "x": 412, "y": 236, "w": 776, "h": 260 },
      "lineSpacingPx": 20,
      "speedPxS": 1200,
      "action": "move",
      "loop": true
    }
  }
}
```

**Validation** (in `ch-core`, applied on import, on update and on start):

- `intervalMs` from 1 to 60000. `blocksMs`: each from 1 to 60000, 1 to 32 blocks.
- `stopAfter.seconds` ≥ 30, in steps of 30 in the UI. The backend accepts any value ≥ 30.
- `amplitudePct` from 0 to 100, with up to 4 decimal places. `lineSpacingPx` from 8 to 130.
- `points`: 0 to 64, `delayMs` from 0 to 60000, coordinates within ±100000. Starting in `points` with an empty list returns `NO_POINTS`.
- `scan.area` null, or with `w,h` ≥ 16 px and ≤ 32768 px. Starting the scan without an area returns `NO_AREA`.
- `name`: 1 to 48 characters, no control characters, unique (case-insensitive). Duplicating turns the name into "Name (2)".

`layoutFingerprint` records the monitor layout at capture time. If, on start, the current layout differs and some point falls outside the virtual desktop, the start fails with `POINTS_OFFSCREEN` and the UI warns.

### Settings (`schemaVersion: 1`)

```json
{
  "schemaVersion": 1,
  "locale": "pt-BR",
  "theme": "grafite",
  "followSystemTheme": false,
  "autostart": false,
  "minimizeToTray": true,
  "showHud": true,
  "closeAction": "minimize",
  "countdownSec": 3,
  "inputBackend": "sendinput",
  "activeProfileId": "6f1c0e8a-3b6e-4c9a-9f3e-2a1d7c5b8e40",
  "profileOrder": ["6f1c0e8a-3b6e-4c9a-9f3e-2a1d7c5b8e40"],
  "hotkeys": {
    "startStop": { "mods": [], "key": "F6" },
    "pauseResume": { "mods": [], "key": "F7" },
    "capturePoint": { "mods": ["ctrl", "shift"], "key": "P" },
    "nextProfile": { "mods": ["ctrl", "alt"], "key": "Right" },
    "emergencyStop": { "mods": ["ctrl", "alt"], "key": "Escape" }
  },
  "hud": { "x": null, "y": null },
  "window": { "x": null, "y": null }
}
```

A disabled hotkey is `null` (shown as "—"). `emergencyStop` can never be `null`: the schema rejects it.

Fields owned by Rust (profile order, active profile, hotkeys, HUD and window positions, anti-cheat acknowledgement, HUD hint counter) are preserved when the UI sends a settings edit (`Settings::merge_user_edit`).

### Files

```
%APPDATA%\Chenoli\CH Clicker\
├─ settings.json
├─ profiles\{id}.json            # one file per profile (+ {id}.json.bak with the previous version): isolated corruption
├─ backups\                      # pre-migration copies (profile.vN.json), deleted profiles and quarantined corrupt files
%LOCALAPPDATA%\Chenoli\CH Clicker\
├─ cache\capabilities.json       # benchmark result (key: Windows build + CPU)
├─ cache\thumbs\{profileId}.png  # scan area thumbnail
└─ logs\ch-clicker.log           # tauri-plugin-log (log crate), 5 MB × 5 files
```

**`.chprofile` format** (import/export):

```json
{ "format": "chprofile", "schemaVersion": 1, "exportedBy": "CH. Clicker 0.1.0", "profile": { ... } }
```

On import the profile gets a new `id`. The `hotkey` is dropped if it conflicts, and the name is adjusted if it is taken.

---

## 5. Scheduler and state machine

### 5.1 States

```
                ┌───────────── start (F6, click, profile hotkey) ────────────────┐
                │                                                               ▼
           ┌────────┐   cancel (F6/Esc/emergency)     ┌───────────┐
           │  idle  │ ◀────────────────────────────── │ countdown │  countdownSec = 0 ⇒ skipped
           └────────┘                                 └───────────┘
             ▲   ▲  ▲                                       │ countdown ends
             │   │  │                                       ▼
             │   │  │   F7    ┌────────┐    F7     ┌───────────┐
             │   │  └──────── │ paused │ ◀───────▶ │  running  │
             │   │            └────────┘           └───────────┘
             │   │                 │ F6/emergency/timer   │ F6, timer, emergency, scan without loop done
             │   │                 ▼                      ▼
             │   │            ┌──────────────────────────────┐
             │   └─────────── │ stopping (releases buttons,  │
             │                │ returns the cursor in wiggle,│
             │                │ timeEndPeriod)               │
             │                └──────────────────────────────┘
             │  ack/start                       │ backend failure while running
          ┌───────┐ ◀───────────────────────────┘
          │ error │
          └───────┘
```

| From           | Event                                                           | To                                           | Effect                                                                                            |
| -------------- | --------------------------------------------------------------- | -------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| idle / error   | `start`                                                         | countdown (or running if `countdownSec = 0`) | Compiles a `RunPlan` from the profile snapshot. Validation failure → stays, emits `engine-error`. |
| countdown      | 100 ms tick                                                     | countdown                                    | emits `remainingMs`                                                                               |
| countdown      | countdown reaches 0                                             | running                                      | `timeBeginPeriod`/HR timer, sets `t0`                                                             |
| countdown      | `start`/`stop`/emergency                                        | idle                                         | `reason = user/emergency`                                                                         |
| running        | `pause`                                                         | paused                                       | freezes `elapsed` and `remaining`; wiggle returns to the origin                                   |
| paused         | `pause`                                                         | running                                      | shifts every deadline by the paused time                                                          |
| running/paused | `stop`, timer, emergency, scan done                             | stopping → idle                              | matching `reason`                                                                                 |
| running        | backend error (e.g. `SendInput` blocked by UIPI/secure desktop) | stopping → error                             | `engine-error {code}`                                                                             |
| error          | `start` or `ack`                                                | countdown / idle                             |                                                                                                   |

**Hotkey errors are not engine state.** They live in `hotkeyFaults[]` (design decision 8). The UI derives the main button's look:

```
buttonVisual =
  engineState == countdown         → Countdown
  engineState == running           → Running
  engineState == paused            → Paused (Running look, PAUSADO kicker, fg-2 color)
  engineState == error             → Error (engine code)
  idle && hotkeyFaults not empty   → Error "ATALHO EM CONFLITO" (starting by click still works)
  idle                             → Stopped
```

The `RunPlan` is an **immutable snapshot**. Profile changes during a run apply from the next start (see D8).

### 5.2 Multi-timeline scheduler

Each run has up to three timelines, and each exposes `next_deadline()` in absolute QPC ticks:

- **ClickTimeline**: intervals from the variation cycle (or fixed). In multi-point mode it adds the point's `delayMs` and moves to the point before clicking.
- **MotionTimeline**: wiggle or zigzag. Updates at a fixed rate (`MOTION_HZ = 250`), emitting a move only when the integer position changes.
- **StopTimeline**: a single deadline for the timer.

```
engine loop:
  if EMERGENCY.load(): go to stopping(emergency)
  drain the command channel (start/stop/pause/ack)
  t   = timeline with the earliest deadline
  now = qpc_now()
  rem = t.deadline - now
  if rem > SPIN_WINDOW (1.0 ms):
      WaitForMultipleObjects([hr_timer armed for (t.deadline - SPIN_WINDOW), cmd_event], timeout)
      continue (re-evaluate; a command may have arrived)
  else:
      spin with spin_loop_hint until now >= t.deadline   // at most ~1 ms of CPU per event
  lateness = qpc_now() - t.deadline
  stats.record(lateness)                                 // SPSC ring buffer, no allocation
  t.fire(backend)                                        // injection
  t.advance()                                            // deadline += next interval (absolute, no drift)
  tolerance = min(interval / 2, 5 ms)
  if t.deadline < now + interval - tolerance:            // the next click would come too soon (or is already late)
      stats.missed += deadlines already passed
      t.rebase(now)                                      // deadline = now + interval: no catch-up burst
```

- **Absolute deadlines** (`deadline_{n+1} = deadline_n + interval_n`): while the lateness stays within the tolerance, error does not accumulate and the average CPS stays exact.
- **Late clicks:** a late click fires once. Two consecutive clicks are never closer than `interval − tolerance`, with `tolerance = min(interval / 2, 5 ms)` (50% up to 10 ms, 5% at 100 ms). Past the tolerance the lost time is dropped instead of replayed, so a stall never turns into a burst; skipping time never skips a point of a multi-point sequence.
- **Waiter:** `CreateWaitableTimerExW(CREATE_WAITABLE_TIMER_HIGH_RESOLUTION)` (Win10 1803+). Fallback: `timeBeginPeriod(1)` + a regular waitable timer. `timeBeginPeriod` is only active between `running` and `stopping`, so the system is not kept at 1 ms while idle.
- **Hybrid spin:** the last 1.0 ms is spent spinning. The cost is up to one core during very short intervals. For intervals ≥ 20 ms the spin window drops to 0.3 ms.
- **Pause:** stores `paused_at` and, on resume, adds `now - paused_at` to every deadline.
- **Double click:** a single `SendInput` with 4 events (down/up/down/up). The pair counts as 1 tick and 2 clicks on the counter, and the displayed CPS is ×2, as the handoff specifies.
- **Atomic injection:** down and up in the same `SendInput`, so a button is never left pressed. With the Interception backend (separate events), `stopping` sends "up" for every button to be safe.

### 5.3 Jitter, capabilities and the infeasibility warning

- **Continuous measurement:** lateness histogram with logarithmic buckets from 1 µs to 1 s. The telemetry thread publishes `p50/p95/p99/max/missed` every second.
- **Capability benchmark** (`engine_run_benchmark`, run in the background on first launch and after a Windows update; ~3 s): runs the scheduler with a no-op action at 1, 2, 4, 8 and 16 ms. `minReliableIntervalMs` is the shortest interval with `p99 lateness < 25% of the interval` and at most 0.1% of the deadlines skipped. It is cached in `capabilities.json` and exposed through `engine.capabilities` (design decision 3). Until the benchmark finishes, the UI uses 4 ms.
- **Static warning (UI):** blocks or intervals below `minReliableIntervalMs` get the `--warn` dot and the screen 02.B callout.
- **Dynamic warning:** if `p95` stays above 50% of the interval for 2 s in a row, the engine emits `engine-degraded {reason: "timing"}` and the UI shows a warn callout on the panel. It clears after 5 normal seconds.
- **Honest limit:** the app can measure when the injection happened, not when the target app processed the event. Many apps read input once per frame. The diagnostics UI says so.

---

## 6. Algorithms

### 6.1 Interval variation cycle

```
intervals(cfg):
  if cfg.variation.enabled and blocksMs not empty:
      i = 0
      loop: yield blocksMs[i]; i = (i + 1) mod len
  else:
      loop: yield cfg.intervalMs

First click: right when entering running (t0). The next ones follow intervals().
Stats (screen 02.B): cycle = Σ blocks; average = cycle / n; effective (CPS) = 1000 / average × (double ? 2 : 1).
```

### 6.2 Multi click

```
pts = points (user order); k = 0
fire(): target = pts[k]
        if SelfWindowGuard.is_own(target): skipped++ ; else backend.move_abs(target); backend.click(button, kind)
        k = (k + 1) mod len
advance(): deadline += intervals.next() + pts[k].delayMs     // "The delay is added to the interval"
```

"Current cursor position" clicks wherever the cursor is, without moving it. A single fixed point is `points` mode with one point: "if the cursor is away from the configured position, it goes there at click time".

### 6.3 Absolute coordinates

```
V = virtual desktop (SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN)
nx = ceil((x - V.left) * 65536 / V.width)          // same for y with V.top / V.height
SendInput(MOUSEEVENTF_MOVE | ABSOLUTE | VIRTUALDESK | MOVE_NOCOALESCE, nx, ny)
```

- Windows converts back with `x = V.left + floor(nx * V.width / 65536)`. Rounding up on the way in is the exact inverse of that conversion for any width up to 65536 px. A property test in `ch-core::geometry` guarantees the round trip for random layouts (negative origin, 1 to 65536 px).
- Absolute moves are **not** affected by pointer acceleration ("Enhance pointer precision") or mouse speed. Relative moves are, so they are never used.
- `MOVE_NOCOALESCE` stops Windows from merging consecutive moves (important for wiggle and scan).
- After every move, `GetCursorPos` checks the pixel. A difference of up to 2 px is fixed with `SetCursorPos` and counted as a correction (diagnostics). A larger difference means the user moved the mouse, and the backend does not fight them. The integration test `absolute_moves_land_on_the_exact_pixel_on_every_monitor` requires zero corrections.
- The layout is recomputed on `WM_DISPLAYCHANGE` and `WM_DPICHANGED`. If it changes during a run, the engine pauses and emits `engine-degraded {reason: "displayChanged"}` (1 s polling watcher in `display_watch.rs`, because message-only windows do not receive the `WM_DISPLAYCHANGE` broadcast); during the countdown, the run is cancelled.

### 6.4 Mouse wiggle

**"Screen width"** (design decision 9): the width in **physical pixels of the monitor the cursor is on at start** (`MonitorFromPoint` → `rcMonitor`). That is what the UI shows ("≈ 0,96 px em 1920"). Using the virtual desktop would make 1% mean different things depending on the setup, and the movement would cross monitors with different DPI.

**Amplitude** = peak-to-peak excursion: `A = amplitudePct / 100 × monitorWidthPx`, so 100% is the full width.

**Subpixel: 1 px minimum (decision).** The cursor only exists on whole pixels. Accumulating subpixels would turn 0.05% into a 1 px step every ~20 cycles: the chosen frequency (4/8/16 Hz) would stop holding and the movement would look stuck. What users want from values < 1 px is "the smallest noticeable movement at the chosen frequency", so `A_eff = max(1, round(A))`. The UI shows "≈ 0,96 px → 1 px efetivo" when `A < 1`.

```
start:  origin = cursor; mon = monitor(origin)
        L = origin.x - floor(A_eff / 2); R = L + A_eff
        shift [L, R] inside mon (clamp without crossing monitors)
tick at MOTION_HZ (250 Hz):
        phase = frac(t × speedHz)                      // one cycle = L→R→L
        x = L + round(A_eff × (1 - cos(2π × phase)) / 2)
        if x != last_x: backend.move_abs(x, origin.y)
pause/stop: backend.move_abs(origin)                    // returns exactly to the origin
```

- **No drift:** every position is computed absolutely from `origin`. Nothing accumulates.
- With `A_eff = 1` the curve becomes a 1 px square wave, which is the intended behavior.
- **User intervention:** if `GetCursorPos` differs by > 4 px from the last commanded position, `origin` becomes the user's position (re-anchoring). The user never "fights" the cursor.
- **Click while wiggling:** the ClickTimeline runs in parallel with the profile's click settings and clicks at the current position.

### 6.5 Screen scan (zigzag)

```
rows = floor(area.h / spacing) + 1
y_i  = area.top + i × spacing                          (i = 0..rows-1, y_i ≤ area.bottom)
polyline P: (left, y0) → (right, y0) → (right, y1) → (left, y1) → (left, y2) → …
if loop: append a return segment from the last vertex to (left, y0)
length L = rows × area.w + (rows - 1) × spacing [+ return]
cycle = L / speedPxS                                   (badge "13 linhas · ciclo 9,2 s")

tick at MOTION_HZ:
  s = (t × speedPxS) mod L       (with loop)  | min(t × speedPxS, L) (without loop)
  p = point_on_polyline(P, s)    (search by precomputed cumulative length)
  if round(p) != last: backend.move_abs(round(p))
  without loop and s == L → stop(completed)
```

- **Action:** "Move only" just moves. "Move and click" turns on the ClickTimeline with the rate inherited from the Click tab (design decision 1), clicking at the current point of the path. **Requirement met:** the mode moves and, optionally, clicks at intervals.
- At 2400 px/s and 250 Hz the step is 9.6 px. That is enough for the path to look continuous without flooding the input queue.
- The loop return travels a straight segment back to the start at the same speed, with no jump. The `cycle` formula includes that segment, and the UI badge uses the same formula.

### 6.6 "Never click the app's own window" guard

Before every click: `WindowFromPoint(target)` → `GetAncestor(GA_ROOT)` → `GetWindowThreadProcessId`. If the PID is ours (main window, variation editor or overlays), the click is **skipped**, `skippedSelf++`, and telemetry reports it. The HUD has `WS_EX_TRANSPARENT | WS_EX_LAYERED`, so hit-testing passes through it and the click lands on the window below, which is the intended behavior.

---

## 7. IPC contracts

Every command returns `Result<T, AppError>`, with `AppError = { code: string, params?: object }`. The UI translates `code` through i18n (`errors.<code>`). Event names use the domain prefix.

### 7.1 Commands (UI → Rust)

| Command                                                   | Input                                                   | Output                                                                                                                          | Notes                                                                                    |
| --------------------------------------------------------- | ------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| `app_bootstrap`                                           | —                                                       | `{ version, settings, profiles[], activeProfileId, engine: EngineSnapshot, capabilities, hotkeys: HotkeyStatus[], monitors[] }` | One call per window at boot                                                              |
| `settings_update`                                         | `Partial<Settings>`                                     | `Settings`                                                                                                                      | Persisted with a 300 ms debounce. Emits `settings-changed`.                              |
| `profile_create`                                          | `{ name?, fromTemplate? }`                              | `Profile`                                                                                                                       |                                                                                          |
| `profile_update`                                          | `{ id, patch, baseUpdatedAt }`                          | `Profile`                                                                                                                       | Revision conflict (another window edited it) → `STALE_PROFILE`                           |
| `profile_duplicate` / `profile_rename` / `profile_delete` | `{ id, name? }`                                         | `Profile` / `()`                                                                                                                | The last active profile cannot be deleted while it runs                                  |
| `profile_reorder`                                         | `{ ids[] }`                                             | `()`                                                                                                                            |                                                                                          |
| `profile_set_active`                                      | `{ id }`                                                | `()`                                                                                                                            | Running → stops first (`reason = profileSwitch`)                                         |
| `profile_import`                                          | `{ path }`                                              | `{ profile, warnings[] }`                                                                                                       | Notes: hotkey dropped, name adjusted, migrated from vN                                   |
| `profile_export`                                          | `{ id, path }`                                          | `()`                                                                                                                            | File dialog through the dialog plugin                                                    |
| `engine_start`                                            | `{ profileId? }`                                        | `()`                                                                                                                            | Validation errors: `NO_POINTS`, `NO_AREA`, `POINTS_OFFSCREEN`                            |
| `engine_stop`                                             | —                                                       | `()`                                                                                                                            |                                                                                          |
| `engine_toggle_pause`                                     | —                                                       | `()`                                                                                                                            |                                                                                          |
| `engine_snapshot`                                         | —                                                       | `EngineSnapshot`                                                                                                                |                                                                                          |
| `engine_capabilities`                                     | —                                                       | `Capabilities`                                                                                                                  |                                                                                          |
| `engine_run_benchmark`                                    | —                                                       | `Capabilities`                                                                                                                  | Asynchronous, ~3 s                                                                       |
| `engine_ack_error`                                        | —                                                       | `()`                                                                                                                            |                                                                                          |
| `hotkey_begin_record`                                     | `{ action }`                                            | `()`                                                                                                                            | Suspends the global hotkeys and turns on the recording LL hook                           |
| `hotkey_cancel_record`                                    | —                                                       | `()`                                                                                                                            |                                                                                          |
| `hotkey_set`                                              | `{ action \| profileId, combo \| null, replace: bool }` | `HotkeySetResult = ok \| conflict{withAction\|withProfileId} \| registration_failed{reason}`                                    | `replace = true` takes the hotkey from the other one (it becomes "—")                    |
| `hotkey_reset_defaults`                                   | —                                                       | `HotkeyStatus[]`                                                                                                                |                                                                                          |
| `hotkey_retry`                                            | `{ action }`                                            | `HotkeyStatus`                                                                                                                  | "Try again" action of notification 09.B                                                  |
| `overlay_pick_area`                                       | `{ profileId }`                                         | `Rect \| null`                                                                                                                  | Opens 03.A. Resolves on confirm or cancel.                                               |
| `overlay_pick_points`                                     | `{ profileId }`                                         | `Point[]` (added)                                                                                                               | Opens 03.B                                                                               |
| `window_set_content_height`                               | `{ px }`                                                | `()`                                                                                                                            | Design decision 7: Rust animates the height from 640 to 900 px, limited to the work area |
| `window_minimize` / `window_close`                        | —                                                       | `()`                                                                                                                            | `close` applies `closeAction`                                                            |
| `window_open_variation`                                   | `{ profileId }`                                         | `()`                                                                                                                            | Window 02.B (560×640)                                                                    |
| `hud_set_visible` / `hud_save_position`                   | `{ visible }` / `{ x, y }`                              | `()`                                                                                                                            |                                                                                          |
| `app_open_url`                                            | `{ which: "support" \| "repo" }`                        | `()`                                                                                                                            | URL defined in Rust (design decision 10)                                                 |
| `app_set_autostart`                                       | `{ enabled }`                                           | `()`                                                                                                                            |                                                                                          |
| `updater_check` / `updater_install`                       | —                                                       | `UpdateInfo \| null` / `()`                                                                                                     |                                                                                          |

### 7.2 Events (Rust → UI)

tauri-specta names each event in kebab-case from its type (`EngineTick` → `engine-tick`). In the implementation, transitions go out as `engine-status` (`EngineStatus`), telemetry as `engine-tick`, data changes as `settings-changed` and `profiles-changed`, the benchmark result as `capabilities`, and hotkeys as `hotkey-status-changed`, `hotkey-recording` and `hotkey-action-failed`.

| Event                 | Payload                                                                                               | Rate                              | Feeds                                                                                                             |
| --------------------- | ----------------------------------------------------------------------------------------------------- | --------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| `engine-state`        | `{ state, prevState, reason?: StopReason, profileId, mode, countdownTotalMs?, startedAt? }`           | on transitions                    | main button (screen 10), footer, HUD, tray                                                                        |
| `engine-tick`         | `{ countdownRemainingMs?, clicks, elapsedMs, remainingMs?, skippedSelf }`                             | 10 Hz in countdown/running/paused | countdown in tenths ("INICIANDO EM 2,4 S", rounded up on the button), "CLIQUES n", "TEMPO mm:ss.d", "RESTA 04:32" |
| `engine-jitter`       | `{ intervalMs, p50Us, p95Us, p99Us, maxUs, missed }`                                                  | 1 Hz while running                | diagnostics                                                                                                       |
| `engine-degraded`     | `{ reason: "timing" \| "display_changed" \| "uipi_blocked", detail? }`                                | when it happens                   | panel callout                                                                                                     |
| `engine-error`        | `AppError`                                                                                            | when it happens                   | Error state                                                                                                       |
| `engine-capabilities` | `Capabilities { minReliableIntervalMs, timerKind: "high_res" \| "legacy", measuredAt }`               | after the benchmark               | 4 ms warning (02.B)                                                                                               |
| `hotkey-status`       | `HotkeyStatus[] = { action \| profileId, combo, status: "ok" \| "failed" \| "disabled", errorCode? }` | on changes                        | screen 06, Error state, notification 09.B                                                                         |
| `hotkey-recording`    | `{ action, mods[], key?, done: bool, cancelled: bool }`                                               | on every key while recording      | "Press the keys…" capture field                                                                                   |
| `hotkey-triggered`    | `{ action \| profileId }`                                                                             | when fired                        | visual feedback (optional)                                                                                        |
| `profiles-changed`    | `{ profiles[], activeProfileId }`                                                                     | on changes                        | syncs main, HUD and tray                                                                                          |
| `settings-changed`    | `Settings`                                                                                            | on changes                        | theme, language in every window                                                                                   |
| `display-changed`     | `Monitor[] = { id, name, rect, workArea, scale, primary }`                                            | `WM_DISPLAYCHANGE`                | minimap, wiggle scale                                                                                             |
| `hud-hover`           | `{ over: bool }`                                                                                      | on changes                        | fade the HUD to 40%                                                                                               |
| `updater-available`   | `UpdateInfo`                                                                                          | after a check                     | footer of 08                                                                                                      |

**Countdown in tenths:** the engine emits `countdownRemainingMs` every 100 ms. The UI shows `ceil(ms/1000)` on the button and `(ms/1000).toFixed(1)`, formatted for the locale, in the footer.

---

## 8. Design tokens, themes and i18n

### 8.1 Tokens

- **Source of truth:** `design/tokens.json`, a copy of the handoff tokens. Names never change and no new token is created.
- `tools/gen-tokens` generates:
  - `ui/src/styles/themes.css`, equivalent to `design/themes.reference.css` (a versioned copy of the handoff CSS), with one fix: derived values are also declared under `[data-theme]`, otherwise a nested theme (Screen 07 thumbnails) would inherit the derived values computed on `:root`. Base values per `[data-theme]` + the derived `--divider`, `--border-strong`, `--seg-active`, `--tint-*`, `--edge-*`, radii, shadows, easing.
  - `ui/src/styles/tokens.ts`: typed list of the 6 themes with `id/name/tag/description/tokens`, used by the screen 07 thumbnails, which draw each card with that theme's own tokens.
  - `src-tauri/src/tokens.rs`: colors converted from `oklch` to sRGB hex for native drawing (per-state tray icons).
  - CI fails if the generated files drift from the JSON.
- **Tailwind v4:** `@theme` maps the variables (`--color-bg: var(--bg)`, `--color-fg-3: var(--fg-3)`, `--color-accent: var(--accent)`…) and the radii (`--radius-card`…), so classes read `bg-bg-2 text-fg-3 border-border rounded-control`.
- **One-off mixes from the handoff** (`fg 12%`, `accent 22% over bg-2`, `warn 6%`, `danger 9%` and so on) **do not become new tokens.** They stay as local `color-mix(in oklch, var(--x) N%, …)` in the component, through a Tailwind arbitrary utility or component CSS.
- **Prototype literals** to convert: the HUD background `rgba(20,20,18,.95)` becomes `color-mix(in oklch, var(--bg) 95%, transparent)`, and Grafite hex values are replaced by tokens. Only the Win11 close-button red `#C42B1C` stays literal.
- `color-mix`/`oklch` need Chromium 111+. Evergreen WebView2 meets that, and the installer guarantees WebView2 (bootstrapper).

### 8.2 Themes

- `data-theme` on the `<html>` of **every** window (main, variation, hud, overlays), applied at bootstrap and on `settings-changed`, before the first paint to avoid flicker.
- **Follow Windows:** reads `HKCU\…\Personalize\AppsUseLightTheme` and listens for `WM_SETTINGCHANGE("ImmersiveColorSet")`. Light → Papel, dark → Grafite.
- Contrast (AAA): focus is always visible (`outline 2px var(--accent)`) in every theme. In that theme, borders use its own `--border` (45%).

### 8.3 i18n

- Languages: **pt-BR** (default), **en-US** and **es-419** (Latin American Spanish, also used for the other Spanish variants; added at the user's request on 2026-10-05).
- **i18next** + ICU with one `common` catalog per language in `ui/src/i18n/locales/{locale}/common.json`, grouped by feature in the keys (`click.*`, `profiles.*`, `errors.*`…). One catalog per language proved enough for the size of the app.
- Typed keys (TS resource declaration), so a missing key is a compile error. The `ui/src/i18n/locales.test.ts` test (runs in CI with Vitest) requires the same keys and the same ICU placeholders in all three languages and no empty strings.
- Numbers through the active locale's `Intl.NumberFormat` ("10,0" in pt-BR, "10.0" in en-US and es-419). The `mm:ss.d` and `hh:mm:ss` clocks follow the handoff and are the same in every language.
- Text drawn by Rust (window titles, file type in dialogs, tray menu, notifications) lives in `src-tauri/src/native_text.rs`, one table per language covered by a test.
- Initial locale: the language picked in the installer, when there is one; otherwise `GetUserDefaultUILanguage` (Portuguese → pt-BR, Spanish → es-419, anything else → en-US). The installer writes `HKCU\Software\Chenoli\CH Clicker\PendingLocale` on interactive installs only; the app always deletes it on launch but uses it only when it creates the settings (first run), so reinstalling over existing data and passive updates never override the user's choice. Live switch through the Screen 08 segmented control; every window applies the settings language at boot and on each `settings-changed`. Adding a language = a catalog folder + a `Locale` enum variant + native texts + an NSIS language.
- **Every UI string goes through i18n from phase 0**, so there is no rework.

---

## 9. Test strategy

### 9.1 Unit (`ch-core`, run on any OS in CI)

- **Interval cycle:** order, wrap, variation off, stats (cycle/average/CPS) and double-click CPS.
- **Wiggle:** with `proptest`, positions always within `[L, R]` inside the monitor. After N full cycles the position returns exactly to the origin. The `A < 1 → 1 px` rule holds, the frequency is kept, re-anchoring works and the edge clamp works.
- **Zigzag:** number of rows, vertices, length, point by length (monotonic), constant speed (equidistant samples), loop and end without loop.
- **Normalization:** coordinate round trip for layouts with a negative origin, three monitors and mixed scales.
- **State machine:** the full transition table, including ignored invalid events.
- **Scheduler on `FakeClock`** (virtual time): absolute deadlines with no drift over 10⁶ events, SKIP policy, pause/resume shifting deadlines, timer, timeline merge.
- **Schema:** migrations from per-version fixtures, rejection of future versions and invalid JSON, validation rules, `.chprofile` import (new id, duplicate name, conflicting hotkey).
- **Hotkeys:** internal conflict detection (action × action, action × profile), combo normalization, recording rules (a lone modifier does not confirm, Esc without modifiers cancels).

### 9.2 Windows integration (`ch-win`, `windows-latest` runner)

- `SendInputBackend` against a test window created in-process: counts `WM_LBUTTONDOWN/UP`, double click, X1/X2.
- `HighResWaiter`: checks that the HR timer is available and measures its resolution.
- Storage: atomic write under simulated failure (process killed between write and rename), `.bak` rotation.
- Tests that need an interactive session (real cursor, hooks) are `#[ignore]` and run locally with `cargo test -- --ignored`.

### 9.3 Jitter benchmark (`tools/jitter-bench`)

- Intervals of 1, 2, 4, 8 and 16 ms × 10 s, in three scenarios: idle, CPU at 100% on every core (stress) and on battery (power saver).
- Output: Markdown table + CSV with p50/p95/p99/max/missed, saved to `docs/bench/{date}-{machine}.md`.
- Also measures the cost of `SendInput` (µs per call).
- It is the input for confirming or adjusting the provisional 4 ms limit.

### 9.4 UI

- **Vitest + Testing Library:** primitives (Segmented, Stepper with step/limits, BigNumberField with comma/dot, Toggle), formatters (CPS, time), validation, store (event handling), hotkey recording flow with mocked IPC (`@tauri-apps/api/mocks`).
- **Visual parity (Playwright):** the `dev/gallery` route renders every screen and every state from the handoff (01.A–C, 02.A–B, 03.A–B, 04.A–C, 05.A–B, 06 in 3 states, 07, 08, 09.A–B, 10 in 4 states) at a fixed size. References are captured from the HTML prototype with the same fonts, by anchor (`#janela`, `#pontos`…). The comparison uses `toHaveScreenshot` with a low tolerance. Each screen has its own test, and that test closes the "visual parity" acceptance criterion.

### 9.5 Manual checklist (`docs/checklists/v0.1-manual.md`)

1. One monitor at 100%, 125%, 150% and 200%: captured points land on the exact pixel (check with the magnifier and `GetCursorPos`).
2. Two monitors with different scales (100% + 150%), secondary to the left (negative coordinates) and above (negative Y).
3. Three monitors in an L shape. Points on all of them. The minimap shows the right monitor.
4. Area overlay on each monitor: HUD dimensions in physical pixels and a correct thumbnail.
5. Wiggle: width comes from the cursor's monitor, never crosses into the neighbor at the edge, returns to the origin.
6. Scan near the edges of each monitor.
7. Change the scale or disconnect a monitor while running → pause + warning. Start with points outside the layout → `POINTS_OFFSCREEN`.
8. Main window dragged between monitors with different DPI: sharpness, maximum height limited to the work area.
9. HUD: top-right corner of the primary monitor, click-through, fades on hover, drag (Ctrl + drag, D5).
10. Elevated (admin) target app with a non-elevated clicker → `uipi_blocked` detected and reported.
11. Screen lock/UAC while running → stops with an error, no stuck button.
12. Emergency stop with the target window focused, in a full-screen game and with the app unfocused.

---

## 10. Security, risks and decisions

### 10.1 Safe use

- **Emergency stop** (`Ctrl Alt Esc`, cannot be disabled): registered through **two** independent paths, `RegisterHotKey` and `WH_KEYBOARD_LL`. Both write the `AtomicBool` the engine reads on every iteration. A watchdog reinstalls the hook if Windows removes it on timeout (`LowLevelHooksTimeout`). If `RegisterHotKey` fails, the hook still guarantees the stop and the UI warns.
- **Countdown** before starting (0–10 s, 3 s by default), visible on the button, the footer, the HUD and the tray.
- **Never clicks its own window** (section 6.6).
- **Own events ignored by the hooks:** the mouse hook drops events with `LLMHF_INJECTED`, so an X1 click from the engine does not fire an X1 hotkey. With the Interception backend, which does not mark events as injected, the app **forbids** using the button the profile clicks as a hotkey.
- **Anti-cheat warning** fixed on screen 08 (handoff text) and on the first start (one-time dialog with "Got it").
- The app does not hide its activity: no detection evasion, a process with its real name and `SendInput` events marked as injected.

### 10.2 Input backends

|              | SendInput (default)                                                                | Interception (experimental)                                                                                                                                                               |
| ------------ | ---------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Level        | User mode, Windows input queue                                                     | Kernel filter driver (keyboard/mouse)                                                                                                                                                     |
| Marking      | `LLMHF_INJECTED`/`LLKHF_INJECTED` visible to LL hooks                              | Indistinguishable from a physical device for hooks                                                                                                                                        |
| UIPI         | Cannot inject into higher-integrity windows (elevated apps) and **fails silently** | Not subject to UIPI                                                                                                                                                                       |
| Installation | None                                                                               | Driver installer with admin + **reboot**                                                                                                                                                  |
| Risks        | Low                                                                                | Old third-party driver, reports of instability with some devices, detected and blocked by modern anti-cheats (the driver's presence is the signal), LGPL/non-commercial license to verify |
| Fallback     | —                                                                                  | If the driver does not answer on start, falls back to SendInput and warns                                                                                                                 |

**Interception flow (v0.2, see D10):** Settings → Advanced → "Input backend". A dialog explains the risks and the user confirms explicitly. The app opens the driver's official page: the installer is not redistributed until the license is validated. After the reboot, the app detects the driver (`interception.dll` + device). Compile-time feature flag `interception`, absent from the v0.1 build.

True "hardware-level clicking" (a physical HID device, e.g. a microcontroller) is out of scope and only mentioned as an alternative.

### 10.3 Known technical risks

| Risk                                                                                    | Mitigation                                                                                                                                                                                                                   |
| --------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Animated window resize through `SetWindowPos` with WebView2 may flicker                 | Animator in Rust, top-anchored layout and a `--bg` background on the host. Fallback: instant resize (flag).                                                                                                                  |
| A single overlay covering mixed-DPI monitors renders with one scale factor              | **One overlay window per monitor** (a deliberate departure from the handoff's "one window"), coordinates converted with `monitor.rect + css × scale`                                                                         |
| The overlay needs the keyboard (Esc, arrows, Enter) but the requirements say "no focus" | The overlay takes focus: it is modal and the engine is forced idle during selection. The HUD stays `NOACTIVATE`.                                                                                                             |
| The magnifier needs the screen pixels                                                   | Per-monitor GDI `BitBlt` capture when the overlay opens (frozen screen, like the Snipping Tool), served to the WebView through a custom `chcap://` protocol as BMP, no base64 over IPC. The magnifier reads from the canvas. |
| Click-through HUD × "draggable"                                                         | See D5                                                                                                                                                                                                                       |
| Hover on the click-through HUD (no mouse events)                                        | `GetCursorPos` polling in Rust → `hud-hover`                                                                                                                                                                                 |
| Native notifications with actions need a registered AUMID                               | The NSIS installer registers the shortcut with the AUMID. Without it, falls back to a simple notification + visible state.                                                                                                   |
| SmartScreen on a new binary                                                             | Authenticode signing + accumulated reputation. Since 2024, EV no longer grants instant reputation.                                                                                                                           |
| Spin-wait uses CPU                                                                      | Short spin window (1 ms), only while `running`, reduced for long intervals                                                                                                                                                   |
| LL hook removed on timeout                                                              | The callback only enqueues, and a watchdog reinstalls it                                                                                                                                                                     |

### 10.4 Distribution

- **Installer:** Tauri NSIS (per-user, no admin) with the WebView2 bootstrapper and a language selector (pt-BR, en-US, es-419) whose choice becomes the app's initial language. Optional MSI for those who prefer it.
- **Signing:** Authenticode in CI (see D9). **Updater:** `tauri-plugin-updater` with a minisign key (GitHub secret), `latest.json` manifest on GitHub Releases. Manual check through "Check for updates" and an automatic, silent check every 24 h.
- **Autostart:** `tauri-plugin-autostart` (HKCU Run key) with `--minimized`, which opens in the tray. On launch the Run entry is brought in line with `settings.autostart` (the uninstaller removes it even when the data is kept).
- **Single instance:** `tauri-plugin-single-instance`. A second launch focuses the existing window.
- **CI (GitHub Actions):** `ci.yml` on PRs (fmt, clippy `-D warnings`, `cargo test`, eslint, tsc, vitest, Playwright visual, token/i18n checks, `cargo audit`/`pnpm audit`). `release.yml` on `v*` tags (build, signing, upload of the installer + `latest.json` to Releases).

### 10.5 Decisions

Made on 2026-10-04 based on common industry, design and usability practice. Any of them can be revisited.

| #   | Topic                                   | Decision                                                                                                                                                                                                                                                      | Rationale                                                                                                                                                                                                                                                                               |
| --- | --------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D1  | X1/X2 buttons                           | The BUTTON segmented control gets a compact 4th item, "Side ▾", which opens a popover with "X1 · Back" and "X2 · Forward". The item shows "X1"/"X2" when selected.                                                                                            | Keeps the 3 most used buttons one click away and the prototype's visual parity. Side buttons are one click further, without cluttering the row.                                                                                                                                         |
| D2  | Configurable countdown                  | A new 56 px row on screen 08, "Countdown before starting", with a 0 to 10 s stepper (step 1), between "Show HUD while running" and "On window close".                                                                                                         | It is a global preference, like its neighbors. Follows the screen's row pattern.                                                                                                                                                                                                        |
| D3  | Diagnostics                             | A collapsible "Advanced" section at the end of screen 08: measured minimum reliable interval, last p95/p99 and a "Measure again" button. v0.1 has no backend picker (see D10).                                                                                | Technical information is available without weighing on the screen for those who do not need it (progressive disclosure).                                                                                                                                                                |
| D4  | Per-profile hotkey                      | A "Set hotkey…" item in the screen 05 context menu, above "Duplicate", which opens the capture field inline on the card, with the same conflict rules as screen 06. Pressing a profile's hotkey activates the profile and starts; pressing again stops.       | Reuses the existing component and rules. "One hotkey = one profile ready to run" is the use that justifies per-profile hotkeys.                                                                                                                                                         |
| D5  | Click-through × draggable HUD           | Always click-through. With **Ctrl** held and the cursor over the HUD, it becomes interactive and can be dragged. The position is saved. A "Hold Ctrl to move" hint shows the first 3 times the HUD appears. The tray's "Show HUD" item gets "Reset position". | The engine must never click the HUD by accident: safety first. A pattern similar to game overlays. The hint solves discoverability.                                                                                                                                                     |
| D6  | Point capture                           | Continuous: each click adds a point (the label advances "05", "06"…), Enter or Esc ends while keeping the added points, and Ctrl+Z undoes the last one.                                                                                                       | Building 4 or more points would otherwise mean opening the overlay several times. Esc keeps the work, and Ctrl+Z covers a wrong click.                                                                                                                                                  |
| D7  | Area across monitors                    | The area is restricted to the monitor where the drag started.                                                                                                                                                                                                 | Avoids mixed-DPI ambiguity and corners outside the desktop. Common capture tools behave the same way.                                                                                                                                                                                   |
| D8  | Editing while running                   | Snapshot at start. While running, the controls stay editable and the panel shows a "Changes apply on restart" line.                                                                                                                                           | Predictable: what is running is what was started. The UI does not lock, and the effect of an edit is clear.                                                                                                                                                                             |
| D9  | Code signing                            | Unsigned v0.1.0, then free signing through the **SignPath Foundation** for open-source projects (see `docs/release.md`). Paid alternative: an OV certificate. Optional: Microsoft Store (MSIX signed by Microsoft).                                           | Paid certificates are out of budget for an open-source project. **Signing matters for a broad public release:** Windows 11 Smart App Control blocks executables without a signature from a recognized authority, so without it the app does not open for users who have the feature on. |
| D10 | Interception driver                     | **Out of v0.1**, planned for v0.2. The `InputBackend` abstraction is ready for it. When it lands: no redistribution of the driver, a link to the official installer, a risk warning and explicit opt-in.                                                      | A third-party kernel driver carries real risk (instability, anti-cheat bans, licensing) and is not needed for the product's core value.                                                                                                                                                 |
| D11 | Interval limit                          | 4 ms until the benchmark runs. After that, the `minReliableIntervalMs` measured on each machine applies.                                                                                                                                                      | Honest about the real hardware and keeps design decision 3.                                                                                                                                                                                                                             |
| D12 | Next profile while running              | Stops the run, switches profiles and stays stopped. The HUD/tray show the new profile.                                                                                                                                                                        | Never start automation with a configuration the user has not checked.                                                                                                                                                                                                                   |
| D13 | Variation editor                        | Its own 560×640 window, a child of the main window, modal to it and centered over it.                                                                                                                                                                         | Respects the design's measurements without distorting the fixed-width main window.                                                                                                                                                                                                      |
| D14 | Corners on Windows 10                   | Win11 uses DWM's native rounded corners. Win10 keeps square corners and the native shadow.                                                                                                                                                                    | A transparent window just for rounding would cost performance and the native shadow. That is how native apps behave on Win10.                                                                                                                                                           |
| D15 | External links                          | Support: `https://ko-fi.com/chenoli` ("Apoiar ↗" button on screen 08 and the tray item). Public repository: `https://github.com/chenoli/ch-auto-clicker`.                                                                                                     | Set by the author.                                                                                                                                                                                                                                                                      |
| D16 | User moves the mouse during wiggle/scan | Wiggle starts oscillating where the user left the cursor. Scan and multi-point carry on, because they reposition the cursor every step. Stopping is still one key away (F6/emergency).                                                                        | Does not fight the user in the mode where that would be annoying, and does not interrupt path automation.                                                                                                                                                                               |
| D17 | Main button with a conflicting hotkey   | Follows Screen 10 ("ATALHO EM CONFLITO / Revisar", struck-through key, "F6 INDISPONÍVEL" footer), and clicking opens Hotkeys. Right below, a "Start without hotkey" link keeps starting by click, as the handoff asks.                                        | The handoff asks for both things on the same button; splitting the actions makes both explicit.                                                                                                                                                                                         |
