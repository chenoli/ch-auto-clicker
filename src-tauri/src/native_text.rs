use ch_core::model::Locale;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Text {
    CaptureTitle,
    VariationTitle,
    ProfileFileType,
    StateIdle,
    StateCountdown,
    StateRunning,
    StatePaused,
    StateError,
    Start,
    Stop,
    Pause,
    Resume,
    Profile,
    ShowWindow,
    ShowHud,
    ResetHudPosition,
    Settings,
    Support,
    Quit,
    HotkeyTakenBody,
    DefaultProfileName,
}

pub const fn text(locale: Locale, key: Text) -> &'static str {
    use Locale::{EnUs, Es419, PtBr};
    use Text::*;
    match (locale, key) {
        (PtBr, CaptureTitle) => "CH. Clicker — Captura",
        (EnUs, CaptureTitle) => "CH. Clicker — Capture",
        (Es419, CaptureTitle) => "CH. Clicker — Captura",
        (PtBr, VariationTitle) => "CH. Clicker — Variação de intervalo",
        (EnUs, VariationTitle) => "CH. Clicker — Interval variation",
        (Es419, VariationTitle) => "CH. Clicker — Variación de intervalo",
        (PtBr, ProfileFileType) => "Perfil do CH. Clicker",
        (EnUs, ProfileFileType) => "CH. Clicker profile",
        (Es419, ProfileFileType) => "Perfil de CH. Clicker",
        (PtBr, StateIdle) => "PARADO",
        (EnUs, StateIdle) => "STOPPED",
        (Es419, StateIdle) => "DETENIDO",
        (PtBr, StateCountdown) => "INICIANDO",
        (EnUs, StateCountdown) => "STARTING",
        (Es419, StateCountdown) => "INICIANDO",
        (PtBr, StateRunning) => "RODANDO",
        (EnUs, StateRunning) => "RUNNING",
        (Es419, StateRunning) => "EN EJECUCIÓN",
        (PtBr, StatePaused) => "PAUSADO",
        (EnUs, StatePaused) => "PAUSED",
        (Es419, StatePaused) => "EN PAUSA",
        (PtBr, StateError) => "ERRO",
        (EnUs, StateError) => "ERROR",
        (Es419, StateError) => "ERROR",
        (PtBr, Start) => "Iniciar",
        (EnUs, Start) => "Start",
        (Es419, Start) => "Iniciar",
        (PtBr, Stop) => "Parar",
        (EnUs, Stop) => "Stop",
        (Es419, Stop) => "Detener",
        (PtBr, Pause) => "Pausar",
        (EnUs, Pause) => "Pause",
        (Es419, Pause) => "Pausar",
        (PtBr, Resume) => "Retomar",
        (EnUs, Resume) => "Resume",
        (Es419, Resume) => "Reanudar",
        (PtBr, Profile) => "Perfil",
        (EnUs, Profile) => "Profile",
        (Es419, Profile) => "Perfil",
        (PtBr, ShowWindow) => "Mostrar janela",
        (EnUs, ShowWindow) => "Show window",
        (Es419, ShowWindow) => "Mostrar ventana",
        (PtBr, ShowHud) => "Mostrar HUD",
        (EnUs, ShowHud) => "Show HUD",
        (Es419, ShowHud) => "Mostrar HUD",
        (PtBr, ResetHudPosition) => "Restaurar posição do HUD",
        (EnUs, ResetHudPosition) => "Reset HUD position",
        (Es419, ResetHudPosition) => "Restaurar posición del HUD",
        (PtBr, Settings) => "Configurações",
        (EnUs, Settings) => "Settings",
        (Es419, Settings) => "Configuración",
        (PtBr, Support) => "Me pague um café ↗",
        (EnUs, Support) => "Buy me a coffee ↗",
        (Es419, Support) => "Invítame un café ↗",
        (PtBr, Quit) => "Sair",
        (EnUs, Quit) => "Quit",
        (Es419, Quit) => "Salir",
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

    const ALL: [Text; 21] = [
        Text::CaptureTitle,
        Text::VariationTitle,
        Text::ProfileFileType,
        Text::StateIdle,
        Text::StateCountdown,
        Text::StateRunning,
        Text::StatePaused,
        Text::StateError,
        Text::Start,
        Text::Stop,
        Text::Pause,
        Text::Resume,
        Text::Profile,
        Text::ShowWindow,
        Text::ShowHud,
        Text::ResetHudPosition,
        Text::Settings,
        Text::Support,
        Text::Quit,
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
        assert!(text(Locale::PtBr, Text::CaptureTitle).contains("CH. Clicker"));
        assert_eq!(
            text(Locale::EnUs, Text::VariationTitle),
            "CH. Clicker — Interval variation"
        );
        assert_eq!(text(Locale::EnUs, Text::DefaultProfileName), "My profile");
    }
}
