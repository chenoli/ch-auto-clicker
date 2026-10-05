//! `.chprofile` import/export (ARCHITECTURE §4, TASKS T8.2):
//! `{ "format": "chprofile", "schemaVersion": 1, "exportedBy": "...", "profile": { ... } }`.
//!
//! Imports are judged by their content, never by the file name: the envelope, the profile's schema
//! version (migrated when older) and every validation rule. The imported profile gets a new id, a
//! free name and loses a shortcut that another action or profile already uses.

use serde::Serialize;
use serde_json::{Value, json};

use crate::hotkeys::{HotkeyTarget, find_conflict};
use crate::model::{Hotkey, HotkeyBindings, Profile};
use crate::profiles::unique_name;
use crate::schema::{SchemaError, load_profile};

pub const FORMAT: &str = "chprofile";
pub const ENVELOPE_VERSION: u32 = 1;
pub const EXTENSION: &str = "chprofile";
/// Real profiles are a few KB; anything this big is not one.
pub const MAX_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ChprofileError {
    #[error("the file is larger than 1 MB")]
    TooLarge,
    #[error("the file is not JSON: {0}")]
    NotJson(String),
    #[error("the file is not a .chprofile")]
    NotChprofile,
    #[error("the file was made by a newer version (format {0})")]
    FutureFormat(u32),
    #[error(transparent)]
    Profile(#[from] SchemaError),
}

impl ChprofileError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::TooLarge => "IMPORT_TOO_LARGE",
            Self::NotJson(_) => "IMPORT_NOT_JSON",
            Self::NotChprofile => "IMPORT_NOT_CHPROFILE",
            Self::FutureFormat(_) => "IMPORT_FUTURE_VERSION",
            Self::Profile(SchemaError::FutureVersion { .. }) => "IMPORT_FUTURE_VERSION",
            Self::Profile(_) => "IMPORT_INVALID",
        }
    }
}

/// What the import had to change, so the UI can tell the user.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum ImportNote {
    /// The name was taken; the profile was renamed.
    Renamed { from: String, to: String },
    /// The shortcut was already used and was dropped.
    HotkeyDropped { combo: Hotkey },
    /// The file came from an older schema and was upgraded.
    Migrated { from_version: u32 },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Imported {
    pub profile: Profile,
    pub notes: Vec<ImportNote>,
}

/// The `.chprofile` text for a profile (pretty JSON, so people can read and diff it).
pub fn export(profile: &Profile, exported_by: &str) -> String {
    let doc = json!({
        "format": FORMAT,
        "schemaVersion": ENVELOPE_VERSION,
        "exportedBy": exported_by,
        "profile": profile,
    });
    let mut text = serde_json::to_string_pretty(&doc).unwrap_or_default();
    text.push('\n');
    text
}

/// A safe file name for the save dialog: the profile name without characters Windows forbids.
pub fn file_name(profile: &Profile) -> String {
    let stem: String = profile
        .name
        .chars()
        .map(|c| {
            if "<>:\"/\\|?*".contains(c) || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();
    let stem = stem.trim().trim_end_matches('.');
    format!(
        "{}.{EXTENSION}",
        if stem.is_empty() { "perfil" } else { stem }
    )
}

/// Parses and adapts an imported profile for this library.
pub fn import(
    text: &str,
    new_id: &str,
    now: &str,
    bindings: &HotkeyBindings,
    existing: &[Profile],
) -> Result<Imported, ChprofileError> {
    if text.len() > MAX_BYTES {
        return Err(ChprofileError::TooLarge);
    }
    let doc: Value = serde_json::from_str(text.trim_start_matches('\u{feff}'))
        .map_err(|e| ChprofileError::NotJson(e.to_string()))?;
    if doc.get("format").and_then(Value::as_str) != Some(FORMAT) {
        return Err(ChprofileError::NotChprofile);
    }
    let envelope = doc
        .get("schemaVersion")
        .and_then(Value::as_u64)
        .unwrap_or(1);
    if envelope > u64::from(ENVELOPE_VERSION) {
        return Err(ChprofileError::FutureFormat(
            u32::try_from(envelope).unwrap_or(u32::MAX),
        ));
    }
    let raw = doc
        .get("profile")
        .cloned()
        .ok_or(ChprofileError::NotChprofile)?;
    let loaded = load_profile(raw)?;
    let mut notes = Vec::new();
    if loaded.migrated(crate::model::PROFILE_SCHEMA_VERSION) {
        notes.push(ImportNote::Migrated {
            from_version: loaded.from_version,
        });
    }
    let mut profile = loaded.value;
    profile.id = new_id.to_owned();
    profile.created_at = now.to_owned();
    profile.updated_at = now.to_owned();

    let name = unique_name(&profile.name, existing, None);
    if name != profile.name {
        notes.push(ImportNote::Renamed {
            from: std::mem::replace(&mut profile.name, name.clone()),
            to: name,
        });
    }
    if let Some(combo) = profile.hotkey.clone()
        && find_conflict(
            bindings,
            existing,
            &HotkeyTarget::Profile(new_id.to_owned()),
            &combo,
        )
        .is_some()
    {
        profile.hotkey = None;
        notes.push(ImportNote::HotkeyDropped { combo });
    }
    Ok(Imported { profile, notes })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Modifier;

    const NOW: &str = "2026-10-05T12:00:00Z";

    fn sample() -> Profile {
        let mut p = Profile::new_default("orig", "Coleta rápida", "2026-01-01T00:00:00Z");
        p.click.interval_ms = 42;
        p
    }

    #[test]
    fn export_then_import_gives_an_equivalent_profile() {
        let text = export(&sample(), "CH. Clicker 0.1.0");
        assert!(text.contains("\"format\": \"chprofile\""));
        let imported = import(&text, "new", NOW, &HotkeyBindings::default(), &[]).expect("import");
        assert!(imported.notes.is_empty());
        let mut expected = sample();
        expected.id = "new".into();
        expected.created_at = NOW.into();
        expected.updated_at = NOW.into();
        assert_eq!(imported.profile, expected);
    }

    #[test]
    fn duplicated_names_and_taken_shortcuts_are_adjusted() {
        let mut p = sample();
        p.hotkey = Some(Hotkey::new(&[], "F6")); // the default Iniciar / Parar
        let text = export(&p, "x");
        let existing = [sample()];
        let imported =
            import(&text, "new", NOW, &HotkeyBindings::default(), &existing).expect("import");
        assert_eq!(imported.profile.name, "Coleta rápida (2)");
        assert_eq!(imported.profile.hotkey, None);
        assert_eq!(
            imported.notes,
            vec![
                ImportNote::Renamed {
                    from: "Coleta rápida".into(),
                    to: "Coleta rápida (2)".into()
                },
                ImportNote::HotkeyDropped {
                    combo: Hotkey::new(&[], "F6")
                },
            ]
        );
        let mut free = sample();
        free.hotkey = Some(Hotkey::new(&[Modifier::Ctrl], "F9"));
        let kept = import(
            &export(&free, "x"),
            "n2",
            NOW,
            &HotkeyBindings::default(),
            &[],
        )
        .expect("import");
        assert_eq!(kept.profile.hotkey, free.hotkey);
    }

    #[test]
    fn older_profiles_are_migrated() {
        let mut doc: Value = serde_json::from_str(&export(&sample(), "x")).expect("json");
        doc["profile"]
            .as_object_mut()
            .expect("profile")
            .remove("schemaVersion");
        let imported =
            import(&doc.to_string(), "n", NOW, &HotkeyBindings::default(), &[]).expect("import");
        assert_eq!(
            imported.notes,
            vec![ImportNote::Migrated { from_version: 0 }]
        );
    }

    #[test]
    fn rejects_files_by_content() {
        let b = HotkeyBindings::default();
        let err = |text: &str| {
            import(text, "n", NOW, &b, &[])
                .expect_err("must fail")
                .code()
        };
        assert_eq!(err("not json"), "IMPORT_NOT_JSON");
        assert_eq!(err("{\"format\":\"other\"}"), "IMPORT_NOT_CHPROFILE");
        assert_eq!(err("{\"format\":\"chprofile\"}"), "IMPORT_NOT_CHPROFILE");
        assert_eq!(
            err("{\"format\":\"chprofile\",\"schemaVersion\":9,\"profile\":{}}"),
            "IMPORT_FUTURE_VERSION"
        );
        let mut future: Value = serde_json::from_str(&export(&sample(), "x")).expect("json");
        future["profile"]["schemaVersion"] = json!(99);
        assert_eq!(err(&future.to_string()), "IMPORT_FUTURE_VERSION");
        let mut invalid: Value = serde_json::from_str(&export(&sample(), "x")).expect("json");
        invalid["profile"]["click"]["intervalMs"] = json!(0);
        assert_eq!(err(&invalid.to_string()), "IMPORT_INVALID");
        assert_eq!(err(&" ".repeat(MAX_BYTES + 1)), "IMPORT_TOO_LARGE");
        // A UTF-8 BOM (Notepad) is fine.
        assert!(
            import(
                &format!("\u{feff}{}", export(&sample(), "x")),
                "n",
                NOW,
                &b,
                &[]
            )
            .is_ok()
        );
        assert!(ChprofileError::TooLarge.to_string().contains("1 MB"));
    }

    #[test]
    fn file_names_are_safe_for_windows() {
        let mut p = sample();
        assert_eq!(file_name(&p), "Coleta rápida.chprofile");
        p.name = "a/b:c*?. ".into();
        assert_eq!(file_name(&p), "a_b_c__.chprofile");
        p.name = "...".into();
        assert_eq!(file_name(&p), "perfil.chprofile");
    }
}
