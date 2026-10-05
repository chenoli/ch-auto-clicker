//! Timing capabilities (ARCHITECTURE §5.3, TASKS T2.9): the shortest interval this machine
//! sustains, measured once per machine fingerprint and cached.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ch_engine::benchmark::{BENCH_INTERVALS_MS, BenchmarkReport, DEFAULT_MIN_RELIABLE_MS};
use serde::{Deserialize, Serialize};

/// ~3 s in total across the five intervals.
pub const PER_INTERVAL: Duration = Duration::from_millis(600);
const CACHE_VERSION: u32 = 1;
/// When nothing passes, fall back to the longest interval measured.
const WORST_CASE_MS: u32 = 16;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BenchResult {
    pub interval_ms: u32,
    pub p99_us: f64,
    pub missed: u32,
    pub reliable: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub min_reliable_interval_ms: u32,
    /// False until the benchmark has run on this machine; the UI then shows the 4 ms default.
    pub measured: bool,
    pub timer_kind: String,
    pub mmcss: bool,
    pub measured_at_ms: Option<f64>,
    pub fingerprint: String,
    pub results: Vec<BenchResult>,
}

impl Capabilities {
    pub fn provisional(fingerprint: String) -> Self {
        Self {
            min_reliable_interval_ms: DEFAULT_MIN_RELIABLE_MS as u32,
            measured: false,
            timer_kind: String::new(),
            mmcss: false,
            measured_at_ms: None,
            fingerprint,
            results: Vec::new(),
        }
    }

    pub fn from_report(report: &BenchmarkReport, fingerprint: String, measured_at_ms: f64) -> Self {
        Self {
            min_reliable_interval_ms: report.min_reliable_ms.map_or(WORST_CASE_MS, |ms| ms as u32),
            measured: true,
            timer_kind: report.timer_kind.to_owned(),
            mmcss: report.mmcss,
            measured_at_ms: Some(measured_at_ms),
            fingerprint,
            results: report
                .results
                .iter()
                .map(|r| BenchResult {
                    interval_ms: r.interval_ms as u32,
                    p99_us: r.stats.p99_us,
                    missed: r.stats.missed.min(u64::from(u32::MAX)) as u32,
                    reliable: r.is_reliable(),
                })
                .collect(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct CacheFile {
    version: u32,
    capabilities: Capabilities,
}

pub fn cache_path() -> PathBuf {
    crate::paths::cache_dir().join("capabilities.json")
}

/// Cached capabilities, only if they were measured on this same machine and build.
pub fn load_cached(path: &Path, fingerprint: &str) -> Option<Capabilities> {
    let text = std::fs::read_to_string(path).ok()?;
    let file: CacheFile = serde_json::from_str(&text).ok()?;
    (file.version == CACHE_VERSION
        && file.capabilities.measured
        && file.capabilities.fingerprint == fingerprint)
        .then_some(file.capabilities)
}

pub fn save_cache(path: &Path, capabilities: &Capabilities) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let file = CacheFile {
        version: CACHE_VERSION,
        capabilities: capabilities.clone(),
    };
    let json = serde_json::to_string_pretty(&file).map_err(std::io::Error::other)?;
    // Write-then-rename so a crash never leaves a truncated cache behind.
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json)?;
    std::fs::rename(tmp, path)
}

pub fn now_ms() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0.0, |d| d.as_secs_f64() * 1000.0)
}

pub fn fingerprint() -> String {
    #[cfg(windows)]
    return ch_win::system::machine_fingerprint();
    #[cfg(not(windows))]
    return String::from("non-windows");
}

/// Runs the benchmark on a real-time thread (blocks ~3 s) and caches the result.
#[cfg(windows)]
pub fn measure() -> Result<Capabilities, String> {
    let report = ch_engine::benchmark::run_system_benchmark(&BENCH_INTERVALS_MS, PER_INTERVAL)
        .map_err(|e| e.to_string())?;
    let capabilities = Capabilities::from_report(&report, fingerprint(), now_ms());
    if let Err(e) = save_cache(&cache_path(), &capabilities) {
        log::warn!("could not cache timing capabilities: {e}");
    }
    log::info!(
        "timing benchmark: min reliable {} ms ({}, mmcss {})",
        capabilities.min_reliable_interval_ms,
        capabilities.timer_kind,
        capabilities.mmcss
    );
    Ok(capabilities)
}

#[cfg(not(windows))]
pub fn measure() -> Result<Capabilities, String> {
    let _ = BENCH_INTERVALS_MS;
    Err("timing benchmark needs Windows".into())
}

#[cfg(test)]
mod tests {
    use ch_core::stats::JitterStats;
    use ch_engine::benchmark::IntervalResult;

    use super::*;

    fn report(min: Option<u64>) -> BenchmarkReport {
        BenchmarkReport {
            timer_kind: "highResolution",
            mmcss: true,
            min_reliable_ms: min,
            results: vec![IntervalResult {
                interval_ms: 2,
                stats: JitterStats {
                    samples: 300,
                    p99_us: 12.0,
                    ..JitterStats::default()
                },
            }],
        }
    }

    fn temp_cache(name: &str) -> PathBuf {
        std::env::temp_dir()
            .join(format!("ch-clicker-test-{}-{name}", std::process::id()))
            .join("capabilities.json")
    }

    #[test]
    fn provisional_uses_the_handoff_default() {
        let c = Capabilities::provisional("fp".into());
        assert_eq!(c.min_reliable_interval_ms, 4);
        assert!(!c.measured);
    }

    #[test]
    fn converts_a_benchmark_report() {
        let c = Capabilities::from_report(&report(Some(2)), "fp".into(), 1_000.0);
        assert_eq!(c.min_reliable_interval_ms, 2);
        assert!(c.measured && c.mmcss);
        assert_eq!(c.timer_kind, "highResolution");
        assert_eq!(
            c.results,
            vec![BenchResult {
                interval_ms: 2,
                p99_us: 12.0,
                missed: 0,
                reliable: true
            }]
        );
        assert_eq!(
            Capabilities::from_report(&report(None), "fp".into(), 0.0).min_reliable_interval_ms,
            16
        );
    }

    #[test]
    fn cache_round_trips_and_is_tied_to_the_fingerprint() {
        let path = temp_cache("roundtrip");
        let c = Capabilities::from_report(&report(Some(1)), "machine-a".into(), now_ms());
        save_cache(&path, &c).expect("save");
        assert_eq!(load_cached(&path, "machine-a"), Some(c));
        assert_eq!(
            load_cached(&path, "machine-b"),
            None,
            "another machine or Windows build"
        );
        let _ = std::fs::remove_dir_all(path.parent().expect("dir"));
    }

    #[test]
    fn unmeasured_corrupt_or_missing_caches_are_ignored() {
        let path = temp_cache("invalid");
        assert_eq!(load_cached(&path, "fp"), None, "missing");
        save_cache(&path, &Capabilities::provisional("fp".into())).expect("save");
        assert_eq!(load_cached(&path, "fp"), None, "never measured");
        std::fs::write(&path, "{ not json").expect("write");
        assert_eq!(load_cached(&path, "fp"), None, "corrupt");
        let _ = std::fs::remove_dir_all(path.parent().expect("dir"));
    }

    #[test]
    fn default_cache_lives_in_the_cache_dir() {
        assert!(
            cache_path().ends_with("cache\\capabilities.json")
                || cache_path().ends_with("cache/capabilities.json")
        );
        assert!(!fingerprint().is_empty());
    }
}
