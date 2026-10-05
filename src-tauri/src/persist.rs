//! Debounced writer (ARCHITECTURE §4): edits coalesce for 300 ms (2 s at most while the user keeps
//! typing or dragging) and go to disk on a background thread, so the UI never waits on I/O.

use std::collections::BTreeMap;
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use ch_core::model::{Profile, Settings};
use ch_storage::Store;

pub const DEBOUNCE: Duration = Duration::from_millis(300);
const MAX_DELAY: Duration = Duration::from_secs(2);
const FLUSH_TIMEOUT: Duration = Duration::from_secs(5);

enum Save {
    Settings(Box<Settings>),
    Profile(Box<Profile>),
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "profile deletion arrives in Phase 8 (T8.1)")
    )]
    Delete(String),
    Flush(mpsc::Sender<()>),
}

#[derive(Default)]
struct Pending {
    settings: Option<Settings>,
    /// `None` marks a deletion.
    profiles: BTreeMap<String, Option<Profile>>,
    since: Option<Instant>,
}

impl Pending {
    fn is_dirty(&self) -> bool {
        self.settings.is_some() || !self.profiles.is_empty()
    }

    fn mark(&mut self) {
        self.since.get_or_insert_with(Instant::now);
    }

    fn write(&mut self, store: &Store) {
        if let Some(s) = self.settings.take()
            && let Err(e) = store.save_settings(&s)
        {
            log::error!("saving settings failed: {e}");
        }
        for (id, profile) in std::mem::take(&mut self.profiles) {
            let result = match profile {
                Some(p) => store.save_profile(&p),
                None => store.delete_profile(&id),
            };
            if let Err(e) = result {
                log::error!("saving profile {id} failed: {e}");
            }
        }
        self.since = None;
    }
}

#[derive(Debug)]
pub struct Persister {
    tx: mpsc::Sender<Save>,
    thread: Option<JoinHandle<()>>,
}

impl std::fmt::Debug for Save {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Settings(_) => "Settings",
            Self::Profile(_) => "Profile",
            Self::Delete(_) => "Delete",
            Self::Flush(_) => "Flush",
        })
    }
}

impl Persister {
    pub fn spawn(store: Store, debounce: Duration) -> Self {
        let (tx, rx) = mpsc::channel::<Save>();
        let thread = std::thread::Builder::new()
            .name("ch-persist".into())
            .spawn(move || {
                let mut pending = Pending::default();
                loop {
                    let message = if pending.is_dirty() {
                        let overdue = pending.since.is_some_and(|t| t.elapsed() >= MAX_DELAY);
                        if overdue {
                            pending.write(&store);
                            continue;
                        }
                        rx.recv_timeout(debounce)
                    } else {
                        rx.recv().map_err(|_| mpsc::RecvTimeoutError::Disconnected)
                    };
                    match message {
                        Ok(Save::Settings(s)) => {
                            pending.settings = Some(*s);
                            pending.mark();
                        }
                        Ok(Save::Profile(p)) => {
                            pending.profiles.insert(p.id.clone(), Some(*p));
                            pending.mark();
                        }
                        Ok(Save::Delete(id)) => {
                            pending.profiles.insert(id, None);
                            pending.mark();
                        }
                        Ok(Save::Flush(ack)) => {
                            pending.write(&store);
                            let _ = ack.send(());
                        }
                        Err(mpsc::RecvTimeoutError::Timeout) => pending.write(&store),
                        Err(mpsc::RecvTimeoutError::Disconnected) => {
                            pending.write(&store);
                            break;
                        }
                    }
                }
            })
            .expect("failed to spawn the persistence thread");
        Self {
            tx,
            thread: Some(thread),
        }
    }

    pub fn save_settings(&self, settings: &Settings) {
        let _ = self.tx.send(Save::Settings(Box::new(settings.clone())));
    }

    pub fn save_profile(&self, profile: &Profile) {
        let _ = self.tx.send(Save::Profile(Box::new(profile.clone())));
    }

    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "profile deletion arrives in Phase 8 (T8.1)")
    )]
    pub fn delete_profile(&self, id: &str) {
        let _ = self.tx.send(Save::Delete(id.to_owned()));
    }

    /// Writes everything pending now and waits for it (used on exit).
    pub fn flush(&self) {
        let (ack, done) = mpsc::channel();
        if self.tx.send(Save::Flush(ack)).is_ok() {
            let _ = done.recv_timeout(FLUSH_TIMEOUT);
        }
    }
}

impl Drop for Persister {
    fn drop(&mut self) {
        self.flush();
        // Replacing the sender closes the channel so the thread writes and exits.
        let (dead, _) = mpsc::channel();
        drop(std::mem::replace(&mut self.tx, dead));
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use ch_core::model::Locale;

    use super::*;

    fn temp_store(name: &str) -> (Store, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "ch-persist-{}-{name}-{}",
            std::process::id(),
            ch_core::time::now_epoch_ms()
        ));
        (Store::open(&dir).expect("store"), dir)
    }

    fn profile(id: &str, name: &str) -> Profile {
        Profile::new_default(id, name, "2026-10-04T00:00:00Z")
    }

    fn read_profile(store: &Store, id: &str) -> Option<Profile> {
        let text = std::fs::read_to_string(store.profile_path(id)).ok()?;
        serde_json::from_str(&text).ok()
    }

    #[test]
    fn coalesces_rapid_edits_into_one_write() {
        let (store, dir) = temp_store("coalesce");
        let persister = Persister::spawn(store.clone(), Duration::from_millis(80));
        for i in 0..20 {
            persister.save_profile(&profile("p", &format!("v{i}")));
        }
        assert!(
            read_profile(&store, "p").is_none(),
            "nothing written while edits keep coming"
        );
        std::thread::sleep(Duration::from_millis(300));
        assert_eq!(
            read_profile(&store, "p").map(|p| p.name).as_deref(),
            Some("v19")
        );
        assert!(
            !store.profile_path("p").with_extension("json.bak").exists(),
            "a single write, so no previous version"
        );
        drop(persister);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn flush_writes_immediately_and_drop_writes_the_rest() {
        let (store, dir) = temp_store("flush");
        let persister = Persister::spawn(store.clone(), Duration::from_secs(60));
        persister.save_settings(&Settings::new_default(Locale::EnUs));
        persister.save_profile(&profile("a", "A"));
        persister.flush();
        assert!(store.settings_path().exists());
        assert!(read_profile(&store, "a").is_some());

        persister.delete_profile("a");
        persister.save_profile(&profile("b", "B"));
        drop(persister);
        assert!(read_profile(&store, "a").is_none(), "deleted on drop");
        assert!(read_profile(&store, "b").is_some(), "written on drop");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn debug_names_the_message() {
        assert_eq!(format!("{:?}", Save::Delete("x".into())), "Delete");
    }
}
