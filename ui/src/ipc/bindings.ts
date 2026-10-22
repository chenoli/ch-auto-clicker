import { invoke as __TAURI_INVOKE } from "@tauri-apps/api/core";
import * as __TAURI_EVENT from "@tauri-apps/api/event";

export const commands = {
	appBootstrap: () => __TAURI_INVOKE<Bootstrap>("app_bootstrap"),
	settingsUpdate: (settings: Settings) => typedError<Settings, AppError>(__TAURI_INVOKE("settings_update", { settings })),
	settingsGet: () => __TAURI_INVOKE<Settings>("settings_get"),
	settingsAcknowledgeAntiCheat: () => __TAURI_INVOKE<Settings>("settings_acknowledge_anti_cheat"),
	appOpenSupport: () => typedError<null, AppError>(__TAURI_INVOKE("app_open_support")),
	hudResize: (width: number | null, height: number | null) => __TAURI_INVOKE<void>("hud_resize", { width, height }),
	updateCheck: () => typedError<{
	version: string,
	notes: string | null,
} | null, AppError>(__TAURI_INVOKE("update_check")),
	updateInstall: () => typedError<null, AppError>(__TAURI_INVOKE("update_install")),
	profileUpdate: (profile: Profile, baseUpdatedAt: string) => typedError<Profile, AppError>(__TAURI_INVOKE("profile_update", { profile, baseUpdatedAt })),
	engineStart: (profileId: string | null) => typedError<null, AppError>(__TAURI_INVOKE("engine_start", { profileId })),
	engineStop: () => __TAURI_INVOKE<void>("engine_stop"),
	engineTogglePause: () => __TAURI_INVOKE<void>("engine_toggle_pause"),
	engineAckError: () => __TAURI_INVOKE<void>("engine_ack_error"),
	engineSnapshot: () => __TAURI_INVOKE<EngineTick>("engine_snapshot"),
	engineCapabilities: () => __TAURI_INVOKE<Capabilities>("engine_capabilities"),
	engineRunBenchmark: () => typedError<Capabilities, string>(__TAURI_INVOKE("engine_run_benchmark")),
	hotkeyStatus: () => __TAURI_INVOKE<HotkeyStatus[]>("hotkey_status"),
	hotkeyBeginRecord: () => __TAURI_INVOKE<void>("hotkey_begin_record"),
	hotkeyRecordKey: (key: string, down: boolean) => __TAURI_INVOKE<boolean>("hotkey_record_key", { key, down }),
	hotkeyCancelRecord: () => __TAURI_INVOKE<HotkeyStatus[]>("hotkey_cancel_record"),
	hotkeySet: (target: HotkeyTarget, combo: {
	mods: Modifier[],
	key: string,
} | null, replace: boolean) => typedError<HotkeySetResult, AppError>(__TAURI_INVOKE("hotkey_set", { target, combo, replace })),
	hotkeyResetDefaults: () => __TAURI_INVOKE<HotkeyStatus[]>("hotkey_reset_defaults"),
	hotkeyRetry: () => __TAURI_INVOKE<HotkeyStatus[]>("hotkey_retry"),
	screenLayout: () => __TAURI_INVOKE<ScreenLayout>("screen_layout"),
	profileCreate: (name: string) => typedError<Profile, AppError>(__TAURI_INVOKE("profile_create", { name })),
	profileDuplicate: (id: string) => typedError<Profile, AppError>(__TAURI_INVOKE("profile_duplicate", { id })),
	profileRename: (id: string, name: string) => typedError<Profile, AppError>(__TAURI_INVOKE("profile_rename", { id, name })),
	profileDelete: (id: string) => typedError<null, AppError>(__TAURI_INVOKE("profile_delete", { id })),
	profileReorder: (order: string[]) => typedError<null, AppError>(__TAURI_INVOKE("profile_reorder", { order })),
	profileActivate: (id: string) => typedError<null, AppError>(__TAURI_INVOKE("profile_activate", { id })),
	profileExport: (id: string) => typedError<string | null, AppError>(__TAURI_INVOKE("profile_export", { id })),
	profileImport: () => typedError<{
	profile: Profile,
	notes: ImportNote[],
} | null, AppError>(__TAURI_INVOKE("profile_import")),
	profileImportText: (text: string) => typedError<ImportResult, AppError>(__TAURI_INVOKE("profile_import_text", { text })),
	overlayPickPoints: () => typedError<null, AppError>(__TAURI_INVOKE("overlay_pick_points")),
	overlayPickArea: () => typedError<null, AppError>(__TAURI_INVOKE("overlay_pick_area")),
	overlaySetMode: (mode: OverlayMode) => typedError<null, AppError>(__TAURI_INVOKE("overlay_set_mode", { mode })),
	overlayConfirmArea: (monitor: number, x: number | null, y: number | null, width: number | null, height: number | null) => typedError<Area, AppError>(__TAURI_INVOKE("overlay_confirm_area", { monitor, x, y, width, height })),
	overlayState: (monitor: number) => typedError<OverlayState, AppError>(__TAURI_INVOKE("overlay_state", { monitor })),
	overlayAddPoint: (monitor: number, x: number | null, y: number | null) => typedError<OverlayState, AppError>(__TAURI_INVOKE("overlay_add_point", { monitor, x, y })),
	overlayUndo: () => typedError<null, AppError>(__TAURI_INVOKE("overlay_undo")),
	overlayFinish: () => __TAURI_INVOKE<void>("overlay_finish"),
	overlayFocus: (monitor: number) => __TAURI_INVOKE<void>("overlay_focus", { monitor }),
	overlayFocusCursor: () => __TAURI_INVOKE<void>("overlay_focus_cursor"),
	windowSetContentHeight: (height: number | null) => typedError<number | null, AppError>(__TAURI_INVOKE("window_set_content_height", { height })),
	windowOpenVariation: () => typedError<null, AppError>(__TAURI_INVOKE("window_open_variation")),
	debugClickOnce: () => typedError<null, string>(__TAURI_INVOKE("debug_click_once")),
};

export const events = {
	capabilities: makeEvent<Capabilities>("capabilities"),
	engineDegraded: makeEvent<EngineDegraded>("engine-degraded"),
	engineJitter: makeEvent<EngineJitter>("engine-jitter"),
	engineStatus: makeEvent<EngineStatus>("engine-status"),
	engineTick: makeEvent<EngineTick>("engine-tick"),
	hotkeyActionFailed: makeEvent<HotkeyActionFailed>("hotkey-action-failed"),
	hotkeyRecording: makeEvent<HotkeyRecording>("hotkey-recording"),
	hotkeyStatusChanged: makeEvent<HotkeyStatusChanged>("hotkey-status-changed"),
	hudHover: makeEvent<HudHover>("hud-hover"),
	navigateRequested: makeEvent<NavigateRequested>("navigate-requested"),
	overlayChanged: makeEvent<OverlayChanged>("overlay-changed"),
	profilesChanged: makeEvent<ProfilesChanged>("profiles-changed"),
	settingsChanged: makeEvent<SettingsChanged>("settings-changed"),
	updateAvailable: makeEvent<UpdateAvailable>("update-available"),
};

export type AppError = {
	code: string,
	message: string,
};

export type Area = {
	x: number,
	y: number,
	w: number,
	h: number,
};

export type BenchResult = {
	intervalMs: number,
	p99Us: number | null,
	missed: number,
	reliable: boolean,
};

export type Bootstrap = {
	version: string,
	dpiAwareness: string,
	settings: Settings,
	profiles: Profile[],
	engine: EngineTick,
	capabilities: Capabilities,
	hotkeys: HotkeyStatus[],
};

export type Capabilities = {
	minReliableIntervalMs: number,
	measured: boolean,
	timerKind: string,
	mmcss: boolean,
	measuredAtMs: number | null,
	fingerprint: string,
	results: BenchResult[],
};

export type ClickKind = "single" | "double";

export type ClickMode = {
	position: Position,
	points: ScreenPoint[],
	layoutFingerprint: string | null,
};

export type ClickSettings = {
	button: MouseButton,
	kind: ClickKind,
	intervalMs: number,
	variation: Variation,
};

export type CloseAction = "minimize" | "quit";

export type DegradedReason = "timing" | "displayChanged";

export type EngineDegraded = {
	degraded: boolean,
	reason: DegradedReason,
};

export type EngineJitter = {
	samples: number,
	p50Us: number | null,
	p95Us: number | null,
	p99Us: number | null,
	maxUs: number | null,
	missed: number,
};

export type EngineState = "idle" | "countdown" | "running" | "paused" | "stopping" | "error";

export type EngineStatus = {
	state: EngineState,
	prevState: EngineState,
	reason: StopReason | null,
	countdownTotalMs: number | null,
	errorCode: string | null,
};

export type EngineTick = {
	state: EngineState,
	fires: number,
	clicks: number,
	skippedSelf: number,
	missed: number,
	elapsedMs: number | null,
	countdownRemainingMs: number | null,
	remainingMs: number | null,
};

export type Hotkey = {
	mods: Modifier[],
	key: string,
};

export type HotkeyAction = "startStop" | "pauseResume" | "capturePoint" | "nextProfile" | "emergencyStop";

export type HotkeyActionFailed = {
	code: string,
};

export type HotkeyBindings = {
	startStop: Hotkey | null,
	pauseResume: Hotkey | null,
	capturePoint: Hotkey | null,
	nextProfile: Hotkey | null,
	emergencyStop: Hotkey,
};

export type HotkeyRecording = {
	mods: Modifier[],
	combo: Hotkey | null,
	cancelled: boolean,
};

export type HotkeySetResult = { result: "saved"; statuses: HotkeyStatus[] } | { result: "conflict"; with: HotkeyTarget };

export type HotkeyStatus = {
	target: HotkeyTarget,
	combo: Hotkey,
	ok: boolean,
	errorCode: string | null,
};

export type HotkeyStatusChanged = HotkeyStatus[];

export type HotkeyTarget = { kind: "action"; id: HotkeyAction } | { kind: "profile"; id: string };

export type HudHover = {
	hover: boolean,
	draggable: boolean,
};

export type ImportNote = { kind: "renamed"; from: string; to: string } | { kind: "hotkeyDropped"; combo: Hotkey } | { kind: "migrated"; from_version: number };

export type ImportResult = {
	profile: Profile,
	notes: ImportNote[],
};

export type InputBackendKind = "sendinput" | "interception";

export type Locale = "pt-BR" | "en-US" | "es-419";

export type Mode = "click" | "wiggle" | "scan";

export type Modes = {
	click: ClickMode,
	wiggle: WiggleMode,
	scan: ScanMode,
};

export type Modifier = "ctrl" | "alt" | "shift" | "win";

export type MouseButton = "left" | "middle" | "right" | "x1" | "x2";

export type NavigateRequested = {
	screen: string,
};

export type OverlayChanged = {
	mode: OverlayMode,
};

export type OverlayCursor = {
	x: number,
	y: number,
};

export type OverlayMode = "points" | "area";

export type OverlayMonitor = {
	index: number,
	x: number,
	y: number,
	width: number,
	height: number,
	scale: number | null,
};

export type OverlayPoint = {
	number: number,
	x: number,
	y: number,
};

export type OverlayState = {
	monitor: OverlayMonitor,
	mode: OverlayMode,
	points: OverlayPoint[],
	nextNumber: number,
	area: Area | null,
	cursor: OverlayCursor | null,
};

export type Position = "cursor" | "points";

export type Profile = {
	schemaVersion: number,
	id: string,
	name: string,
	mode: Mode,
	hotkey: Hotkey | null,
	createdAt: string,
	updatedAt: string,
	click: ClickSettings,
	stopAfter: StopAfter,
	modes: Modes,
};

export type ProfilesChanged = {
	profiles: Profile[],
	activeProfileId: string | null,
};

export type ScanAction = "move" | "move_click";

export type ScanMode = {
	area: Area | null,
	lineSpacingPx: number,
	speedPxS: number,
	action: ScanAction,
	loop: boolean,
};

export type ScreenLayout = {
	monitors: OverlayMonitor[],
	fingerprint: string,
};

export type ScreenPoint = {
	id: string,
	x: number,
	y: number,
	delayMs: number,
};

export type Settings = {
	schemaVersion: number,
	locale: Locale,
	theme: ThemeId,
	followSystemTheme: boolean,
	autostart: boolean,
	minimizeToTray: boolean,
	showHud: boolean,
	closeAction: CloseAction,
	countdownSec: number,
	inputBackend: InputBackendKind,
	activeProfileId: string | null,
	profileOrder: string[],
	hotkeys: HotkeyBindings,
	hud: WindowPosition,
	window: WindowPosition,
	antiCheatAcknowledged?: boolean,
};

export type SettingsChanged = Settings;

export type StopAfter = {
	enabled: boolean,
	seconds: number,
};

export type StopReason = "user" | "timer" | "emergency" | "completed" | "error" | "profileSwitch";

export type ThemeId = "grafite" | "blueprint" | "fosforo" | "bancada" | "papel" | "contraste";

export type UpdateAvailable = {
	version: string,
	notes: string | null,
};

export type Variation = {
	enabled: boolean,
	blocksMs: number[],
};

export type WiggleMode = {
	amplitudePct: number | null,
	speedHz: number,
	clickWhileWiggling: boolean,
};

export type WindowPosition = {
	x: number | null,
	y: number | null,
};

async function typedError<T, E>(result: Promise<T>): Promise<{ status: "ok"; data: T } | { status: "error"; error: E }> {
    try {
        return { status: "ok", data: await result };
    } catch (e) {
        if (e instanceof Error) throw e;
        return { status: "error", error: e as any };
    }
}

type EventEmit<T> = [T] extends [null] ? () => Promise<void> : (payload: T) => Promise<void>;

function makeEvent<T>(name: string, serialize?: (payload: T) => unknown, deserialize?: (payload: any) => T) {
    const mapEvent = (cb: __TAURI_EVENT.EventCallback<T>) => (event: __TAURI_EVENT.Event<any>) => cb({ ...event, payload: deserialize ? deserialize(event.payload) : event.payload });
    const mapPayload = (payload: T) => serialize ? serialize(payload) : payload;

    const base = {
        listen: (cb: __TAURI_EVENT.EventCallback<T>) => __TAURI_EVENT.listen(name, mapEvent(cb)),
        once: (cb: __TAURI_EVENT.EventCallback<T>) => __TAURI_EVENT.once(name, mapEvent(cb)),
        emit: ((payload: T) => __TAURI_EVENT.emit(name, mapPayload(payload)) as unknown) as EventEmit<T>
    };

    const fn = (target: import("@tauri-apps/api/webview").Webview | import("@tauri-apps/api/window").Window) => ({
        listen: (cb: __TAURI_EVENT.EventCallback<T>) => target.listen(name, mapEvent(cb)),
        once: (cb: __TAURI_EVENT.EventCallback<T>) => target.once(name, mapEvent(cb)),
        emit: ((payload: T) => target.emit(name, mapPayload(payload)) as unknown) as EventEmit<T>
    });

    return Object.assign(fn, base);
}

