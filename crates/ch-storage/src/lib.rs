use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use ch_core::model::{PROFILE_SCHEMA_VERSION, Profile, SETTINGS_SCHEMA_VERSION, Settings};
use ch_core::schema::{self, SchemaError};

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("i/o error on {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{path}: {source}")]
    Schema {
        path: PathBuf,
        #[source]
        source: SchemaError,
    },
    #[error("cannot serialize: {0}")]
    Serialize(#[from] serde_json::Error),
}

impl StorageError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Io { .. } => "STORAGE_IO",
            Self::Schema { source, .. } => source.code(),
            Self::Serialize(_) => "STORAGE_SERIALIZE",
        }
    }
}

fn io_err(path: &Path) -> impl FnOnce(std::io::Error) -> StorageError + '_ {
    move |source| StorageError::Io {
        path: path.to_path_buf(),
        source,
    }
}

#[derive(Debug)]
pub struct LoadIssue {
    pub file: PathBuf,
    pub quarantined_to: Option<PathBuf>,
    pub error: StorageError,
}

#[derive(Debug)]
pub struct LoadedState {
    pub settings: Settings,
    pub profiles: Vec<Profile>,
    pub issues: Vec<LoadIssue>,
    pub first_run: bool,
}

#[derive(Debug, Clone)]
pub struct Store {
    root: PathBuf,
}

impl Store {
    pub fn default_root() -> PathBuf {
        std::env::var_os("APPDATA")
            .map_or_else(std::env::temp_dir, PathBuf::from)
            .join("Chenoli")
            .join("CH Auto Clicker")
    }

    pub fn open(root: impl Into<PathBuf>) -> Result<Self, StorageError> {
        let store = Self { root: root.into() };
        for dir in [
            store.root.clone(),
            store.profiles_dir(),
            store.backups_dir(),
        ] {
            fs::create_dir_all(&dir).map_err(io_err(&dir))?;
        }
        Ok(store)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn settings_path(&self) -> PathBuf {
        self.root.join("settings.json")
    }

    pub fn profiles_dir(&self) -> PathBuf {
        self.root.join("profiles")
    }

    pub fn backups_dir(&self) -> PathBuf {
        self.root.join("backups")
    }

    pub fn profile_path(&self, id: &str) -> PathBuf {
        self.profiles_dir()
            .join(format!("{}.json", sanitize_id(id)))
    }

    pub fn load_or_init(
        &self,
        default_settings: impl FnOnce() -> Settings,
        default_profile: impl FnOnce() -> Profile,
    ) -> Result<LoadedState, StorageError> {
        let mut issues = Vec::new();
        let settings_path = self.settings_path();
        let had_settings = settings_path.exists();
        let mut settings = if had_settings {
            match self.read_doc(&settings_path).and_then(|doc| {
                schema::load_settings(doc).map_err(|source| StorageError::Schema {
                    path: settings_path.clone(),
                    source,
                })
            }) {
                Ok(loaded) => {
                    if loaded.migrated(SETTINGS_SCHEMA_VERSION) {
                        self.backup_before_migration(&settings_path, loaded.from_version)?;
                        self.save_settings(&loaded.value)?;
                    }
                    Some(loaded.value)
                }
                Err(error) => {
                    issues.push(self.quarantine(&settings_path, error));
                    None
                }
            }
        } else {
            None
        };

        let mut profiles = Vec::new();
        let dir = self.profiles_dir();
        let mut entries: Vec<_> = fs::read_dir(&dir)
            .map_err(io_err(&dir))?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "json"))
            .collect();
        entries.sort();
        for path in entries {
            let loaded = self.read_doc(&path).and_then(|doc| {
                schema::load_profile(doc).map_err(|source| StorageError::Schema {
                    path: path.clone(),
                    source,
                })
            });
            match loaded {
                Ok(loaded) => {
                    if loaded.migrated(PROFILE_SCHEMA_VERSION) {
                        self.backup_before_migration(&path, loaded.from_version)?;
                        self.save_profile(&loaded.value)?;
                    }
                    profiles.push(loaded.value);
                }
                Err(error) => issues.push(self.quarantine(&path, error)),
            }
        }

        let first_run = !had_settings && profiles.is_empty();
        let settings_missing = settings.is_none();
        let mut settings = settings.take().unwrap_or_else(default_settings);
        if profiles.is_empty() {
            let profile = default_profile();
            self.save_profile(&profile)?;
            profiles.push(profile);
        }
        let changed = reconcile(&mut settings, &profiles);
        if settings_missing || changed {
            self.save_settings(&settings)?;
        }
        profiles.sort_by_key(|p| {
            settings
                .profile_order
                .iter()
                .position(|id| *id == p.id)
                .unwrap_or(usize::MAX)
        });
        Ok(LoadedState {
            settings,
            profiles,
            issues,
            first_run,
        })
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<(), StorageError> {
        let json = serde_json::to_vec_pretty(settings)?;
        write_atomic(&self.settings_path(), &json)
    }

    pub fn save_profile(&self, profile: &Profile) -> Result<(), StorageError> {
        let json = serde_json::to_vec_pretty(profile)?;
        write_atomic(&self.profile_path(&profile.id), &json)
    }

    pub fn delete_profile(&self, id: &str) -> Result<(), StorageError> {
        let path = self.profile_path(id);
        if !path.exists() {
            return Ok(());
        }
        let target = self
            .backups_dir()
            .join(format!("{}.deleted.json", sanitize_id(id)));
        fs::rename(&path, &target).map_err(io_err(&path))?;
        let _ = fs::remove_file(path.with_extension("json.bak"));
        Ok(())
    }

    fn read_doc(&self, path: &Path) -> Result<serde_json::Value, StorageError> {
        let text = fs::read_to_string(path).map_err(io_err(path))?;
        serde_json::from_str(&text).map_err(|e| StorageError::Schema {
            path: path.to_path_buf(),
            source: SchemaError::Malformed(e.to_string()),
        })
    }

    fn backup_before_migration(&self, path: &Path, from_version: u32) -> Result<(), StorageError> {
        let name = path
            .file_stem()
            .map_or_else(|| "file".into(), |s| s.to_string_lossy().into_owned());
        let target = self
            .backups_dir()
            .join(format!("{name}.v{from_version}.json"));
        fs::copy(path, &target).map_err(io_err(path))?;
        Ok(())
    }

    fn quarantine(&self, path: &Path, error: StorageError) -> LoadIssue {
        let name = path
            .file_name()
            .map_or_else(|| "file".into(), |s| s.to_string_lossy().into_owned());
        let target = self
            .backups_dir()
            .join(format!("{name}.corrupt-{}", ch_core::time::now_epoch_ms()));
        let quarantined_to = fs::rename(path, &target).ok().map(|()| target);
        LoadIssue {
            file: path.to_path_buf(),
            quarantined_to,
            error,
        }
    }
}

pub fn reconcile(settings: &mut Settings, profiles: &[Profile]) -> bool {
    let before = (
        settings.active_profile_id.clone(),
        settings.profile_order.clone(),
    );
    settings
        .profile_order
        .retain(|id| profiles.iter().any(|p| p.id == *id));
    for p in profiles {
        if !settings.profile_order.contains(&p.id) {
            settings.profile_order.push(p.id.clone());
        }
    }
    let active_exists = settings
        .active_profile_id
        .as_ref()
        .is_some_and(|id| profiles.iter().any(|p| p.id == *id));
    if !active_exists {
        settings.active_profile_id = settings.profile_order.first().cloned();
    }
    before
        != (
            settings.active_profile_id.clone(),
            settings.profile_order.clone(),
        )
}

fn sanitize_id(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), StorageError> {
    let tmp = path.with_extension("json.tmp");
    {
        let mut file = File::create(&tmp).map_err(io_err(&tmp))?;
        file.write_all(bytes).map_err(io_err(&tmp))?;
        file.sync_all().map_err(io_err(&tmp))?;
    }
    if path.exists() {
        let bak = path.with_extension("json.bak");
        fs::copy(path, &bak).map_err(io_err(path))?;
    }
    fs::rename(&tmp, path).map_err(io_err(path))
}

#[cfg(test)]
mod tests {
    use ch_core::model::Locale;
    use serde_json::json;

    use super::*;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "ch-storage-{}-{name}-{}",
                std::process::id(),
                ch_core::time::now_epoch_ms()
            ));
            let _ = fs::remove_dir_all(&dir);
            Self(dir)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn profile(id: &str) -> Profile {
        Profile::new_default(id, format!("Profile {id}"), "2026-10-04T00:00:00Z")
    }

    fn init(store: &Store) -> LoadedState {
        store
            .load_or_init(
                || Settings::new_default(Locale::PtBr),
                || profile("default"),
            )
            .expect("load")
    }

    #[test]
    fn first_run_writes_defaults() {
        let dir = TempDir::new("first-run");
        let store = Store::open(&dir.0).expect("open");
        let state = init(&store);
        assert!(state.first_run);
        assert_eq!(state.profiles.len(), 1);
        assert_eq!(state.settings.active_profile_id.as_deref(), Some("default"));
        assert_eq!(state.settings.profile_order, vec!["default".to_owned()]);
        assert!(store.settings_path().exists());
        assert!(store.profile_path("default").exists());

        let again = init(&store);
        assert!(!again.first_run);
        assert_eq!(again.profiles, state.profiles);
    }

    #[test]
    fn saved_settings_ignore_the_first_run_defaults() {
        let dir = TempDir::new("keep-locale");
        let store = Store::open(&dir.0).expect("open");
        init(&store);

        let again = store
            .load_or_init(|| Settings::new_default(Locale::Es419), || profile("other"))
            .expect("load");
        assert_eq!(again.settings.locale, Locale::PtBr);
        assert_eq!(again.profiles.len(), 1);
        assert_eq!(again.profiles[0].id, "default");
    }

    #[test]
    fn profiles_follow_the_saved_order() {
        let dir = TempDir::new("order");
        let store = Store::open(&dir.0).expect("open");
        for id in ["a", "b", "c"] {
            store.save_profile(&profile(id)).expect("save");
        }
        let mut settings = Settings::new_default(Locale::PtBr);
        settings.profile_order = vec!["c".into(), "a".into(), "gone".into()];
        settings.active_profile_id = Some("gone".into());
        store.save_settings(&settings).expect("save");

        let state = init(&store);
        let ids: Vec<_> = state.profiles.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, vec!["c", "a", "b"]);
        assert_eq!(state.settings.profile_order, vec!["c", "a", "b"]);
        assert_eq!(
            state.settings.active_profile_id.as_deref(),
            Some("c"),
            "missing active falls back"
        );
    }

    #[test]
    fn a_corrupt_profile_is_quarantined_without_blocking_the_rest() {
        let dir = TempDir::new("corrupt");
        let store = Store::open(&dir.0).expect("open");
        store.save_profile(&profile("good")).expect("save");
        fs::write(store.profile_path("bad"), "{ nope").expect("write");

        let state = init(&store);
        assert_eq!(state.profiles.len(), 1);
        assert_eq!(state.issues.len(), 1);
        assert_eq!(state.issues[0].error.code(), "SCHEMA_MALFORMED");
        assert!(
            state.issues[0]
                .quarantined_to
                .as_ref()
                .is_some_and(|p| p.exists())
        );
        assert!(!store.profile_path("bad").exists());
    }

    #[test]
    fn a_file_from_a_newer_version_is_set_aside() {
        let dir = TempDir::new("future");
        let store = Store::open(&dir.0).expect("open");
        let mut doc = serde_json::to_value(profile("new")).expect("json");
        doc["schemaVersion"] = json!(9);
        fs::write(store.profile_path("new"), doc.to_string()).expect("write");
        let state = init(&store);
        assert_eq!(state.issues[0].error.code(), "SCHEMA_FUTURE_VERSION");
        assert_eq!(state.profiles[0].id, "default", "defaults fill the gap");
    }

    #[test]
    fn migrated_profiles_are_backed_up_and_rewritten() {
        let dir = TempDir::new("migrate");
        let store = Store::open(&dir.0).expect("open");
        let mut doc = serde_json::to_value(profile("old")).expect("json");
        let map = doc.as_object_mut().expect("object");
        let click = map.remove("click").expect("click");
        map.remove("schemaVersion");
        for key in ["button", "kind", "intervalMs", "variation"] {
            map.insert(key.into(), click[key].clone());
        }
        fs::write(store.profile_path("old"), doc.to_string()).expect("write");

        let state = init(&store);
        assert_eq!(state.profiles[0].schema_version, 1);
        assert!(store.backups_dir().join("old.v0.json").exists());
        let rewritten: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(store.profile_path("old")).expect("read"))
                .expect("json");
        assert_eq!(rewritten["schemaVersion"], 1);
    }

    #[test]
    fn corrupt_settings_are_replaced_by_defaults() {
        let dir = TempDir::new("settings");
        let store = Store::open(&dir.0).expect("open");
        fs::write(store.settings_path(), "[]").expect("write");
        let state = init(&store);
        assert_eq!(state.issues.len(), 1);
        assert_eq!(state.settings.countdown_sec, 3);
        assert!(store.settings_path().exists(), "defaults written back");
    }

    #[test]
    fn writes_keep_the_previous_version_and_leave_no_temp_files() {
        let dir = TempDir::new("atomic");
        let store = Store::open(&dir.0).expect("open");
        let mut p = profile("p");
        store.save_profile(&p).expect("first");
        p.name = "Renomeado".into();
        store.save_profile(&p).expect("second");

        let path = store.profile_path("p");
        let current: Profile =
            serde_json::from_str(&fs::read_to_string(&path).expect("read")).expect("json");
        let previous: Profile = serde_json::from_str(
            &fs::read_to_string(path.with_extension("json.bak")).expect("bak"),
        )
        .expect("json");
        assert_eq!(current.name, "Renomeado");
        assert_eq!(previous.name, "Profile p");
        assert!(!path.with_extension("json.tmp").exists());
    }

    #[test]
    fn a_crash_before_the_rename_leaves_the_old_file_intact() {
        let dir = TempDir::new("crash");
        let store = Store::open(&dir.0).expect("open");
        store.save_profile(&profile("p")).expect("save");
        fs::write(store.profile_path("p").with_extension("json.tmp"), "{ half").expect("write");
        let state = init(&store);
        assert!(state.issues.is_empty(), "temp files are not profiles");
        assert_eq!(state.profiles[0].id, "p");
    }

    #[test]
    fn delete_moves_the_profile_to_backups() {
        let dir = TempDir::new("delete");
        let store = Store::open(&dir.0).expect("open");
        store.save_profile(&profile("x")).expect("save");
        store.delete_profile("x").expect("delete");
        assert!(!store.profile_path("x").exists());
        assert!(store.backups_dir().join("x.deleted.json").exists());
        store.delete_profile("x").expect("deleting twice is fine");
    }

    #[test]
    fn ids_cannot_escape_the_profiles_folder() {
        let dir = TempDir::new("sanitize");
        let store = Store::open(&dir.0).expect("open");
        let path = store.profile_path("..\\..\\evil");
        assert_eq!(path.parent(), Some(store.profiles_dir().as_path()));
        assert!(
            Store::default_root().ends_with("Chenoli\\CH Auto Clicker")
                || Store::default_root().ends_with("Chenoli/CH Auto Clicker")
        );
        assert_eq!(store.root(), dir.0.as_path());
    }

    #[test]
    fn errors_expose_codes() {
        let err = StorageError::Io {
            path: "x".into(),
            source: std::io::Error::other("boom"),
        };
        assert_eq!(err.code(), "STORAGE_IO");
        assert!(err.to_string().contains("boom"));
    }
}
