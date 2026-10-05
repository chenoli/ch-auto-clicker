//! Text that Rust draws itself (window titles, file dialogs; the tray in Phase 11), in the UI
//! language (TASKS T10.3). The web UI has its own catalogs under `ui/src/i18n/locales`.

use ch_core::model::Locale;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Text {
    /// Title of the capture overlays (Telas 03.A/03.B).
    CaptureTitle,
    /// Title of the variation editor window (Tela 02.B).
    VariationTitle,
    /// File type name in the export/import dialogs.
    ProfileFileType,
}

pub const fn text(locale: Locale, key: Text) -> &'static str {
    match (locale, key) {
        (Locale::PtBr, Text::CaptureTitle) => "CH. Clicker — Captura",
        (Locale::EnUs, Text::CaptureTitle) => "CH. Clicker — Capture",
        (Locale::Es419, Text::CaptureTitle) => "CH. Clicker — Captura",
        (Locale::PtBr, Text::VariationTitle) => "CH. Clicker — Variação de intervalo",
        (Locale::EnUs, Text::VariationTitle) => "CH. Clicker — Interval variation",
        (Locale::Es419, Text::VariationTitle) => "CH. Clicker — Variación de intervalo",
        (Locale::PtBr, Text::ProfileFileType) => "Perfil do CH. Clicker",
        (Locale::EnUs, Text::ProfileFileType) => "CH. Clicker profile",
        (Locale::Es419, Text::ProfileFileType) => "Perfil de CH. Clicker",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_locale_has_every_text() {
        for locale in [Locale::PtBr, Locale::EnUs, Locale::Es419] {
            for key in [
                Text::CaptureTitle,
                Text::VariationTitle,
                Text::ProfileFileType,
            ] {
                assert!(
                    text(locale, key).contains("CH. Clicker"),
                    "{locale:?} {key:?}"
                );
            }
        }
        assert_eq!(
            text(Locale::EnUs, Text::VariationTitle),
            "CH. Clicker — Interval variation"
        );
    }
}
