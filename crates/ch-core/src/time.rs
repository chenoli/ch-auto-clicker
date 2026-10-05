pub fn civil_date(days: i64) -> (i64, u32, u32) {
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

pub fn iso_utc(epoch_ms: u64) -> String {
    let secs = epoch_ms / 1_000;
    let (y, mo, d) = civil_date((secs / 86_400) as i64);
    let rem = secs % 86_400;
    format!(
        "{y:04}-{mo:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3_600,
        (rem / 60) % 60,
        rem % 60
    )
}

pub fn iso_utc_ms(epoch_ms: u64) -> String {
    let base = iso_utc(epoch_ms);
    format!("{}.{:03}Z", &base[..base.len() - 1], epoch_ms % 1_000)
}

pub fn now_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_date_matches_known_days() {
        assert_eq!(civil_date(0), (1970, 1, 1));
        assert_eq!(civil_date(11_016), (2000, 2, 29));
        assert_eq!(civil_date(20_730), (2026, 10, 4));
        assert_eq!(civil_date(-1), (1969, 12, 31));
    }

    #[test]
    fn formats_iso_utc() {
        assert_eq!(iso_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso_utc(1_791_165_000_999), "2026-10-05T01:50:00Z");
        assert!(now_epoch_ms() > 1_791_000_000_000);
        assert_eq!(iso_utc_ms(1_791_165_000_009), "2026-10-05T01:50:00.009Z");
    }
}
