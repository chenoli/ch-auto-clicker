//! Scheduler jitter benchmark (TASKS T2.8, ARCHITECTURE §9.3). Injects no input: every fire runs
//! a no-op action.
//!
//!   cargo run -p jitter-bench --release -- [--seconds 10] [--out docs/bench]

use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ch_engine::benchmark::{
    BENCH_INTERVALS_MS, BenchmarkReport, IntervalResult, min_reliable_interval_ms,
};

struct Args {
    seconds: u64,
    out_dir: PathBuf,
}

fn parse_args() -> Args {
    let mut args = Args {
        seconds: 10,
        out_dir: PathBuf::from("docs/bench"),
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--seconds" => {
                args.seconds = it
                    .next()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(args.seconds)
            }
            "--out" => args.out_dir = it.next().map(PathBuf::from).unwrap_or(args.out_dir),
            other => eprintln!("ignoring unknown argument: {other}"),
        }
    }
    args
}

/// Keeps every logical CPU busy at normal priority until dropped.
struct CpuStress {
    stop: Arc<AtomicBool>,
    threads: Vec<std::thread::JoinHandle<()>>,
}

impl CpuStress {
    fn start() -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let n = std::thread::available_parallelism().map_or(4, |n| n.get());
        let threads = (0..n)
            .map(|_| {
                let stop = Arc::clone(&stop);
                std::thread::spawn(move || {
                    let mut x = 0u64;
                    while !stop.load(Ordering::Relaxed) {
                        x = std::hint::black_box(
                            x.wrapping_mul(6364136223846793005).wrapping_add(1),
                        );
                    }
                })
            })
            .collect();
        Self { stop, threads }
    }
}

impl Drop for CpuStress {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        for t in self.threads.drain(..) {
            let _ = t.join();
        }
    }
}

/// UTC civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
fn civil_date(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

fn today() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let (y, m, d) = civil_date((secs / 86_400) as i64);
    format!("{y:04}-{m:02}-{d:02}")
}

fn table(out: &mut String, results: &[IntervalResult]) {
    out.push_str("| Intervalo | Amostras | p50 (µs) | p95 (µs) | p99 (µs) | máx (µs) | perdidos | confiável |\n");
    out.push_str("|---:|---:|---:|---:|---:|---:|---:|:---:|\n");
    for r in results {
        let s = r.stats;
        let _ = writeln!(
            out,
            "| {} ms | {} | {:.0} | {:.0} | {:.0} | {:.0} | {} | {} |",
            r.interval_ms,
            s.samples,
            s.p50_us,
            s.p95_us,
            s.p99_us,
            s.max_us,
            s.missed,
            if r.is_reliable() { "sim" } else { "não" }
        );
    }
}

fn csv(out: &mut String, scenario: &str, results: &[IntervalResult]) {
    for r in results {
        let s = r.stats;
        let _ = writeln!(
            out,
            "{scenario},{},{},{:.1},{:.1},{:.1},{:.1},{}",
            r.interval_ms, s.samples, s.p50_us, s.p95_us, s.p99_us, s.max_us, s.missed
        );
    }
}

#[cfg(windows)]
fn run(seconds: u64) -> Result<(BenchmarkReport, BenchmarkReport), String> {
    let per_interval = Duration::from_secs(seconds);
    let bench = |label: &str| {
        eprintln!(
            "cenário {label}: {} intervalos × {seconds} s…",
            BENCH_INTERVALS_MS.len()
        );
        ch_engine::benchmark::run_system_benchmark(&BENCH_INTERVALS_MS, per_interval)
            .map_err(|e| e.to_string())
    };
    let idle = bench("ocioso")?;
    let stressed = {
        let _stress = CpuStress::start();
        bench("CPU 100%")?
    };
    Ok((idle, stressed))
}

#[cfg(not(windows))]
fn run(_seconds: u64) -> Result<(BenchmarkReport, BenchmarkReport), String> {
    Err("jitter-bench needs Windows".into())
}

fn main() {
    let args = parse_args();
    let (idle, stressed) = match run(args.seconds) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("benchmark failed: {e}");
            std::process::exit(1);
        }
    };

    #[cfg(windows)]
    let (machine, power) = (
        ch_win::system::machine_fingerprint(),
        match ch_win::system::on_battery() {
            Some(true) => "bateria",
            Some(false) => "tomada",
            None => "desconhecida",
        },
    );
    #[cfg(not(windows))]
    let (machine, power) = (String::from("?"), "desconhecida");

    let host = std::env::var("COMPUTERNAME")
        .unwrap_or_else(|_| "maquina".into())
        .to_lowercase();
    let date = today();
    let mut md = String::new();
    let _ = writeln!(md, "# Benchmark de jitter — {date}\n");
    let _ = writeln!(md, "- Máquina: {machine}");
    let _ = writeln!(md, "- Energia: {power}");
    let _ = writeln!(
        md,
        "- Timer: {} · MMCSS: {}",
        idle.timer_kind,
        if idle.mmcss { "ativo" } else { "inativo" }
    );
    let _ = writeln!(
        md,
        "- Duração: {} s por intervalo; ação no-op (nenhum input injetado)",
        args.seconds
    );
    let _ = writeln!(
        md,
        "- Critério de confiável: p99 do atraso < 25% do intervalo e nenhum evento perdido\n"
    );
    for (label, report) in [
        ("Ocioso", &idle),
        ("CPU 100% em todos os núcleos", &stressed),
    ] {
        let _ = writeln!(md, "## {label}\n");
        table(&mut md, &report.results);
        let min = min_reliable_interval_ms(&report.results)
            .map_or("nenhum".into(), |ms| format!("{ms} ms"));
        let _ = writeln!(md, "\nMenor intervalo confiável: **{min}**\n");
    }
    let _ = writeln!(
        md,
        "## CSV\n\n```csv\ncenario,intervalo_ms,amostras,p50_us,p95_us,p99_us,max_us,perdidos"
    );
    csv(&mut md, "ocioso", &idle.results);
    csv(&mut md, "stress", &stressed.results);
    md.push_str("```\n");

    if let Err(e) = std::fs::create_dir_all(&args.out_dir) {
        eprintln!("cannot create {}: {e}", args.out_dir.display());
        std::process::exit(1);
    }
    let path = args.out_dir.join(format!("{date}-{host}.md"));
    match std::fs::write(&path, &md) {
        Ok(()) => println!("{md}\nrelatório: {}", path.display()),
        Err(e) => {
            eprintln!("cannot write {}: {e}", path.display());
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_date_matches_known_days() {
        assert_eq!(civil_date(0), (1970, 1, 1));
        assert_eq!(civil_date(11_016), (2000, 2, 29));
        assert_eq!(civil_date(20_730), (2026, 10, 4));
    }
}
