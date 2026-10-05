use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::model::{Hotkey, HotkeyBindings, Modifier, Profile};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum HotkeyAction {
    StartStop,
    PauseResume,
    CapturePoint,
    NextProfile,
    EmergencyStop,
}

impl HotkeyAction {
    pub const ALL: [HotkeyAction; 5] = [
        Self::StartStop,
        Self::PauseResume,
        Self::CapturePoint,
        Self::NextProfile,
        Self::EmergencyStop,
    ];
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase", tag = "kind", content = "id")]
pub enum HotkeyTarget {
    Action(HotkeyAction),
    Profile(String),
}

pub const MOUSE_KEYS: [&str; 3] = ["MouseMiddle", "MouseX1", "MouseX2"];

pub fn is_known_key(key: &str) -> bool {
    if MOUSE_KEYS.contains(&key) || NAMED_KEYS.contains(&key) {
        return true;
    }
    let mut chars = key.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => c.is_ascii_uppercase() || c.is_ascii_digit(),
        (Some('F'), Some(_)) => key[1..].parse::<u8>().is_ok_and(|n| (1..=24).contains(&n)),
        _ => key
            .strip_prefix("Numpad")
            .is_some_and(|d| d.len() == 1 && d.chars().all(|c| c.is_ascii_digit())),
    }
}

const NAMED_KEYS: [&str; 26] = [
    "Escape",
    "Tab",
    "Space",
    "Enter",
    "Backspace",
    "Insert",
    "Delete",
    "Home",
    "End",
    "PageUp",
    "PageDown",
    "Left",
    "Right",
    "Up",
    "Down",
    "PrintScreen",
    "Pause",
    "ScrollLock",
    "Minus",
    "Equal",
    "Comma",
    "Period",
    "Slash",
    "Semicolon",
    "Quote",
    "Backquote",
];

pub fn is_mouse_key(key: &str) -> bool {
    MOUSE_KEYS.contains(&key)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum HotkeyError {
    #[error("unknown key")]
    UnknownKey,
    #[error("the left and right mouse buttons cannot be shortcuts")]
    PrimaryMouseButton,
    #[error("the emergency stop cannot be disabled")]
    EmergencyRequired,
}

impl HotkeyError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnknownKey => "HOTKEY_UNKNOWN_KEY",
            Self::PrimaryMouseButton => "HOTKEY_PRIMARY_MOUSE",
            Self::EmergencyRequired => "HOTKEY_EMERGENCY_REQUIRED",
        }
    }
}

pub fn normalize(hotkey: &Hotkey) -> Result<Hotkey, HotkeyError> {
    if matches!(hotkey.key.as_str(), "MouseLeft" | "MouseRight") {
        return Err(HotkeyError::PrimaryMouseButton);
    }
    if !is_known_key(&hotkey.key) {
        return Err(HotkeyError::UnknownKey);
    }
    Ok(Hotkey::new(&hotkey.mods, &hotkey.key))
}

pub fn all_bindings(
    bindings: &HotkeyBindings,
    profiles: &[Profile],
) -> Vec<(HotkeyTarget, Hotkey)> {
    let mut out: Vec<(HotkeyTarget, Hotkey)> = HotkeyAction::ALL
        .iter()
        .filter_map(|&a| binding_of(bindings, a).map(|h| (HotkeyTarget::Action(a), h.clone())))
        .collect();
    out.extend(profiles.iter().filter_map(|p| {
        p.hotkey
            .clone()
            .map(|h| (HotkeyTarget::Profile(p.id.clone()), h))
    }));
    out
}

pub fn binding_of(bindings: &HotkeyBindings, action: HotkeyAction) -> Option<&Hotkey> {
    match action {
        HotkeyAction::StartStop => bindings.start_stop.as_ref(),
        HotkeyAction::PauseResume => bindings.pause_resume.as_ref(),
        HotkeyAction::CapturePoint => bindings.capture_point.as_ref(),
        HotkeyAction::NextProfile => bindings.next_profile.as_ref(),
        HotkeyAction::EmergencyStop => Some(&bindings.emergency_stop),
    }
}

pub fn set_binding(
    bindings: &mut HotkeyBindings,
    action: HotkeyAction,
    hotkey: Option<Hotkey>,
) -> Result<(), HotkeyError> {
    match action {
        HotkeyAction::StartStop => bindings.start_stop = hotkey,
        HotkeyAction::PauseResume => bindings.pause_resume = hotkey,
        HotkeyAction::CapturePoint => bindings.capture_point = hotkey,
        HotkeyAction::NextProfile => bindings.next_profile = hotkey,
        HotkeyAction::EmergencyStop => {
            bindings.emergency_stop = hotkey.ok_or(HotkeyError::EmergencyRequired)?
        }
    }
    Ok(())
}

pub fn find_conflict(
    bindings: &HotkeyBindings,
    profiles: &[Profile],
    target: &HotkeyTarget,
    combo: &Hotkey,
) -> Option<HotkeyTarget> {
    all_bindings(bindings, profiles)
        .into_iter()
        .find(|(t, h)| t != target && h == combo)
        .map(|(t, _)| t)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordStep {
    Partial(Vec<Modifier>),
    Done(Hotkey),
    Cancelled,
}

#[derive(Debug, Default)]
pub struct Recorder {
    held: BTreeSet<Modifier>,
}

impl Recorder {
    pub fn key_down(&mut self, key: &str) -> RecordStep {
        if let Some(m) = modifier_of(key) {
            self.held.insert(m);
            return RecordStep::Partial(self.held.iter().copied().collect());
        }
        if key == "Escape" && self.held.is_empty() {
            return RecordStep::Cancelled;
        }
        if !is_known_key(key) {
            return RecordStep::Partial(self.held.iter().copied().collect());
        }
        let mods: Vec<Modifier> = self.held.iter().copied().collect();
        self.held.clear();
        RecordStep::Done(Hotkey::new(&mods, key))
    }

    pub fn key_up(&mut self, key: &str) -> RecordStep {
        if let Some(m) = modifier_of(key) {
            self.held.remove(&m);
        }
        RecordStep::Partial(self.held.iter().copied().collect())
    }
}

pub fn modifier_of(key: &str) -> Option<Modifier> {
    match key {
        "Control" | "LControl" | "RControl" => Some(Modifier::Ctrl),
        "Alt" | "LAlt" | "RAlt" => Some(Modifier::Alt),
        "Shift" | "LShift" | "RShift" => Some(Modifier::Shift),
        "LWin" | "RWin" => Some(Modifier::Win),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Modifier::{Alt, Ctrl, Shift};

    fn hk(mods: &[Modifier], key: &str) -> Hotkey {
        Hotkey::new(mods, key)
    }

    #[test]
    fn knows_the_supported_keys() {
        for key in [
            "A",
            "Z",
            "0",
            "9",
            "F1",
            "F24",
            "Escape",
            "Right",
            "Numpad7",
            "MouseX1",
            "Backquote",
        ] {
            assert!(is_known_key(key), "{key}");
        }
        for key in [
            "a",
            "F0",
            "F25",
            "Fx",
            "Numpad",
            "Numpad12",
            "Hyper",
            "",
            "MouseLeft",
        ] {
            assert!(!is_known_key(key), "{key}");
        }
        assert!(is_mouse_key("MouseMiddle"));
        assert!(!is_mouse_key("F6"));
    }

    #[test]
    fn normalizes_and_rejects_bad_keys() {
        assert_eq!(
            normalize(&Hotkey {
                mods: vec![Shift, Ctrl, Shift],
                key: "P".into()
            }),
            Ok(hk(&[Ctrl, Shift], "P"))
        );
        assert_eq!(
            normalize(&hk(&[], "MouseLeft")),
            Err(HotkeyError::PrimaryMouseButton)
        );
        assert_eq!(normalize(&hk(&[], "Banana")), Err(HotkeyError::UnknownKey));
        assert_eq!(HotkeyError::UnknownKey.code(), "HOTKEY_UNKNOWN_KEY");
        assert_eq!(
            HotkeyError::PrimaryMouseButton.code(),
            "HOTKEY_PRIMARY_MOUSE"
        );
        assert_eq!(
            HotkeyError::EmergencyRequired.code(),
            "HOTKEY_EMERGENCY_REQUIRED"
        );
    }

    #[test]
    fn lists_actions_then_profiles() {
        let mut p = Profile::new_default("p1", "Farm", "t");
        p.hotkey = Some(hk(&[Ctrl, Alt], "1"));
        let all = all_bindings(&HotkeyBindings::default(), &[p]);
        assert_eq!(all.len(), 6);
        assert_eq!(
            all[0],
            (HotkeyTarget::Action(HotkeyAction::StartStop), hk(&[], "F6"))
        );
        assert_eq!(all[5].0, HotkeyTarget::Profile("p1".into()));
    }

    #[test]
    fn finds_conflicts_across_actions_and_profiles() {
        let bindings = HotkeyBindings::default();
        let mut p = Profile::new_default("p1", "Farm", "t");
        p.hotkey = Some(hk(&[Ctrl, Alt], "1"));
        let profiles = [p];
        let start = HotkeyTarget::Action(HotkeyAction::StartStop);
        assert_eq!(
            find_conflict(&bindings, &profiles, &start, &hk(&[], "F7")),
            Some(HotkeyTarget::Action(HotkeyAction::PauseResume))
        );
        assert_eq!(
            find_conflict(&bindings, &profiles, &start, &hk(&[Ctrl, Alt], "1")),
            Some(HotkeyTarget::Profile("p1".into()))
        );
        assert_eq!(
            find_conflict(&bindings, &profiles, &start, &hk(&[], "F6")),
            None,
            "its own combo"
        );
        assert_eq!(
            find_conflict(&bindings, &profiles, &start, &hk(&[], "F9")),
            None
        );
    }

    #[test]
    fn sets_and_clears_bindings_but_never_the_emergency_stop() {
        let mut b = HotkeyBindings::default();
        set_binding(&mut b, HotkeyAction::PauseResume, None).expect("clear");
        assert_eq!(binding_of(&b, HotkeyAction::PauseResume), None);
        for action in [
            HotkeyAction::StartStop,
            HotkeyAction::CapturePoint,
            HotkeyAction::NextProfile,
        ] {
            set_binding(&mut b, action, Some(hk(&[], "F9"))).expect("set");
            assert_eq!(binding_of(&b, action), Some(&hk(&[], "F9")));
        }
        assert_eq!(
            set_binding(&mut b, HotkeyAction::EmergencyStop, None),
            Err(HotkeyError::EmergencyRequired)
        );
        set_binding(
            &mut b,
            HotkeyAction::EmergencyStop,
            Some(hk(&[Ctrl, Shift], "Pause")),
        )
        .expect("set");
        assert_eq!(b.emergency_stop, hk(&[Ctrl, Shift], "Pause"));
    }

    #[test]
    fn recorder_follows_the_capture_rules() {
        let mut r = Recorder::default();
        assert_eq!(r.key_down("LControl"), RecordStep::Partial(vec![Ctrl]));
        assert_eq!(r.key_down("RShift"), RecordStep::Partial(vec![Ctrl, Shift]));
        assert_eq!(
            r.key_down("Unknown"),
            RecordStep::Partial(vec![Ctrl, Shift]),
            "unsupported keys are ignored"
        );
        assert_eq!(r.key_up("RShift"), RecordStep::Partial(vec![Ctrl]));
        assert_eq!(
            r.key_down("Escape"),
            RecordStep::Done(hk(&[Ctrl], "Escape")),
            "Esc with a modifier is a combo"
        );
        assert_eq!(
            r.key_down("Escape"),
            RecordStep::Cancelled,
            "Esc alone cancels"
        );
        assert_eq!(r.key_down("F6"), RecordStep::Done(hk(&[], "F6")));
        assert_eq!(r.key_down("LWin"), RecordStep::Partial(vec![Modifier::Win]));
        assert_eq!(
            r.key_down("MouseX2"),
            RecordStep::Done(hk(&[Modifier::Win], "MouseX2"))
        );
        assert_eq!(r.key_up("A"), RecordStep::Partial(vec![]));
    }

    #[test]
    fn maps_modifier_keys() {
        assert_eq!(modifier_of("Alt"), Some(Alt));
        assert_eq!(modifier_of("Control"), Some(Ctrl));
        assert_eq!(modifier_of("Shift"), Some(Shift));
        assert_eq!(modifier_of("Q"), None);
    }

    #[test]
    fn targets_serialize_with_a_tag() {
        let json =
            serde_json::to_value(HotkeyTarget::Action(HotkeyAction::NextProfile)).expect("json");
        assert_eq!(
            json,
            serde_json::json!({ "kind": "action", "id": "nextProfile" })
        );
        let json = serde_json::to_value(HotkeyTarget::Profile("p".into())).expect("json");
        assert_eq!(json, serde_json::json!({ "kind": "profile", "id": "p" }));
    }
}
