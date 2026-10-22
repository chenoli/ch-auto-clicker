use ch_core::model::Locale;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Text {
    CaptureTitle,
    VariationTitle,
    ProfileFileType,
    HotkeyTakenBody,
    DefaultProfileName,
}

pub const fn text(locale: Locale, key: Text) -> &'static str {
    use Locale::{EnUs, Es419, PtBr};
    use Text::*;
    match (locale, key) {
        (PtBr, CaptureTitle) => "CH Auto Clicker — Captura",
        (EnUs, CaptureTitle) => "CH Auto Clicker — Capture",
        (Es419, CaptureTitle) => "CH Auto Clicker — Captura",
        (PtBr, VariationTitle) => "CH Auto Clicker — Variação de intervalo",
        (EnUs, VariationTitle) => "CH Auto Clicker — Interval variation",
        (Es419, VariationTitle) => "CH Auto Clicker — Variación de intervalo",
        (PtBr, ProfileFileType) => "Perfil do CH Auto Clicker",
        (EnUs, ProfileFileType) => "CH Auto Clicker profile",
        (Es419, ProfileFileType) => "Perfil de CH Auto Clicker",
        (PtBr, HotkeyTakenBody) => {
            "Outro aplicativo já usa este atalho global. Iniciar / Parar está sem atalho até você escolher outro."
        }
        (EnUs, HotkeyTakenBody) => {
            "Another app already uses this global shortcut. Start / Stop has no shortcut until you choose another one."
        }
        (Es419, HotkeyTakenBody) => {
            "Otra aplicación ya usa este atajo global. Iniciar / Detener no tiene atajo hasta que elijas otro."
        }
        (PtBr, DefaultProfileName) => "Meu perfil",
        (EnUs, DefaultProfileName) => "My profile",
        (Es419, DefaultProfileName) => "Mi perfil",
    }
}

pub fn localize_default_names(names: &[&str], locale: Locale) -> Vec<usize> {
    let target = text(locale, Text::DefaultProfileName);
    if names.iter().any(|n| n.eq_ignore_ascii_case(target)) {
        return Vec::new();
    }
    let defaults =
        [Locale::PtBr, Locale::EnUs, Locale::Es419].map(|l| text(l, Text::DefaultProfileName));
    names
        .iter()
        .position(|n| defaults.contains(n))
        .into_iter()
        .collect()
}

pub fn hotkey_taken_title(locale: Locale, combo: &str) -> String {
    match locale {
        Locale::PtBr => format!("{combo} não pôde ser registrado"),
        Locale::EnUs => format!("{combo} could not be registered"),
        Locale::Es419 => format!("{combo} no se pudo registrar"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Text; 5] = [
        Text::CaptureTitle,
        Text::VariationTitle,
        Text::ProfileFileType,
        Text::HotkeyTakenBody,
        Text::DefaultProfileName,
    ];

    #[test]
    fn every_locale_has_every_text() {
        for locale in [Locale::PtBr, Locale::EnUs, Locale::Es419] {
            for key in ALL {
                assert!(!text(locale, key).trim().is_empty(), "{locale:?} {key:?}");
            }
            assert!(hotkey_taken_title(locale, "F6").starts_with("F6 "));
        }
        assert!(text(Locale::PtBr, Text::CaptureTitle).contains("CH Auto Clicker"));
        assert_eq!(
            text(Locale::EnUs, Text::VariationTitle),
            "CH Auto Clicker — Interval variation"
        );
        assert_eq!(text(Locale::EnUs, Text::DefaultProfileName), "My profile");
    }

    #[test]
    fn default_profile_names_follow_the_language() {
        assert_eq!(
            localize_default_names(&["Mi perfil", "Farm"], Locale::PtBr),
            vec![0]
        );
        assert_eq!(
            localize_default_names(&["Farm", "Meu perfil", "My profile"], Locale::EnUs),
            Vec::<usize>::new()
        );
        assert_eq!(
            localize_default_names(&["Meu perfil (2)", "Farm"], Locale::Es419),
            Vec::<usize>::new()
        );
        assert_eq!(
            localize_default_names(&["Meu perfil", "My profile"], Locale::Es419),
            vec![0]
        );
    }
}
