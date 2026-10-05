# CH. Clicker — Arquitetura (v0.1)

> Fontes: `refs/ARCHITECTURE_PROMPT.md` (requisitos), `refs/README.md` + `refs/design/CH Clicker.dc.html` (interface), `refs/themes.css` + `refs/tokens.json` (tokens).
> Este documento não contém código de implementação. Pseudocódigo aparece só onde o algoritmo precisa ficar inequívoco.

---

## 1. Stack final

| Camada             | Escolha                                                                                   | Por quê                                                                                                                                        |
| ------------------ | ----------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| Shell da aplicação | **Tauri 2**                                                                               | Binário pequeno, WebView2 do sistema, bundler NSIS/MSI, plugins oficiais de updater, autostart, notificação e single-instance.                 |
| Núcleo nativo      | **Rust (stable) + crate `windows` (windows-rs)**                                          | Acesso direto a `SendInput`, timers de alta resolução, `RegisterHotKey`, hooks LL, DPI, GDI. Sem GC, então a thread do motor não sofre pausas. |
| UI                 | **React 19 + TypeScript 6 + Vite 8 + Tailwind CSS v4**                                    | Reaproveita o design system web. Os tokens viram variáveis CSS (`themes.css`) mapeadas no `@theme` do Tailwind.                                |
| Estado da UI       | **Zustand** (um store espelho por janela)                                                 | Leve, sem boilerplate. O Rust é a fonte da verdade, e o store só espelha o que chega por eventos.                                              |
| Bindings de IPC    | **tauri-specta**                                                                          | Gera tipos TS a partir dos comandos e eventos Rust, então o contrato da seção 7 fica checado em compilação dos dois lados.                     |
| i18n               | **i18next + react-i18next + i18next-icu**                                                 | Plurais e números ICU. O Rust lê o mesmo JSON para tray e notificações.                                                                        |
| Fontes             | **@fontsource/syne** (800) e **@fontsource/space-grotesk** (400–700)                      | Empacotadas localmente, como o README exige.                                                                                                   |
| Testes             | `cargo test` + `proptest`, **Vitest** + Testing Library, **Playwright** (paridade visual) | Ver seção 9.                                                                                                                                   |

### Alternativas descartadas

| Critério                      | Tauri 2 (Rust + WebView2)                                     | C# WinUI 3 / WPF                                                                                                                 | Electron                                                                                         |
| ----------------------------- | ------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| Precisão de timing            | Ótima: thread nativa dedicada, sem GC.                        | Boa, mas o GC do .NET pode pausar a thread do motor (mitigável com `GCSettings.LatencyMode` e alocação zero, o que dá trabalho). | O motor teria que ser um addon nativo (N-API) de qualquer forma, então vira Rust/C++ + Electron. |
| Tamanho do instalador         | ~5–10 MB                                                      | ~1 MB dependente de framework, 60–150 MB self-contained (WinUI 3 ainda exige o Windows App SDK).                                 | 80–150 MB                                                                                        |
| Memória em uso                | ~50–90 MB (WebView2 compartilhado)                            | ~40–80 MB                                                                                                                        | 150–300 MB                                                                                       |
| Temas e i18n                  | Reaproveita CSS/tokens do site.                               | XAML ResourceDictionaries: bom, mas é reescrever o design system.                                                                | Igual ao Tauri.                                                                                  |
| Paridade com o protótipo HTML | Direta (mesmo CSS: `color-mix`, `oklch`).                     | Recriar tudo em XAML. `oklch`/`color-mix` não existem e precisam ser pré-calculados.                                             | Direta.                                                                                          |
| Empacotamento e assinatura    | NSIS/MSI nativos, updater assinado (minisign) + Authenticode. | MSIX/ClickOnce, bom.                                                                                                             | electron-builder, bom.                                                                           |

**Decisão:** mantenho a recomendação **Tauri 2**. O ponto que decide é este: o timing crítico vive numa thread Rust que não depende do WebView, e a UI reaproveita o design system web. O custo é a complexidade de janelas especiais (HUD click-through, overlays por monitor), que exige Win32 direto sobre os HWNDs do Tauri. Isso está previsto nas seções 3 e 5.

---

## 2. Estrutura do repositório

```
chenoli-auto-clicker/
├─ Cargo.toml                  # workspace Rust
├─ package.json                # workspace pnpm (ui)
├─ crates/
│  ├─ ch-core/                 # Rust puro, SEM dependência de Windows (testável em qualquer SO)
│  │   ├─ model/               # Profile, Settings, Hotkey, enums, validação
│  │   ├─ schema/              # versionamento, migrações v1→vN, formato .chprofile
│  │   ├─ patterns/            # ciclo de intervalos, wiggle, zigue-zague, multi-ponto
│  │   ├─ engine/              # máquina de estados, scheduler genérico sobre trait Clock
│  │   ├─ geometry/            # retângulos, layout de monitores, normalização 0..65535
│  │   └─ stats/               # histograma de jitter (p50/p95/p99/max)
│  ├─ ch-win/                  # implementações Windows (crate `windows`)
│  │   ├─ input/               # trait InputBackend: SendInputBackend, InterceptionBackend (feature)
│  │   ├─ timing/              # QpcClock, HighResWaiter (waitable timer HR + fallback), MMCSS
│  │   ├─ display/             # EnumDisplayMonitors, DPI, work area, WM_DISPLAYCHANGE
│  │   ├─ hotkeys/             # thread de hotkeys: RegisterHotKey + WH_KEYBOARD_LL/WH_MOUSE_LL
│  │   ├─ capture/             # captura GDI por monitor (BitBlt), amostragem de pixel
│  │   ├─ window/              # estilos HUD (WS_EX_*), resize animado, guarda "própria janela"
│  │   └─ theme/               # AppsUseLightTheme + WM_SETTINGCHANGE
│  ├─ ch-engine/               # runtime: thread do motor, RunPlan, executores, telemetria
│  └─ ch-storage/              # %APPDATA%, escrita atômica, backup, debounce, import/export
├─ src-tauri/                  # app Tauri: comandos, eventos, janelas, tray, updater, autostart
│   ├─ commands/               # um módulo por domínio (profiles, settings, engine, hotkeys, overlay, window, app)
│   ├─ events.rs               # nomes e payloads tipados (specta)
│   ├─ windows/                # main, variation, hud, overlay-N
│   ├─ tray/                   # menu nativo, ícones por estado
│   └─ icons/                  # ícones gerados (16/20/24/32/48/256)
├─ ui/
│   ├─ src/
│   │   ├─ entries/            # main.tsx, variation.tsx, hud.tsx, overlay.tsx (multi-página Vite)
│   │   ├─ ipc/                # bindings gerados + wrappers + mocks para testes
│   │   ├─ store/              # Zustand: engine, profiles, settings, hotkeys
│   │   ├─ components/         # primitivas do design system (Segmented, Toggle, RadioCard, Stepper, BigNumberField, Kbd, Badge, Button, Popover, Callout, Slider)
│   │   ├─ features/           # click, wiggle, scan, points, variation, profiles, hotkeys, themes, settings, overlay, hud
│   │   ├─ i18n/locales/{pt-BR,en-US}/*.json
│   │   ├─ styles/             # themes.css (gerado), base.css, fonts
│   │   └─ dev/gallery/        # rota de desenvolvimento com cada tela em cada estado (paridade visual)
│   └─ tests/visual/           # Playwright + screenshots de referência do protótipo
├─ design/                     # tokens.json (fonte da verdade) + script de geração
├─ tools/
│   ├─ jitter-bench/           # CLI de benchmark do scheduler
│   └─ gen-tokens/             # tokens.json → themes.css, tokens.ts, tokens.rs (sRGB)
├─ refs/                       # handoff de design (somente leitura)
├─ docs/                       # ARCHITECTURE.md, TASKS.md, bench/, checklists/
└─ .github/workflows/          # ci.yml, release.yml
```

**Regra de dependência:** `ch-core` ← `ch-win` ← `ch-engine` ← `src-tauri`. A UI só fala com `src-tauri` via IPC. `ch-core` não conhece Windows nem Tauri, e é ali que mora toda lógica testável: padrões, máquina de estados e migrações.

---

## 3. Diagrama da arquitetura

```
┌──────────────────────────── Processo único: ch-clicker.exe (PerMonitorV2) ────────────────────────────┐
│                                                                                                       │
│  WebView2 (React)                                     Rust                                            │
│  ┌──────────────┐  invoke (cmds)   ┌───────────────────────────────────────────────┐                  │
│  │ main  420×h  │ ───────────────▶ │ src-tauri: Command Router                     │                  │
│  │ variation    │ ◀─────────────── │  ├─ ProfileService ──▶ ch-storage (%APPDATA%) │                  │
│  │ hud          │   events         │  ├─ SettingsService ─▶ ch-storage             │                  │
│  │ overlay×N    │                  │  ├─ HotkeyService ◀─┐                          │                  │
│  └──────────────┘                  │  ├─ OverlayService ─┼─▶ ch-win::capture        │                  │
│        ▲                           │  ├─ WindowService ──┼─▶ ch-win::window         │                  │
│        │ 10 Hz agregado            │  ├─ Tray / Notifications / Updater             │                  │
│        │                           │  └─ EngineController ──cmd channel──┐          │                  │
│  ┌─────┴──────────┐                └─────────────────────────────────────┼──────────┘                  │
│  │ Telemetry      │◀── SPSC ring (lateness) + atômicos (clicks, estado) ─┐│                             │
│  │ Emitter thread │                                                     ││                             │
│  └────────────────┘                                                     │▼                             │
│                                         ┌───────────────────────────────┴─────────────┐                │
│  ┌──────────────────────┐   HotkeyEvent │ Engine thread (TIME_CRITICAL + MMCSS)       │                │
│  │ Hotkey thread        │ ─────────────▶│  StateMachine (idle/countdown/running/      │                │
│  │  message loop próprio│  + AtomicBool │                paused/stopping/error)       │                │
│  │  RegisterHotKey      │  EMERGENCY ──▶│  Scheduler (multi-timeline, deadlines QPC)  │                │
│  │  WH_KEYBOARD_LL      │               │   ├─ ClickTimeline (ciclo de intervalos)    │                │
│  │  WH_MOUSE_LL         │               │   ├─ MotionTimeline (wiggle | zigue-zague)  │                │
│  └──────────────────────┘               │   └─ StopTimeline (temporizador)            │                │
│                                         │  HighResWaiter + spin híbrido               │                │
│                                         │  SelfWindowGuard (nunca clica em si mesmo)  │                │
│                                         │        │                                    │                │
│                                         │        ▼                                    │                │
│                                         │  dyn InputBackend                           │                │
│                                         │   ├─ SendInputBackend (padrão)              │                │
│                                         │   └─ InterceptionBackend (experimental) ────┼─▶ driver kernel │
│                                         └─────────────────────────────────────────────┘                │
└───────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

### Threads

| Thread           | Responsabilidade                                                                                                     | Prioridade                                                                          |
| ---------------- | -------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| Main (Tauri/tao) | Loop de eventos, janelas, tray, comandos IPC                                                                         | normal                                                                              |
| Engine           | Countdown, scheduler, injeção. Recebe comandos por canal + `Event` Win32 para acordar do wait.                       | `THREAD_PRIORITY_TIME_CRITICAL` + MMCSS "Games" via `AvSetMmThreadCharacteristicsW` |
| Hotkey           | Message loop próprio com `RegisterHotKey` (hwnd message-only) e hooks LL. Responde ao hook em < 1 ms (só enfileira). | `ABOVE_NORMAL`                                                                      |
| Telemetry        | A cada 100 ms lê atômicos e o ring buffer e emite eventos Tauri agregados                                            | normal                                                                              |
| Window animator  | Resize animado da janela principal (`SetWindowPos` a cada frame)                                                     | normal                                                                              |

A thread do motor **nunca** chama o Tauri nem aloca no caminho quente. Ela escreve em atômicos e num ring buffer lock-free. A parada de emergência é um `AtomicBool` que o hook escreve e o motor lê a cada iteração, além de sinalizar o `Event` que acorda o wait. Isso garante a parada mesmo que o canal de comandos esteja congestionado.

---

## 4. Modelo de dados

Coordenadas sempre em **pixels físicos do desktop virtual** (podem ser negativas). Tempos em ms (inteiros), exceto amplitude (float).

### Enums

```
Mode         = "click" | "wiggle" | "scan"
MouseButton  = "left" | "middle" | "right" | "x1" | "x2"
ClickKind    = "single" | "double"
PositionMode = "cursor" | "points"
WiggleSpeed  = 4 | 8 | 16                       (Hz)
ScanSpeed    = 600 | 1200 | 2400                (px/s)
ScanAction   = "move" | "move_click"
Locale       = "pt-BR" | "en-US"
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

`mods ⊆ {ctrl, alt, shift, win}`. `key` é o nome canônico de um VK (`"F6"`, `"Escape"`, `"Right"`, `"P"`) ou de um botão do mouse (`"MouseMiddle"`, `"MouseX1"`, `"MouseX2"`). Botão esquerdo e direito não são aceitos como atalho.

### Profile (`schemaVersion: 1`)

Seguindo a **decisão de design 1**, a configuração de clique (`click`) e o temporizador (`stopAfter`) ficam no nível do perfil e são compartilhados pelos modos. Os parâmetros de cada modo ficam em `modes`, e todos são preservados ao trocar de aba.

```json
{
  "schemaVersion": 1,
  "id": "6f1c0e8a-3b6e-4c9a-9f3e-2a1d7c5b8e40",
  "name": "Farm de recursos",
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

**Validação** (em `ch-core`, aplicada no import, no update e no start):

- `intervalMs` de 1 a 60000. `blocksMs`: cada um de 1 a 60000, de 1 a 32 blocos.
- `stopAfter.seconds` ≥ 30, passo de 30 na UI. O backend aceita qualquer valor ≥ 30.
- `amplitudePct` de 0 a 100, com até 4 casas decimais. `lineSpacingPx` de 8 a 130.
- `points`: de 0 a 64, `delayMs` de 0 a 60000. Iniciar em `points` com lista vazia retorna o erro `NO_POINTS`.
- `scan.area` nulo, ou com `w,h` ≥ 16 px. Iniciar o scan sem área retorna o erro `NO_AREA`.
- `name`: de 1 a 48 caracteres, único (case-insensitive). Ao duplicar, o nome vira "Nome (2)".

`layoutFingerprint` registra o layout de monitores na captura. Se, ao iniciar, o layout atual não bater e algum ponto cair fora do desktop virtual, o start falha com `POINTS_OFFSCREEN` e a UI avisa.

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

Um atalho desativado é `null` (aparece como "—"). `emergencyStop` nunca pode ser `null`: o schema rejeita.

### Arquivos

```
%APPDATA%\Chenoli\CH Clicker\
├─ settings.json
├─ profiles\{id}.json            # um arquivo por perfil (+ {id}.json.bak com a versão anterior): corrupção isolada
├─ backups\                      # cópias pré-migração (perfil.vN.json), excluídos e arquivos corrompidos em quarentena
%LOCALAPPDATA%\Chenoli\CH Clicker\
├─ cache\capabilities.json       # resultado do benchmark (chave: build do Windows + CPU)
├─ cache\thumbs\{profileId}.png  # miniatura da área do scan
└─ logs\ch-clicker.log           # tauri-plugin-log (crate log), 5 MB × 5 arquivos
```

**Formato `.chprofile`** (import/export):

```json
{ "format": "chprofile", "schemaVersion": 1, "exportedBy": "CH. Clicker 0.1.0", "profile": { ... } }
```

No import, o perfil ganha um novo `id`. O `hotkey` é descartado se conflitar, e o nome é ajustado se duplicado.

---

## 5. Scheduler e máquina de estados

### 5.1 Estados

```
                ┌───────────── start (F6, clique, atalho do perfil) ─────────────┐
                │                                                               ▼
           ┌────────┐   cancel (F6/Esc/emergência)    ┌───────────┐
           │  idle  │ ◀────────────────────────────── │ countdown │  countdownSec = 0 ⇒ pula direto
           └────────┘                                 └───────────┘
             ▲   ▲  ▲                                       │ fim da contagem
             │   │  │                                       ▼
             │   │  │   F7    ┌────────┐    F7     ┌───────────┐
             │   │  └──────── │ paused │ ◀───────▶ │  running  │
             │   │            └────────┘           └───────────┘
             │   │                 │ F6/emergência/timer  │ F6, timer, emergência, scan sem loop concluído
             │   │                 ▼                      ▼
             │   │            ┌──────────────────────────────┐
             │   └─────────── │ stopping (libera botões,     │
             │                │ devolve cursor no wiggle,    │
             │                │ timeEndPeriod)               │
             │                └──────────────────────────────┘
             │  ack/start                       │ falha do backend durante a execução
          ┌───────┐ ◀───────────────────────────┘
          │ error │
          └───────┘
```

| De             | Evento                                                               | Para                                         | Efeito                                                                                                  |
| -------------- | -------------------------------------------------------------------- | -------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| idle / error   | `start`                                                              | countdown (ou running se `countdownSec = 0`) | Compila `RunPlan` a partir do snapshot do perfil. Falha de validação → permanece, emite `engine-error`. |
| countdown      | tick de 100 ms                                                       | countdown                                    | emite `remainingMs`                                                                                     |
| countdown      | contagem chega a 0                                                   | running                                      | `timeBeginPeriod`/timer HR, fixa `t0`                                                                   |
| countdown      | `start`/`stop`/emergência                                            | idle                                         | `reason = user/emergency`                                                                               |
| running        | `pause`                                                              | paused                                       | congela `elapsed` e `remaining`; wiggle volta à origem                                                  |
| paused         | `pause`                                                              | running                                      | desloca todos os deadlines pelo tempo pausado                                                           |
| running/paused | `stop`, timer, emergência, scan concluído                            | stopping → idle                              | `reason` correspondente                                                                                 |
| running        | erro do backend (ex.: `SendInput` bloqueado por UIPI/desktop seguro) | stopping → error                             | `engine-error {code}`                                                                                   |
| error          | `start` ou `ack`                                                     | countdown / idle                             |                                                                                                         |

**Erros de atalho não são estado do motor.** Ficam em `hotkeyFaults[]` (decisão de design 8). A UI deriva o visual do botão principal:

```
buttonVisual =
  engineState == countdown         → Contagem
  engineState == running           → Rodando
  engineState == paused            → Pausado (visual Rodando, kicker PAUSADO, cor fg-2)
  engineState == error             → Erro (código do motor)
  idle && hotkeyFaults não vazio   → Erro "ATALHO EM CONFLITO" (iniciar pelo clique continua possível)
  idle                             → Parado
```

O `RunPlan` é um **snapshot imutável**. Mudanças no perfil durante a execução valem a partir do próximo start (ver D8).

### 5.2 Scheduler multi-timeline

Cada execução tem até três timelines, e cada uma expõe `next_deadline()` em ticks QPC absolutos:

- **ClickTimeline**: intervalos do ciclo de variação (ou fixo). No multi-ponto, soma o `delayMs` do ponto e move até o ponto antes de clicar.
- **MotionTimeline**: wiggle ou zigue-zague. Atualização a taxa fixa (`MOTION_HZ = 250`), emitindo movimento só quando a posição inteira muda.
- **StopTimeline**: deadline único do temporizador.

```
loop do motor:
  se EMERGENCY.load(): ir para stopping(emergency)
  drenar canal de comandos (start/stop/pause/ack)
  t   = timeline com menor deadline
  now = qpc_now()
  rem = t.deadline - now
  se rem > SPIN_WINDOW (1,0 ms):
      WaitForMultipleObjects([hr_timer armado para (t.deadline - SPIN_WINDOW), cmd_event], timeout)
      continuar (reavalia; pode ter chegado comando)
  senão:
      spin com spin_loop_hint até now >= t.deadline     // no máx. ~1 ms de CPU por evento
  lateness = qpc_now() - t.deadline
  stats.record(lateness)                                 // ring buffer SPSC, sem alocar
  t.fire(backend)                                        // injeção
  t.advance()                                            // deadline += próximo intervalo (absoluto, sem drift)
  se lateness > max(2 × intervalo, 50 ms):               // ficou para trás (ex.: suspensão, travamento)
      t.rebase(now)                                      // política SKIP: não dispara rajada de recuperação
      stats.missed += eventos pulados
```

- **Deadlines absolutos** (`deadline_{n+1} = deadline_n + intervalo_n`): o erro não acumula, e a média de CPS fica exata mesmo com jitter individual.
- **Waiter:** `CreateWaitableTimerExW(CREATE_WAITABLE_TIMER_HIGH_RESOLUTION)` (Win10 1803+). Fallback: `timeBeginPeriod(1)` + waitable timer comum. `timeBeginPeriod` só fica ativo entre `running` e `stopping`, para não manter o sistema em 1 ms ocioso.
- **Spin híbrido:** os últimos 1,0 ms são em spin. O custo é até um núcleo durante intervalos muito curtos. Para intervalos ≥ 20 ms, a janela de spin cai para 0,3 ms.
- **Pausa:** guarda `paused_at` e, no resume, soma `now - paused_at` a todos os deadlines.
- **Clique duplo:** um único `SendInput` com 4 eventos (down/up/down/up). O par conta como 1 tick e 2 cliques no contador, e o CPS exibido é ×2, conforme o README.
- **Injeção atômica:** down e up no mesmo `SendInput`, então nunca sobra botão preso. No backend Interception (eventos separados), o `stopping` envia "up" de todos os botões por segurança.

### 5.3 Jitter, capacidades e aviso de inviabilidade

- **Medição contínua:** histograma de lateness com buckets logarítmicos de 1 µs a 1 s. A thread de telemetria publica `p50/p95/p99/max/missed` a cada 1 s.
- **Benchmark de capacidades** (`engine_run_benchmark`, executado em background na primeira abertura e após atualização do Windows; ~3 s): roda o scheduler com ação no-op em 1, 2, 4, 8 e 16 ms. `minReliableIntervalMs` é o menor intervalo com `p99 lateness < 25% do intervalo` e `missed = 0`. Fica em cache em `capabilities.json` e é exposto por `engine.capabilities` (decisão de design 3). Até o benchmark terminar, a UI usa 4 ms.
- **Aviso estático (UI):** blocos ou intervalo abaixo de `minReliableIntervalMs` recebem o dot `--warn` e o callout da tela 02.B.
- **Aviso dinâmico:** se o `p95` passar de 50% do intervalo por 2 s seguidos, o motor emite `engine-degraded {reason: "timing"}` e a UI mostra um callout warn no painel. Ele some após 5 s normais.
- **Limite honesto:** o app sabe medir quando a injeção ocorreu, não quando o app alvo processou o evento. Muitos apps leem input por frame. Isso vai documentado na UI de diagnóstico.

---

## 6. Algoritmos

### 6.1 Ciclo de variação de intervalos

```
intervals(cfg):
  se cfg.variation.enabled e blocksMs não vazio:
      i = 0
      loop: yield blocksMs[i]; i = (i + 1) mod len
  senão:
      loop: yield cfg.intervalMs

Primeiro clique: imediatamente ao entrar em running (t0). Os seguintes seguem intervals().
Estatísticas (tela 02.B): ciclo = Σ blocos; média = ciclo / n; efetivo (CPS) = 1000 / média × (duplo ? 2 : 1).
```

### 6.2 Multi click

```
pts = points (ordem do usuário); k = 0
fire(): alvo = pts[k]
        se SelfWindowGuard.is_own(alvo): skipped++ ; senão backend.move_abs(alvo); backend.click(button, kind)
        k = (k + 1) mod len
advance(): deadline += intervals.next() + pts[k].delayMs     // "Delay é somado ao intervalo"
```

"Posição atual do cursor" clica onde o cursor estiver, sem mover. Um ponto fixo único é o modo `points` com um ponto: "se o cursor estiver fora da posição configurada, ele vai até lá no momento do clique".

### 6.3 Coordenadas absolutas

```
V = desktop virtual (SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN)
nx = ceil((x - V.left) * 65536 / V.width)          // idem para y com V.top / V.height
SendInput(MOUSEEVENTF_MOVE | ABSOLUTE | VIRTUALDESK | MOVE_NOCOALESCE, nx, ny)
```

- O Windows converte de volta com `x = V.left + floor(nx * V.width / 65536)`. Arredondar para cima na ida é o inverso exato dessa conversão para qualquer largura até 65536 px. Um teste de propriedade em `ch-core::geometry` garante o round-trip para layouts aleatórios (origem negativa, 1 a 65536 px).
- Movimentos absolutos **não** sofrem aceleração do ponteiro ("Aprimorar precisão do ponteiro") nem a velocidade do mouse. Movimentos relativos sofrem, por isso nunca são usados.
- `MOVE_NOCOALESCE` impede que o Windows funda movimentos seguidos (importante no wiggle e no scan).
- Depois de cada movimento, `GetCursorPos` confere o pixel. Uma diferença de até 2 px é corrigida com `SetCursorPos` e contada como correção (diagnóstico). Uma diferença maior significa que o usuário mexeu o mouse, e o backend não briga com ele. O teste de integração `absolute_moves_land_on_the_exact_pixel_on_every_monitor` exige zero correções.
- O layout é recalculado em `WM_DISPLAYCHANGE` e `WM_DPICHANGED`. Se ele mudar durante uma execução, o motor pausa e emite `engine-degraded {reason: "display_changed"}`.

### 6.4 Mouse wiggle

**"Largura da tela"** (decisão de design 9): a largura em **pixels físicos do monitor onde o cursor está no start** (`MonitorFromPoint` → `rcMonitor`). É o que a UI mostra ("≈ 0,96 px em 1920"). O desktop virtual faria 1% significar coisas diferentes conforme o setup, e o movimento atravessaria monitores com DPI diferentes.

**Amplitude** = excursão pico a pico: `A = amplitudePct / 100 × monitorWidthPx`, então 100% equivale à largura inteira.

**Subpixel: mínimo de 1 px (decisão).** O cursor só existe em pixels inteiros. Acumular subpixel transformaria 0,05% em um passo de 1 px a cada ~20 ciclos: a frequência escolhida (4/8/16 Hz) deixaria de valer e o movimento pareceria travado. O que o usuário quer com valores < 1 px é "o menor movimento perceptível na frequência escolhida", então `A_eff = max(1, round(A))`. A UI mostra "≈ 0,96 px → 1 px efetivo" quando `A < 1`.

```
start:  origin = cursor; mon = monitor(origin)
        L = origin.x - floor(A_eff / 2); R = L + A_eff
        desloca [L, R] para dentro de mon (clamp sem cruzar monitor)
tick a MOTION_HZ (250 Hz):
        phase = frac(t × speedHz)                      // um ciclo = L→R→L
        x = L + round(A_eff × (1 - cos(2π × phase)) / 2)
        se x != último_x: backend.move_abs(x, origin.y)
pause/stop: backend.move_abs(origin)                    // devolve exatamente à origem
```

- **Sem drift:** cada posição é calculada de forma absoluta a partir de `origin`. Nada acumula.
- Com `A_eff = 1` a curva vira uma onda quadrada de 1 px, que é o comportamento desejado.
- **Intervenção do usuário:** se `GetCursorPos` divergir > 4 px da última posição comandada, `origin` passa a ser a posição do usuário (re-ancoragem). O usuário nunca "briga" com o cursor.
- **Clicar enquanto oscila:** a ClickTimeline roda em paralelo com a configuração de clique do perfil e clica na posição atual.

### 6.5 Screen scan (zigue-zague)

```
rows = floor(area.h / spacing) + 1
y_i  = area.top + i × spacing                          (i = 0..rows-1, y_i ≤ area.bottom)
polilinha P: (left, y0) → (right, y0) → (right, y1) → (left, y1) → (left, y2) → …
se loop: acrescenta segmento de retorno do último vértice a (left, y0)
comprimento L = rows × area.w + (rows - 1) × spacing [+ retorno]
ciclo = L / speedPxS                                   (selo "13 linhas · ciclo 9,2 s")

tick a MOTION_HZ:
  s = (t × speedPxS) mod L       (com loop)  | min(t × speedPxS, L) (sem loop)
  p = ponto_na_polilinha(P, s)   (busca por comprimento acumulado, pré-calculado)
  se round(p) != último: backend.move_abs(round(p))
  sem loop e s == L → stop(completed)
```

- **Ação:** "Só mover" só movimenta. "Mover e clicar" liga a ClickTimeline com a taxa herdada da aba Click (decisão de design 1), clicando na posição corrente da trajetória. **Decisão do prompt atendida:** o modo move e, opcionalmente, clica em intervalos.
- A 2400 px/s e 250 Hz, o passo é de 9,6 px. É suficiente para a trajetória parecer contínua, e o app não inunda a fila de input.
- O retorno no loop percorre um segmento reto até o início, na mesma velocidade, sem salto. A fórmula de `ciclo` inclui esse trecho. A tarefa de UI alinha o selo com essa fórmula.

### 6.6 Guarda "nunca clica na própria janela"

Antes de cada clique: `WindowFromPoint(alvo)` → `GetAncestor(GA_ROOT)` → `GetWindowThreadProcessId`. Se o PID for o nosso (janela principal, editor de variação ou overlays), o clique é **pulado**, `skippedSelf++`, e a telemetria informa. O HUD tem `WS_EX_TRANSPARENT | WS_EX_LAYERED`, então o hit-test o atravessa e o clique cai na janela abaixo, que é o comportamento desejado.

---

## 7. Contratos de IPC

Todos os comandos retornam `Result<T, AppError>`, com `AppError = { code: string, params?: object }`. A UI traduz `code` via i18n (`errors.<code>`). Nomes de eventos usam o prefixo de domínio.

### 7.1 Comandos (UI → Rust)

| Comando                                                   | Entrada                                                 | Saída                                                                                                                           | Notas                                                                                   |
| --------------------------------------------------------- | ------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| `app_bootstrap`                                           | —                                                       | `{ version, settings, profiles[], activeProfileId, engine: EngineSnapshot, capabilities, hotkeys: HotkeyStatus[], monitors[] }` | Uma chamada por janela no boot                                                          |
| `settings_update`                                         | `Partial<Settings>`                                     | `Settings`                                                                                                                      | Persistência com debounce de 300 ms. Emite `settings-changed`.                          |
| `profile_create`                                          | `{ name?, fromTemplate? }`                              | `Profile`                                                                                                                       |                                                                                         |
| `profile_update`                                          | `{ id, patch, baseUpdatedAt }`                          | `Profile`                                                                                                                       | Conflito de revisão (outra janela editou) → `STALE_PROFILE`                             |
| `profile_duplicate` / `profile_rename` / `profile_delete` | `{ id, name? }`                                         | `Profile` / `()`                                                                                                                | Não dá para excluir o último perfil ativo enquanto ele roda                             |
| `profile_reorder`                                         | `{ ids[] }`                                             | `()`                                                                                                                            |                                                                                         |
| `profile_set_active`                                      | `{ id }`                                                | `()`                                                                                                                            | Rodando → para antes (`reason = profileSwitch`)                                         |
| `profile_import`                                          | `{ path }`                                              | `{ profile, warnings[] }`                                                                                                       | Avisos: atalho descartado, nome ajustado, migrado de vN                                 |
| `profile_export`                                          | `{ id, path }`                                          | `()`                                                                                                                            | Diálogo de arquivo pelo plugin dialog                                                   |
| `engine_start`                                            | `{ profileId? }`                                        | `()`                                                                                                                            | Erros de validação: `NO_POINTS`, `NO_AREA`, `POINTS_OFFSCREEN`                          |
| `engine_stop`                                             | —                                                       | `()`                                                                                                                            |                                                                                         |
| `engine_toggle_pause`                                     | —                                                       | `()`                                                                                                                            |                                                                                         |
| `engine_snapshot`                                         | —                                                       | `EngineSnapshot`                                                                                                                |                                                                                         |
| `engine_capabilities`                                     | —                                                       | `Capabilities`                                                                                                                  |                                                                                         |
| `engine_run_benchmark`                                    | —                                                       | `Capabilities`                                                                                                                  | Assíncrono, ~3 s                                                                        |
| `engine_ack_error`                                        | —                                                       | `()`                                                                                                                            |                                                                                         |
| `hotkey_begin_record`                                     | `{ action }`                                            | `()`                                                                                                                            | Suspende os atalhos globais e liga o hook LL de gravação                                |
| `hotkey_cancel_record`                                    | —                                                       | `()`                                                                                                                            |                                                                                         |
| `hotkey_set`                                              | `{ action \| profileId, combo \| null, replace: bool }` | `HotkeySetResult = ok \| conflict{withAction\|withProfileId} \| registration_failed{reason}`                                    | `replace = true` tira o atalho do outro (fica "—")                                      |
| `hotkey_reset_defaults`                                   | —                                                       | `HotkeyStatus[]`                                                                                                                |                                                                                         |
| `hotkey_retry`                                            | `{ action }`                                            | `HotkeyStatus`                                                                                                                  | Ação "Tentar de novo" da notificação 09.B                                               |
| `overlay_pick_area`                                       | `{ profileId }`                                         | `Rect \| null`                                                                                                                  | Abre 03.A. Resolve ao confirmar ou cancelar.                                            |
| `overlay_pick_points`                                     | `{ profileId }`                                         | `Point[]` (adicionados)                                                                                                         | Abre 03.B                                                                               |
| `window_set_content_height`                               | `{ px }`                                                | `()`                                                                                                                            | Decisão de design 7: o Rust anima a altura de 640 a 900 px, limitada à área de trabalho |
| `window_minimize` / `window_close`                        | —                                                       | `()`                                                                                                                            | `close` aplica `closeAction`                                                            |
| `window_open_variation`                                   | `{ profileId }`                                         | `()`                                                                                                                            | Janela 02.B (560×640)                                                                   |
| `hud_set_visible` / `hud_save_position`                   | `{ visible }` / `{ x, y }`                              | `()`                                                                                                                            |                                                                                         |
| `app_open_url`                                            | `{ which: "support" \| "repo" }`                        | `()`                                                                                                                            | URL definida no Rust (decisão de design 10)                                             |
| `app_set_autostart`                                       | `{ enabled }`                                           | `()`                                                                                                                            |                                                                                         |
| `updater_check` / `updater_install`                       | —                                                       | `UpdateInfo \| null` / `()`                                                                                                     |                                                                                         |

### 7.2 Eventos (Rust → UI)

O tauri-specta nomeia cada evento em kebab-case a partir do tipo (`EngineTick` → `engine-tick`). Na implementação, as transições saem em `engine-status` (`EngineStatus`), a telemetria em `engine-tick`, as mudanças de dados em `settings-changed` e `profiles-changed`, e o resultado do benchmark em `capabilities`.

| Evento                | Payload                                                                                               | Frequência                        | Alimenta                                                                                               |
| --------------------- | ----------------------------------------------------------------------------------------------------- | --------------------------------- | ------------------------------------------------------------------------------------------------------ |
| `engine-state`        | `{ state, prevState, reason?: StopReason, profileId, mode, countdownTotalMs?, startedAt? }`           | em transições                     | botão principal (tela 10), rodapé, HUD, tray                                                           |
| `engine-tick`         | `{ countdownRemainingMs?, clicks, elapsedMs, remainingMs?, skippedSelf }`                             | 10 Hz em countdown/running/paused | contagem em décimos ("INICIANDO EM 2,4 S", ceil no botão), "CLIQUES n", "TEMPO mm:ss.d", "RESTA 04:32" |
| `engine-jitter`       | `{ intervalMs, p50Us, p95Us, p99Us, maxUs, missed }`                                                  | 1 Hz em running                   | diagnóstico                                                                                            |
| `engine-degraded`     | `{ reason: "timing" \| "display_changed" \| "uipi_blocked", detail? }`                                | quando ocorre                     | callout no painel                                                                                      |
| `engine-error`        | `AppError`                                                                                            | quando ocorre                     | estado Erro                                                                                            |
| `engine-capabilities` | `Capabilities { minReliableIntervalMs, timerKind: "high_res" \| "legacy", measuredAt }`               | após benchmark                    | aviso de 4 ms (02.B)                                                                                   |
| `hotkey-status`       | `HotkeyStatus[] = { action \| profileId, combo, status: "ok" \| "failed" \| "disabled", errorCode? }` | em mudanças                       | tela 06, estado Erro, notificação 09.B                                                                 |
| `hotkey-recording`    | `{ action, mods[], key?, done: bool, cancelled: bool }`                                               | a cada tecla durante a gravação   | campo de captura "Pressione as teclas…"                                                                |
| `hotkey-triggered`    | `{ action \| profileId }`                                                                             | quando disparado                  | feedback visual (opcional)                                                                             |
| `profiles-changed`    | `{ profiles[], activeProfileId }`                                                                     | em mudanças                       | sincroniza main, HUD e tray                                                                            |
| `settings-changed`    | `Settings`                                                                                            | em mudanças                       | tema, idioma em todas as janelas                                                                       |
| `display-changed`     | `Monitor[] = { id, name, rect, workArea, scale, primary }`                                            | `WM_DISPLAYCHANGE`                | minimapa, escala do wiggle                                                                             |
| `hud-hover`           | `{ over: bool }`                                                                                      | em mudanças                       | esmaecer o HUD para 40%                                                                                |
| `updater-available`   | `UpdateInfo`                                                                                          | após check                        | rodapé de 08                                                                                           |

**Contagem em décimos:** o motor emite `countdownRemainingMs` a cada 100 ms. A UI mostra `ceil(ms/1000)` no botão e `(ms/1000).toFixed(1)` com vírgula no rodapé.

---

## 8. Design tokens, temas e i18n

### 8.1 Tokens

- **Fonte da verdade:** `design/tokens.json`, cópia de `refs/tokens.json`. Os nomes não mudam e nenhum token novo é criado.
- `tools/gen-tokens` gera:
  - `ui/src/styles/themes.css`, equivalente a `design/themes.reference.css` (cópia versionada de `refs/themes.css`), com uma correção: os derivados também são declarados em `[data-theme]`, senão um tema aninhado (miniaturas da Tela 07) herdaria os derivados calculados no `:root`. Base por `[data-theme]` + derivados `--divider`, `--border-strong`, `--seg-active`, `--tint-*`, `--edge-*`, raios, sombras, easing.
  - `ui/src/styles/tokens.ts`: lista tipada dos 6 temas com `id/name/tag/description/tokens`, usada pelas miniaturas da tela 07, que desenham cada card com os tokens do próprio tema.
  - `crates/.../tokens.rs`: cores convertidas de `oklch` para sRGB hex, para o que é nativo (ícones da tray por estado).
  - O CI falha se os arquivos gerados divergirem do JSON.
- **Tailwind v4:** `@theme` mapeia as variáveis (`--color-bg: var(--bg)`, `--color-fg-3: var(--fg-3)`, `--color-accent: var(--accent)`…) e os raios (`--radius-card`…), para as classes lerem `bg-bg-2 text-fg-3 border-border rounded-control`.
- **Misturas pontuais do README** (`fg 12%`, `accent 22% sobre bg-2`, `warn 6%`, `danger 9%` etc.) **não viram tokens novos.** Ficam como `color-mix(in oklch, var(--x) N%, …)` locais no componente, via utilitário arbitrário do Tailwind ou CSS do componente.
- **Literais do protótipo** a converter: o fundo do HUD `rgba(20,20,18,.95)` vira `color-mix(in oklch, var(--bg) 95%, transparent)`, e os valores hex do Grafite são trocados pelos tokens. Fica literal só o vermelho de fechar `#C42B1C`, padrão do Win11.
- `color-mix`/`oklch` exigem Chromium 111+. O WebView2 evergreen atende, e o instalador garante o WebView2 (bootstrapper).

### 8.2 Temas

- `data-theme` no `<html>` de **todas** as janelas (main, variation, hud, overlays), aplicado no bootstrap e em `settings-changed`, antes do primeiro paint para não piscar.
- **Seguir o Windows:** lê `HKCU\…\Personalize\AppsUseLightTheme` e escuta `WM_SETTINGCHANGE("ImmersiveColorSet")`. Claro → Papel, escuro → Grafite.
- Contraste (AAA): foco sempre visível (`outline 2px var(--accent)`) em todos os temas. Nesse tema, as bordas usam o próprio `--border` (45%).

### 8.3 i18n

- **i18next**, namespaces por feature (`common`, `click`, `wiggle`, `scan`, `points`, `variation`, `profiles`, `hotkeys`, `themes`, `settings`, `hud`, `overlay`, `errors`, `native`). Os arquivos ficam em `ui/src/i18n/locales/{locale}/{ns}.json`.
- Chaves tipadas (declaração de resources em TS), então uma chave inexistente é erro de compilação. Um script de CI compara os conjuntos de chaves entre os locales.
- Números e tempos via `Intl.NumberFormat`/`Intl.DateTimeFormat` do locale ativo ("10,0" em pt-BR, "10.0" em en-US). Os formatos `mm:ss.d` e `hh:mm:ss` saem de um formatador próprio com o separador decimal do locale.
- O namespace `native` (tray, notificações, diálogos) é embutido no binário Rust (`include_str!`) a partir dos mesmos JSONs. Uma única fonte.
- Locale inicial: `GetUserDefaultUILanguage` (`pt*` → pt-BR, senão en-US). Adicionar idioma = adicionar pasta + entrada no enum `Locale` + opção no segmented da tela 08.
- **Toda string de UI usa i18n desde a fase 0**, para não haver retrabalho. A fase de i18n só adiciona en-US, a troca de idioma e o lado nativo.

---

## 9. Estratégia de testes

### 9.1 Unitários (`ch-core`, rodam em qualquer SO no CI)

- **Ciclo de intervalos:** ordem, wrap, variação desligada, estatísticas (ciclo/média/CPS) e CPS do duplo.
- **Wiggle:** com `proptest`, posições sempre em `[L, R]` dentro do monitor. Após N ciclos completos, a posição volta exatamente à origem. A regra `A < 1 → 1 px` vale, a frequência é preservada, há re-ancoragem e o clamp na borda funciona.
- **Zigue-zague:** número de linhas, vértices, comprimento, ponto por comprimento (monotônico), velocidade constante (amostras equidistantes), loop e fim sem loop.
- **Normalização:** round-trip de coordenadas para layouts com origem negativa, três monitores e escalas mistas.
- **Máquina de estados:** tabela completa de transições, incluindo eventos inválidos ignorados.
- **Scheduler sobre `FakeClock`** (tempo virtual): deadlines absolutos sem drift em 10⁶ eventos, política SKIP, pausa/resume que desloca deadlines, temporizador, merge de timelines.
- **Schema:** migrações a partir de fixtures por versão, rejeição de versão futura e de JSON inválido, regras de validação, import de `.chprofile` (novo id, nome duplicado, atalho em conflito).
- **Hotkeys:** detecção de conflito interno (ação × ação, ação × perfil), normalização de combos, regras de gravação (modificador sozinho não confirma, Esc sem modificador cancela).

### 9.2 Integração Windows (`ch-win`, runner `windows-latest`)

- `SendInputBackend` contra uma janela de teste criada no processo: conta `WM_LBUTTONDOWN/UP`, duplo, X1/X2.
- `HighResWaiter`: verifica que o timer HR está disponível e mede a resolução.
- Armazenamento: escrita atômica sob falha simulada (processo morto entre o write e o rename), rotação de `.bak`.
- Testes que dependem de sessão interativa (cursor real, hooks) ficam com `#[ignore]` e rodam localmente via `cargo test -- --ignored`.

### 9.3 Benchmark de jitter (`tools/jitter-bench`)

- Intervalos de 1, 2, 4, 8 e 16 ms × 10 s, em três cenários: ocioso, CPU 100% em todos os núcleos (stress) e na bateria (modo economia).
- Saída: tabela Markdown + CSV com p50/p95/p99/max/missed, salva em `docs/bench/{data}-{máquina}.md`.
- Mede também o custo de `SendInput` (µs por chamada).
- É o insumo para confirmar ou ajustar o limite provisório de 4 ms.

### 9.4 UI

- **Vitest + Testing Library:** primitivas (Segmented, Stepper com passo/limites, BigNumberField com vírgula/ponto, Toggle), formatadores (CPS, tempo), validações, store (aplicação de eventos), fluxo de gravação de atalho com IPC mockado (`@tauri-apps/api/mocks`).
- **Paridade visual (Playwright):** a rota `dev/gallery` renderiza cada tela e cada estado do README (01.A–C, 02.A–B, 03.A–B, 04.A–C, 05.A–B, 06 nos 3 estados, 07, 08, 09.A–B, 10 nos 4 estados) em tamanho fixo. As referências são capturadas do protótipo `refs/design/CH Clicker.dc.html` com as mesmas fontes, por âncora (`#janela`, `#pontos`…). A comparação usa `toHaveScreenshot` com tolerância baixa. Cada tela tem seu teste, e é ele que fecha o critério de aceite "paridade visual".

### 9.5 Checklist manual (`docs/checklists/multimonitor-dpi.md`)

1. Um monitor a 100%, 125%, 150% e 200%: pontos capturados caem no pixel exato (verificar com a lupa e `GetCursorPos`).
2. Dois monitores com escalas diferentes (100% + 150%), secundário à esquerda (coordenadas negativas) e acima (Y negativo).
3. Três monitores em L. Pontos em todos. O minimapa mostra o monitor certo.
4. Overlay de área em cada monitor: dimensões do HUD em pixels físicos e miniatura correta.
5. Wiggle: largura vem do monitor do cursor, sem atravessar para o vizinho na borda, volta à origem.
6. Scan perto das bordas de cada monitor.
7. Trocar a escala ou desconectar um monitor durante a execução → pausa + aviso. Iniciar com pontos fora do layout → `POINTS_OFFSCREEN`.
8. Janela principal arrastada entre monitores de DPI diferente: nitidez, altura máxima limitada à área de trabalho.
9. HUD: canto superior direito do monitor primário, click-through, esmaece com hover, arrasto (Ctrl + arrastar, D5).
10. App alvo elevado (admin) com app não elevado → `uipi_blocked` detectado e avisado.
11. Bloqueio de tela/UAC durante a execução → para com erro, sem botão preso.
12. Parada de emergência com a janela alvo em foco, em jogo em tela cheia e com o app sem foco.

---

## 10. Segurança, riscos e decisões

### 10.1 Segurança de uso

- **Parada de emergência** (`Ctrl Alt Esc`, não desativável): registrada em **dois caminhos** independentes, `RegisterHotKey` e `WH_KEYBOARD_LL`. Os dois escrevem no `AtomicBool` lido a cada iteração do motor. Um watchdog reinstala o hook se o Windows o remover por timeout (`LowLevelHooksTimeout`). Se o registro via `RegisterHotKey` falhar, o hook garante a parada e a UI avisa.
- **Contagem regressiva** antes de iniciar (0–10 s, padrão 3 s), visível no botão, no rodapé, no HUD e na tray.
- **Nunca clica na própria janela** (seção 6.6).
- **Eventos próprios ignorados nos hooks:** o hook de mouse descarta eventos com `LLMHF_INJECTED`, então um clique X1 do motor não dispara um atalho X1. Com o backend Interception, que não marca eventos como injetados, o app **proíbe** usar como atalho o mesmo botão que o perfil clica.
- **Aviso anti-cheat** fixo na tela 08 (texto do README) e no primeiro start (diálogo único com "Entendi").
- O app não oculta a atividade: nada de evasão de detecção, processo com nome real e eventos `SendInput` marcados como injetados.

### 10.2 Backends de input

|            | SendInput (padrão)                                                                       | Interception (experimental)                                                                                                                                                                           |
| ---------- | ---------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Nível      | User-mode, fila de input do Windows                                                      | Driver de filtro kernel (teclado/mouse)                                                                                                                                                               |
| Marcação   | `LLMHF_INJECTED`/`LLKHF_INJECTED` visível para hooks LL                                  | Indistinguível de um dispositivo físico para hooks                                                                                                                                                    |
| UIPI       | Não injeta em janelas de integridade mais alta (app elevado) e **falha silenciosamente** | Não sofre UIPI                                                                                                                                                                                        |
| Instalação | Nenhuma                                                                                  | Instalador do driver com admin + **reinicialização**                                                                                                                                                  |
| Riscos     | Baixos                                                                                   | Driver de terceiros antigo, relatos de instabilidade com alguns dispositivos, detectado e bloqueado por anti-cheats modernos (a presença do driver é o sinal), licença LGPL/não comercial a verificar |
| Fallback   | —                                                                                        | Se o driver não responder no start, volta para SendInput e avisa                                                                                                                                      |

**Fluxo Interception (v0.2, ver D10):** Configurações → Avançado → "Backend de entrada". Um diálogo explica os riscos e o usuário confirma explicitamente. O app abre a página oficial do driver: não redistribuímos o instalador até a licença ser validada. Depois do reboot, o app detecta o driver (`interception.dll` + dispositivo). Feature flag de compilação `interception`, ausente do build da v0.1.

"Clique em nível de hardware" de verdade (dispositivo HID físico, ex. microcontrolador) fica fora do escopo e é só citado como alternativa.

### 10.3 Riscos técnicos conhecidos

| Risco                                                                              | Mitigação                                                                                                                                                                                                     |
| ---------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Resize animado da janela via `SetWindowPos` com WebView2 pode piscar               | Animador no Rust, layout ancorado no topo e fundo `--bg` no host. Fallback: resize instantâneo (flag).                                                                                                        |
| Overlay único cobrindo monitores com DPI misto renderiza com um só fator de escala | **Uma janela de overlay por monitor** (desvio consciente da "uma janela" do README), coordenadas convertidas por `monitor.rect + css × scale`                                                                 |
| Overlay precisa de teclado (Esc, setas, Enter) mas o prompt fala em "sem foco"     | O overlay recebe foco: é modal e o motor fica forçado em idle durante a seleção. O HUD continua `NOACTIVATE`.                                                                                                 |
| Lupa precisa dos pixels da tela                                                    | Captura GDI `BitBlt` por monitor ao abrir o overlay (tela congelada, como a ferramenta de recorte), servida ao WebView por protocolo customizado `chcap://` como BMP, sem base64 no IPC. A lupa lê do canvas. |
| HUD click-through × "arrastável"                                                   | Ver D5                                                                                                                                                                                                        |
| Hover no HUD click-through (sem eventos de mouse)                                  | Polling de `GetCursorPos` a 30 Hz no Rust → `hud-hover`                                                                                                                                                       |
| Notificação nativa com ações exige AUMID registrado                                | O instalador NSIS registra o atalho com AUMID. Sem ele, fallback para notificação simples + estado visível.                                                                                                   |
| SmartScreen em binário novo                                                        | Assinatura Authenticode + reputação acumulada. Desde 2024, EV não dá reputação instantânea.                                                                                                                   |
| Spin-wait consome CPU                                                              | Janela de spin curta (1 ms), só durante `running`, reduzida para intervalos longos                                                                                                                            |
| Hook LL removido por timeout                                                       | O callback só enfileira, watchdog reinstala                                                                                                                                                                   |

### 10.4 Distribuição

- **Instalador:** NSIS do Tauri (per-user, sem admin, `%LOCALAPPDATA%\Programs`) com bootstrapper do WebView2. MSI opcional para quem preferir.
- **Assinatura:** Authenticode no CI (ver D9). **Updater:** `tauri-plugin-updater` com chave minisign (secret do GitHub), manifest `latest.json` em GitHub Releases. Verificação manual em "Verificar atualizações" e automática a cada 24 h, silenciosa.
- **Autostart:** `tauri-plugin-autostart` (chave Run do HKCU) com `--minimized`, que abre na bandeja.
- **Single instance:** `tauri-plugin-single-instance`. A segunda execução foca a janela existente.
- **CI (GitHub Actions):** `ci.yml` em PR (fmt, clippy `-D warnings`, `cargo test`, eslint, tsc, vitest, Playwright visual, checagem de tokens/i18n, `cargo audit`/`pnpm audit`). `release.yml` em tag `v*` (build, assinatura, upload do instalador + `latest.json` para Releases).

### 10.5 Decisões tomadas

Decididas em 2026-10-04 com base em boas práticas de mercado, design e usabilidade. Qualquer uma pode ser revista.

| #   | Tema                                     | Decisão                                                                                                                                                                                                                                                                             | Justificativa                                                                                                                                                                                                                                                                                                                                                     |
| --- | ---------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D1  | Botões X1/X2                             | O segmented BOTÃO ganha um 4º item compacto "Lateral ▾", que abre um popover com "X1 · Voltar" e "X2 · Avançar". O item mostra "X1"/"X2" quando selecionado.                                                                                                                        | Mantém os 3 botões mais usados com acesso direto e a paridade visual do protótipo. Os botões laterais ficam a um clique, sem poluir a linha.                                                                                                                                                                                                                      |
| D2  | Contagem configurável                    | Nova linha de 56 px na tela 08, "Contagem antes de iniciar", com stepper de 0 a 10 s (passo 1), entre "Mostrar HUD ao rodar" e "Ao fechar a janela".                                                                                                                                | É uma preferência global, como as vizinhas. Segue o padrão de linha da tela.                                                                                                                                                                                                                                                                                      |
| D3  | Diagnóstico                              | Seção "Avançado" recolhível no fim da tela 08: intervalo mínimo confiável medido, último p95/p99 e botão "Medir novamente". Na v0.1 não há seletor de backend (ver D10).                                                                                                            | Informação técnica fica disponível sem pesar na tela para quem não precisa (divulgação progressiva).                                                                                                                                                                                                                                                              |
| D4  | Atalho por perfil                        | Item "Definir atalho…" no menu de contexto da tela 05, acima de "Duplicar", que abre o campo de captura inline no card, com as mesmas regras de conflito da tela 06. Pressionar o atalho de um perfil ativa o perfil e inicia; pressionar de novo para.                             | Reaproveita o componente e as regras que já existem. "Um atalho = um perfil pronto para rodar" é o uso que justifica ter atalho por perfil.                                                                                                                                                                                                                       |
| D5  | HUD click-through × arrastável           | Click-through sempre. Com **Ctrl** pressionado e o cursor sobre o HUD, ele fica interativo e pode ser arrastado. A posição é salva. Uma dica "Segure Ctrl para mover" aparece nas 3 primeiras vezes que o HUD é exibido. O item "Mostrar HUD" da bandeja ganha "Restaurar posição". | O motor nunca pode clicar no HUD por acidente: segurança vem antes. Padrão parecido com overlays de jogos. A dica resolve a descoberta.                                                                                                                                                                                                                           |
| D6  | Captura de pontos                        | Contínua: cada clique adiciona um ponto (o rótulo avança "05", "06"…), Enter ou Esc encerra mantendo os pontos adicionados, e Ctrl+Z desfaz o último.                                                                                                                               | Montar 4 ou mais pontos exigiria abrir o overlay várias vezes. O Esc preserva o trabalho, e o Ctrl+Z cobre o clique errado.                                                                                                                                                                                                                                       |
| D7  | Área entre monitores                     | A área fica restrita ao monitor onde o arrasto começou.                                                                                                                                                                                                                             | Evita a ambiguidade de DPI misto e cantos fora do desktop. É o mesmo comportamento de ferramentas de captura comuns.                                                                                                                                                                                                                                              |
| D8  | Editar durante a execução                | Snapshot no start. Enquanto roda, os controles continuam editáveis e o painel mostra a linha "Alterações valem ao reiniciar".                                                                                                                                                       | É previsível: o que está rodando é o que foi iniciado. Não trava a UI e deixa claro o efeito da edição.                                                                                                                                                                                                                                                           |
| D9  | Assinatura de código                     | Sem assinatura até a Fase 13. Na release: **Certum Open Source Code Signing** se o repositório for público; senão, certificado OV (SSL.com ou Sectigo). O Azure Trusted Signing hoje só aceita desenvolvedores individuais dos EUA e do Canadá.                                     | É o caminho mais barato e disponível para pessoa física no Brasil. A compra fica com você na Fase 13. **A assinatura é obrigatória para o lançamento público:** o Controle Inteligente de Aplicativos do Windows 11 bloqueia executáveis sem assinatura de uma autoridade certificadora reconhecida, então sem ela o app não abre para quem tem o recurso ligado. |
| D10 | Driver Interception                      | **Fora da v0.1**, previsto para a v0.2. A abstração `InputBackend` já nasce pronta para recebê-lo. Quando entrar: não redistribuir o driver, link para o instalador oficial, aviso de risco e opt-in explícito.                                                                     | Driver de kernel de terceiros tem risco real (instabilidade, ban por anti-cheat, licença) e não é necessário para o valor principal do produto.                                                                                                                                                                                                                   |
| D11 | Limite de intervalo                      | 4 ms até o benchmark rodar. Depois, vale o `minReliableIntervalMs` medido em cada máquina.                                                                                                                                                                                          | É honesto com o hardware real e mantém a decisão de design 3.                                                                                                                                                                                                                                                                                                     |
| D12 | Próximo perfil durante a execução        | Para a execução, troca de perfil e fica parado. O HUD/a bandeja mostram o novo perfil.                                                                                                                                                                                              | Nunca iniciar automação com uma configuração que o usuário não conferiu.                                                                                                                                                                                                                                                                                          |
| D13 | Editor de variação                       | Janela própria de 560×640, filha da principal e modal em relação a ela, centrada sobre a principal.                                                                                                                                                                                 | Respeita as medidas do design sem distorcer a janela principal de largura fixa.                                                                                                                                                                                                                                                                                   |
| D14 | Cantos no Windows 10                     | Win11 usa os cantos arredondados nativos do DWM. Win10 fica com cantos retos e sombra nativa.                                                                                                                                                                                       | Janela transparente só para arredondar custaria desempenho e a sombra nativa. É assim que os apps nativos se comportam no Win10.                                                                                                                                                                                                                                  |
| D15 | Links externos                           | Apoio: `https-ko-fi.com/chenoli` (botão "Apoiar ↗" da tela 08 e item da bandeja). URL do repositório público: definida na Fase 13 junto com o updater.                                                                                                                              | Definido pelo autor.                                                                                                                                                                                                                                                                                                                                              |
| D16 | Usuário mexe o mouse durante wiggle/scan | Wiggle passa a oscilar onde o usuário deixou o cursor. Scan e multi-ponto seguem normalmente, porque reposicionam o cursor a cada passo. Parar continua a um toque (F6/emergência).                                                                                                 | Não briga com o usuário no modo em que isso incomodaria e não interrompe automações de trajetória.                                                                                                                                                                                                                                                                |
