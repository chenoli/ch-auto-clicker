use ch_core::model::Modifier;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN,
};

const NAMED: [(&str, u16); 34] = [
    ("Backspace", 0x08),
    ("Tab", 0x09),
    ("Enter", 0x0D),
    ("Pause", 0x13),
    ("Escape", 0x1B),
    ("Space", 0x20),
    ("PageUp", 0x21),
    ("PageDown", 0x22),
    ("End", 0x23),
    ("Home", 0x24),
    ("Left", 0x25),
    ("Up", 0x26),
    ("Right", 0x27),
    ("Down", 0x28),
    ("PrintScreen", 0x2C),
    ("Insert", 0x2D),
    ("Delete", 0x2E),
    ("ScrollLock", 0x91),
    ("Semicolon", 0xBA),
    ("Equal", 0xBB),
    ("Comma", 0xBC),
    ("Minus", 0xBD),
    ("Period", 0xBE),
    ("Slash", 0xBF),
    ("Backquote", 0xC0),
    ("Quote", 0xDE),
    ("Shift", 0x10),
    ("Control", 0x11),
    ("Alt", 0x12),
    ("LShift", 0xA0),
    ("RShift", 0xA1),
    ("LControl", 0xA2),
    ("RControl", 0xA3),
    ("LAlt", 0xA4),
];

const EXTRA: [(&str, u16); 3] = [("RAlt", 0xA5), ("LWin", 0x5B), ("RWin", 0x5C)];

pub fn vk_from_name(name: &str) -> Option<u16> {
    if let Some(&(_, vk)) = NAMED.iter().chain(EXTRA.iter()).find(|(n, _)| *n == name) {
        return Some(vk);
    }
    let mut chars = name.chars();
    if let (Some(c), None) = (chars.next(), chars.next())
        && (c.is_ascii_uppercase() || c.is_ascii_digit())
    {
        return Some(c as u16);
    }
    if let Some(n) = name.strip_prefix('F').and_then(|d| d.parse::<u16>().ok())
        && (1..=24).contains(&n)
    {
        return Some(0x70 + n - 1);
    }
    if let Some(d) = name.strip_prefix("Numpad")
        && d.len() == 1
        && let Some(n) = d.chars().next().and_then(|c| c.to_digit(10))
    {
        return Some(0x60 + n as u16);
    }
    None
}

pub fn name_from_vk(vk: u16) -> Option<String> {
    if let Some(&(name, _)) = NAMED.iter().chain(EXTRA.iter()).find(|(_, v)| *v == vk) {
        return Some(name.to_owned());
    }
    match vk {
        0x30..=0x39 | 0x41..=0x5A => Some(char::from(vk as u8).to_string()),
        0x60..=0x69 => Some(format!("Numpad{}", vk - 0x60)),
        0x70..=0x87 => Some(format!("F{}", vk - 0x70 + 1)),
        _ => None,
    }
}

pub fn hotkey_modifiers(mods: &[Modifier]) -> HOT_KEY_MODIFIERS {
    mods.iter().fold(HOT_KEY_MODIFIERS(0), |acc, m| {
        acc | match m {
            Modifier::Ctrl => MOD_CONTROL,
            Modifier::Alt => MOD_ALT,
            Modifier::Shift => MOD_SHIFT,
            Modifier::Win => MOD_WIN,
        }
    })
}

#[cfg(test)]
mod tests {
    use ch_core::hotkeys::is_known_key;

    use super::*;

    #[test]
    fn every_known_key_round_trips() {
        let mut names: Vec<String> = ('A'..='Z').chain('0'..='9').map(String::from).collect();
        names.extend((1..=24).map(|n| format!("F{n}")));
        names.extend((0..=9).map(|n| format!("Numpad{n}")));
        names.extend(
            NAMED
                .iter()
                .chain(EXTRA.iter())
                .map(|(n, _)| (*n).to_owned()),
        );
        for name in names {
            let vk = vk_from_name(&name).unwrap_or_else(|| panic!("no vk for {name}"));
            assert_eq!(
                name_from_vk(vk).as_deref(),
                Some(name.as_str()),
                "vk {vk:#x}"
            );
        }
    }

    #[test]
    fn known_core_keys_all_have_codes() {
        for name in [
            "F6",
            "Escape",
            "Right",
            "P",
            "Numpad0",
            "Backquote",
            "PrintScreen",
        ] {
            assert!(is_known_key(name));
            assert!(vk_from_name(name).is_some(), "{name}");
        }
        assert_eq!(vk_from_name("F25"), None);
        assert_eq!(vk_from_name("Numpad10"), None);
        assert_eq!(vk_from_name("p"), None);
        assert_eq!(name_from_vk(0xFF), None);
    }

    #[test]
    fn builds_register_hotkey_modifiers() {
        assert_eq!(
            hotkey_modifiers(&[Modifier::Ctrl, Modifier::Alt]),
            MOD_CONTROL | MOD_ALT
        );
        assert_eq!(
            hotkey_modifiers(&[Modifier::Shift, Modifier::Win]),
            MOD_SHIFT | MOD_WIN
        );
        assert_eq!(hotkey_modifiers(&[]), HOT_KEY_MODIFIERS(0));
    }
}
