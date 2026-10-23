use serde::{Deserialize, Serialize};

use crate::input::{ClickKind, MouseButton};

pub const PROFILE_SCHEMA_VERSION: u32 = 1;
pub const SETTINGS_SCHEMA_VERSION: u32 = 1;

pub const INTERVAL_MS_MIN: u32 = 1;
pub const INTERVAL_MS_MAX: u32 = 60_000;
pub const VARIATION_BLOCKS_MAX: usize = 32;
pub const STOP_AFTER_MIN_SECONDS: u32 = 30;
pub const COUNTDOWN_MAX_SECONDS: u8 = 10;
pub const POINTS_MAX: usize = 64;
pub const DELAY_MS_MAX: u32 = 60_000;
pub const LINE_SPACING_MIN: u32 = 8;
pub const LINE_SPACING_MAX: u32 = 130;
pub const SCAN_AREA_MIN: u32 = 16;
pub const SCAN_AREA_MAX: u32 = 32_768;
pub const COORD_MAX: i32 = 100_000;
pub const NAME_MAX_CHARS: usize = 48;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Click,
    Wiggle,
    Scan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum Position {
    Cursor,
    Points,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum ScanAction {
    Move,
    MoveClick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum Modifier {
    Ctrl,
    Alt,
    Shift,
    Win,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Hotkey {
    pub mods: Vec<Modifier>,
    pub key: String,
}

impl Hotkey {
    pub fn new(mods: &[Modifier], key: &str) -> Self {
        let mut mods = mods.to_vec();
        mods.sort();
        mods.dedup();
        Self {
            mods,
            key: key.to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Variation {
    pub enabled: bool,
    pub blocks_ms: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ClickSettings {
    pub button: MouseButton,
    pub kind: ClickKind,
    pub interval_ms: u32,
    pub variation: Variation,
}

impl ClickSettings {
    pub fn intervals_ms(&self) -> Vec<u32> {
        if self.variation.enabled && !self.variation.blocks_ms.is_empty() {
            self.variation.blocks_ms.clone()
        } else {
            vec![self.interval_ms]
        }
    }

    pub fn min_interval_ms(&self) -> u32 {
        self.intervals_ms()
            .into_iter()
            .min()
            .unwrap_or(self.interval_ms)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct StopAfter {
    pub enabled: bool,
    pub seconds: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ScreenPoint {
    pub id: String,
    pub x: i32,
    pub y: i32,
    pub delay_ms: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ClickMode {
    pub position: Position,
    pub points: Vec<ScreenPoint>,
    pub layout_fingerprint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct WiggleMode {
    pub amplitude_pct: f64,
    pub speed_hz: u8,
    pub click_while_wiggling: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Area {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct ScanMode {
    pub area: Option<Area>,
    pub line_spacing_px: u32,
    pub speed_px_s: u32,
    pub action: ScanAction,
    #[serde(rename = "loop")]
    pub loop_: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Modes {
    pub click: ClickMode,
    pub wiggle: WiggleMode,
    pub scan: ScanMode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub mode: Mode,
    pub hotkey: Option<Hotkey>,
    pub created_at: String,
    pub updated_at: String,
    pub click: ClickSettings,
    pub stop_after: StopAfter,
    pub modes: Modes,
}

impl Profile {
    pub fn new_default(id: impl Into<String>, name: impl Into<String>, now_iso: &str) -> Self {
        Self {
            schema_version: PROFILE_SCHEMA_VERSION,
            id: id.into(),
            name: name.into(),
            mode: Mode::Click,
            hotkey: None,
            created_at: now_iso.to_owned(),
            updated_at: now_iso.to_owned(),
            click: ClickSettings {
                button: MouseButton::Left,
                kind: ClickKind::Single,
                interval_ms: 100,
                variation: Variation {
                    enabled: false,
                    blocks_ms: vec![1, 35, 70],
                },
            },
            stop_after: StopAfter {
                enabled: false,
                seconds: 300,
            },
            modes: Modes {
                click: ClickMode {
                    position: Position::Cursor,
                    points: Vec::new(),
                    layout_fingerprint: None,
                },
                wiggle: WiggleMode {
                    amplitude_pct: 0.05,
                    speed_hz: 8,
                    click_while_wiggling: false,
                },
                scan: ScanMode {
                    area: None,
                    line_spacing_px: 24,
                    speed_px_s: 1200,
                    action: ScanAction::Move,
                    loop_: true,
                },
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum Locale {
    #[serde(rename = "pt-BR")]
    PtBr,
    #[serde(rename = "en-US")]
    EnUs,
    #[serde(rename = "es-419")]
    Es419,
}

impl Locale {
    pub fn from_tag(tag: &str) -> Option<Self> {
        match tag.trim() {
            "pt-BR" => Some(Self::PtBr),
            "en-US" => Some(Self::EnUs),
            "es-419" => Some(Self::Es419),
            _ => None,
        }
    }

    pub fn from_primary_language(primary: u16) -> Self {
        match primary {
            0x16 => Self::PtBr,
            0x0A => Self::Es419,
            _ => Self::EnUs,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum ThemeId {
    Grafite,
    Blueprint,
    Fosforo,
    Bancada,
    Papel,
    Contraste,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum CloseAction {
    Minimize,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum InputBackendKind {
    Sendinput,
    Interception,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct HotkeyBindings {
    pub start_stop: Option<Hotkey>,
    pub pause_resume: Option<Hotkey>,
    pub capture_point: Option<Hotkey>,
    pub next_profile: Option<Hotkey>,
    pub emergency_stop: Hotkey,
}

impl Default for HotkeyBindings {
    fn default() -> Self {
        use Modifier::{Alt, Ctrl, Shift};
        Self {
            start_stop: Some(Hotkey::new(&[], "F6")),
            pause_resume: Some(Hotkey::new(&[], "F7")),
            capture_point: Some(Hotkey::new(&[Ctrl, Shift], "P")),
            next_profile: Some(Hotkey::new(&[Ctrl, Alt], "Right")),
            emergency_stop: Hotkey::new(&[Ctrl, Alt], "Escape"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct WindowPosition {
    pub x: Option<i32>,
    pub y: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub schema_version: u32,
    pub locale: Locale,
    pub theme: ThemeId,
    pub follow_system_theme: bool,
    pub autostart: bool,
    pub minimize_to_tray: bool,
    pub show_hud: bool,
    pub close_action: CloseAction,
    pub countdown_sec: u8,
    pub input_backend: InputBackendKind,
    pub active_profile_id: Option<String>,
    pub profile_order: Vec<String>,
    pub hotkeys: HotkeyBindings,
    pub hud: WindowPosition,
    pub window: WindowPosition,
    #[serde(default)]
    pub anti_cheat_acknowledged: bool,
    #[serde(default)]
    pub run_as_admin: bool,
}

impl Settings {
    pub fn merge_user_edit(&self, incoming: Self) -> Self {
        Self {
            active_profile_id: self.active_profile_id.clone(),
            profile_order: self.profile_order.clone(),
            hotkeys: self.hotkeys.clone(),
            hud: self.hud,
            window: self.window,
            anti_cheat_acknowledged: self.anti_cheat_acknowledged
                || incoming.anti_cheat_acknowledged,
            ..incoming
        }
    }

    pub fn new_default(locale: Locale) -> Self {
        Self {
            schema_version: SETTINGS_SCHEMA_VERSION,
            locale,
            theme: ThemeId::Grafite,
            follow_system_theme: false,
            autostart: false,
            minimize_to_tray: true,
            show_hud: true,
            close_action: CloseAction::Minimize,
            countdown_sec: 3,
            input_backend: InputBackendKind::Sendinput,
            active_profile_id: None,
            profile_order: Vec::new(),
            hotkeys: HotkeyBindings::default(),
            anti_cheat_acknowledged: false,
            run_as_admin: false,
            hud: WindowPosition::default(),
            window: WindowPosition::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn profile_serializes_to_the_documented_shape() {
        let p = Profile::new_default("p1", "Café run", "2026-10-04T20:00:00Z");
        let v = serde_json::to_value(&p).expect("json");
        assert_eq!(v["schemaVersion"], 1);
        assert_eq!(v["mode"], "click");
        assert_eq!(v["click"]["button"], "left");
        assert_eq!(v["click"]["kind"], "single");
        assert_eq!(v["click"]["intervalMs"], 100);
        assert_eq!(
            v["click"]["variation"],
            json!({ "enabled": false, "blocksMs": [1, 35, 70] })
        );
        assert_eq!(v["stopAfter"], json!({ "enabled": false, "seconds": 300 }));
        assert_eq!(v["modes"]["click"]["position"], "cursor");
        assert_eq!(v["modes"]["scan"]["action"], "move");
        assert_eq!(v["modes"]["scan"]["loop"], true);
        assert_eq!(v["modes"]["wiggle"]["speedHz"], 8);
        let back: Profile = serde_json::from_value(v).expect("round trip");
        assert_eq!(back, p);
    }

    #[test]
    fn side_buttons_and_scan_actions_use_the_documented_names() {
        assert_eq!(serde_json::to_value(MouseButton::X2).expect("json"), "x2");
        assert_eq!(
            serde_json::to_value(ScanAction::MoveClick).expect("json"),
            "move_click"
        );
        assert_eq!(serde_json::to_value(Locale::PtBr).expect("json"), "pt-BR");
        assert_eq!(serde_json::to_value(Locale::Es419).expect("json"), "es-419");
    }

    #[test]
    fn user_edits_never_overwrite_fields_owned_by_rust() {
        let mut current = Settings::new_default(Locale::PtBr);
        current.active_profile_id = Some("a".into());
        current.profile_order = vec!["a".into(), "b".into()];
        current.hud = WindowPosition {
            x: Some(10),
            y: Some(20),
        };
        current.anti_cheat_acknowledged = true;
        current.hotkeys.start_stop = Some(Hotkey::new(&[], "F9"));
        let mut stale = Settings::new_default(Locale::PtBr);
        stale.countdown_sec = 7;
        stale.locale = Locale::EnUs;
        let merged = current.merge_user_edit(stale);
        assert_eq!(
            (merged.countdown_sec, merged.locale),
            (7, Locale::EnUs),
            "the edit applies"
        );
        assert_eq!(merged.active_profile_id.as_deref(), Some("a"));
        assert_eq!(merged.profile_order, ["a", "b"]);
        assert_eq!(merged.hud, current.hud);
        assert!(merged.anti_cheat_acknowledged);
        assert_eq!(merged.hotkeys, current.hotkeys);
        assert_eq!(Locale::from_tag(" es-419 "), Some(Locale::Es419));
        assert_eq!(Locale::from_tag("pt-BR"), Some(Locale::PtBr));
        assert_eq!(Locale::from_tag("en-US"), Some(Locale::EnUs));
        assert_eq!(Locale::from_tag("fr-FR"), None);
        assert_eq!(Locale::from_primary_language(0x16), Locale::PtBr);
        assert_eq!(Locale::from_primary_language(0x0A), Locale::Es419);
        assert_eq!(Locale::from_primary_language(0x09), Locale::EnUs);
        assert_eq!(Locale::from_primary_language(0x07), Locale::EnUs);
    }

    #[test]
    fn settings_saved_with_the_old_hud_hint_counter_still_load() {
        let mut v = serde_json::to_value(Settings::new_default(Locale::PtBr)).expect("json");
        v["hudHintsShown"] = json!(3);
        let s: Settings = serde_json::from_value(v).expect("unknown fields are ignored");
        assert_eq!(s, Settings::new_default(Locale::PtBr));
    }

    #[test]
    fn settings_defaults_match_the_handoff() {
        let s = Settings::new_default(Locale::PtBr);
        let v = serde_json::to_value(&s).expect("json");
        assert_eq!(
            v["hotkeys"]["startStop"],
            json!({ "mods": [], "key": "F6" })
        );
        assert_eq!(
            v["hotkeys"]["capturePoint"],
            json!({ "mods": ["ctrl", "shift"], "key": "P" })
        );
        assert_eq!(
            v["hotkeys"]["emergencyStop"],
            json!({ "mods": ["ctrl", "alt"], "key": "Escape" })
        );
        assert_eq!(v["countdownSec"], 3);
        assert_eq!(v["closeAction"], "minimize");
        assert_eq!(v["theme"], "grafite");
        assert_eq!(v["runAsAdmin"], false);
        assert_eq!(
            serde_json::from_value::<Settings>(v).expect("round trip"),
            s
        );
    }

    #[test]
    fn hotkey_modifiers_are_normalized() {
        use Modifier::{Alt, Ctrl};
        assert_eq!(Hotkey::new(&[Alt, Ctrl, Alt], "P").mods, vec![Ctrl, Alt]);
    }

    #[test]
    fn intervals_follow_the_variation_switch() {
        let mut c = Profile::new_default("p", "n", "t").click;
        assert_eq!(c.intervals_ms(), vec![100]);
        c.variation.enabled = true;
        assert_eq!(c.intervals_ms(), vec![1, 35, 70]);
        assert_eq!(c.min_interval_ms(), 1);
        c.variation.blocks_ms.clear();
        assert_eq!(c.intervals_ms(), vec![100]);
    }
}
