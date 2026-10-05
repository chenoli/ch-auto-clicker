//! Versioned JSON documents (ARCHITECTURE §4): migrations run on raw JSON before deserializing,
//! so old files keep loading after the model changes.

use serde_json::{Map, Value, json};

use crate::model::{PROFILE_SCHEMA_VERSION, Profile, SETTINGS_SCHEMA_VERSION, Settings};
use crate::validate::{ValidationError, validate_profile, validate_settings};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SchemaError {
    #[error("file was written by a newer version (schema {found}, supported {supported})")]
    FutureVersion { found: u32, supported: u32 },
    #[error("not a valid document: {0}")]
    Malformed(String),
    #[error("invalid values: {0:?}")]
    Invalid(Vec<ValidationError>),
}

impl SchemaError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::FutureVersion { .. } => "SCHEMA_FUTURE_VERSION",
            Self::Malformed(_) => "SCHEMA_MALFORMED",
            Self::Invalid(_) => "SCHEMA_INVALID",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Loaded<T> {
    pub value: T,
    /// Schema version found on disk; lower than current means the caller should back up and rewrite.
    pub from_version: u32,
}

impl<T> Loaded<T> {
    pub fn migrated(&self, current: u32) -> bool {
        self.from_version < current
    }
}

fn version_of(doc: &Value) -> u32 {
    doc.get("schemaVersion")
        .and_then(Value::as_u64)
        .map_or(0, |v| v as u32)
}

/// v0 (pre-release) kept the click settings at the top level; v1 groups them under `click`.
fn profile_v0_to_v1(mut doc: Map<String, Value>) -> Map<String, Value> {
    let take = |doc: &mut Map<String, Value>, key: &str, default: Value| {
        doc.remove(key).unwrap_or(default)
    };
    let click = json!({
        "button": take(&mut doc, "button", json!("left")),
        "kind": take(&mut doc, "kind", json!("single")),
        "intervalMs": take(&mut doc, "intervalMs", json!(100)),
        "variation": take(&mut doc, "variation", json!({ "enabled": false, "blocksMs": [1, 35, 70] })),
    });
    doc.insert("click".into(), click);
    doc.insert("schemaVersion".into(), json!(1));
    doc
}

type Migration = fn(Map<String, Value>) -> Map<String, Value>;

/// Each step upgrades one version; index `n` migrates from `n` to `n + 1`.
const PROFILE_MIGRATIONS: [Migration; 1] = [profile_v0_to_v1];

pub fn load_profile(doc: Value) -> Result<Loaded<Profile>, SchemaError> {
    let from_version = version_of(&doc);
    if from_version > PROFILE_SCHEMA_VERSION {
        return Err(SchemaError::FutureVersion {
            found: from_version,
            supported: PROFILE_SCHEMA_VERSION,
        });
    }
    let Value::Object(mut map) = doc else {
        return Err(SchemaError::Malformed("expected a JSON object".into()));
    };
    for step in &PROFILE_MIGRATIONS[from_version as usize..] {
        map = step(map);
    }
    let profile: Profile = serde_json::from_value(Value::Object(map))
        .map_err(|e| SchemaError::Malformed(e.to_string()))?;
    let errors = validate_profile(&profile);
    if !errors.is_empty() {
        return Err(SchemaError::Invalid(errors));
    }
    Ok(Loaded {
        value: profile,
        from_version,
    })
}

pub fn load_settings(doc: Value) -> Result<Loaded<Settings>, SchemaError> {
    let from_version = version_of(&doc);
    if from_version > SETTINGS_SCHEMA_VERSION {
        return Err(SchemaError::FutureVersion {
            found: from_version,
            supported: SETTINGS_SCHEMA_VERSION,
        });
    }
    let settings: Settings =
        serde_json::from_value(doc).map_err(|e| SchemaError::Malformed(e.to_string()))?;
    let errors = validate_settings(&settings);
    if !errors.is_empty() {
        return Err(SchemaError::Invalid(errors));
    }
    Ok(Loaded {
        value: settings,
        from_version,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::MouseButton;
    use crate::model::Locale;

    fn v1() -> Value {
        serde_json::to_value(Profile::new_default("p1", "Farm", "2026-10-04T00:00:00Z"))
            .expect("json")
    }

    #[test]
    fn current_documents_load_unchanged() {
        let loaded = load_profile(v1()).expect("loads");
        assert_eq!(loaded.from_version, 1);
        assert!(!loaded.migrated(PROFILE_SCHEMA_VERSION));
        assert_eq!(loaded.value.name, "Farm");
    }

    #[test]
    fn migrates_a_v0_profile() {
        let mut doc = v1();
        let map = doc.as_object_mut().expect("object");
        let click = map.remove("click").expect("click");
        map.remove("schemaVersion");
        map.insert("button".into(), json!("right"));
        map.insert("kind".into(), click["kind"].clone());
        map.insert("intervalMs".into(), json!(250));

        let loaded = load_profile(doc).expect("migrates");
        assert_eq!(loaded.from_version, 0);
        assert!(loaded.migrated(PROFILE_SCHEMA_VERSION));
        assert_eq!(loaded.value.schema_version, 1);
        assert_eq!(loaded.value.click.button, MouseButton::Right);
        assert_eq!(loaded.value.click.interval_ms, 250);
        assert_eq!(
            loaded.value.click.variation.blocks_ms,
            vec![1, 35, 70],
            "missing fields get defaults"
        );
    }

    #[test]
    fn refuses_documents_from_a_newer_version() {
        let mut doc = v1();
        doc["schemaVersion"] = json!(7);
        let err = load_profile(doc).expect_err("future");
        assert_eq!(
            err,
            SchemaError::FutureVersion {
                found: 7,
                supported: 1
            }
        );
        assert_eq!(err.code(), "SCHEMA_FUTURE_VERSION");
    }

    #[test]
    fn rejects_malformed_and_invalid_documents() {
        assert_eq!(
            load_profile(json!([1, 2])).expect_err("array").code(),
            "SCHEMA_MALFORMED"
        );
        let mut doc = v1();
        doc["click"]["button"] = json!("thumb");
        assert_eq!(
            load_profile(doc).expect_err("enum").code(),
            "SCHEMA_MALFORMED"
        );
        let mut doc = v1();
        doc["click"]["intervalMs"] = json!(0);
        assert_eq!(
            load_profile(doc).expect_err("invalid"),
            SchemaError::Invalid(vec![ValidationError::IntervalRange])
        );
    }

    #[test]
    fn settings_load_and_validate() {
        let s = serde_json::to_value(Settings::new_default(Locale::PtBr)).expect("json");
        assert_eq!(load_settings(s.clone()).expect("loads").from_version, 1);

        let mut bad = s.clone();
        bad["countdownSec"] = json!(30);
        assert_eq!(
            load_settings(bad).expect_err("invalid").code(),
            "SCHEMA_INVALID"
        );

        let mut future = s;
        future["schemaVersion"] = json!(2);
        assert_eq!(
            load_settings(future).expect_err("future").code(),
            "SCHEMA_FUTURE_VERSION"
        );
        assert_eq!(
            load_settings(json!("x")).expect_err("malformed").code(),
            "SCHEMA_MALFORMED"
        );
    }
}
