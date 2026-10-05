//! Validation rules from ARCHITECTURE §4, applied on import, update and start. Each error carries
//! a stable code the UI translates.

use crate::model::{
    COORD_MAX, COUNTDOWN_MAX_SECONDS, DELAY_MS_MAX, INTERVAL_MS_MAX, INTERVAL_MS_MIN,
    LINE_SPACING_MAX, LINE_SPACING_MIN, NAME_MAX_CHARS, POINTS_MAX, Profile, SCAN_AREA_MAX,
    SCAN_AREA_MIN, STOP_AFTER_MIN_SECONDS, Settings, VARIATION_BLOCKS_MAX,
};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ValidationError {
    #[error("name must have 1 to {NAME_MAX_CHARS} characters")]
    NameLength,
    #[error("interval must be between {INTERVAL_MS_MIN} and {INTERVAL_MS_MAX} ms")]
    IntervalRange,
    #[error("variation needs 1 to {VARIATION_BLOCKS_MAX} blocks")]
    VariationBlockCount,
    #[error("variation block {index} is outside {INTERVAL_MS_MIN}..={INTERVAL_MS_MAX} ms")]
    VariationBlockRange { index: usize },
    #[error("stop timer must be at least {STOP_AFTER_MIN_SECONDS} s")]
    StopAfterTooShort,
    #[error("amplitude must be between 0 and 100%")]
    AmplitudeRange,
    #[error("wiggle speed must be 4, 8 or 16 Hz")]
    WiggleSpeed,
    #[error("line spacing must be between {LINE_SPACING_MIN} and {LINE_SPACING_MAX} px")]
    LineSpacingRange,
    #[error("scan speed must be 600, 1200 or 2400 px/s")]
    ScanSpeed,
    #[error("scan area must be at least {SCAN_AREA_MIN} px on each side")]
    ScanAreaTooSmall,
    #[error("scan area must be at most {SCAN_AREA_MAX} px on each side, inside ±{COORD_MAX}")]
    ScanAreaTooLarge,
    #[error("point {index} is outside ±{COORD_MAX} px")]
    PointCoordinateRange { index: usize },
    #[error("at most {POINTS_MAX} points")]
    TooManyPoints,
    #[error("point {index} delay exceeds {DELAY_MS_MAX} ms")]
    PointDelayRange { index: usize },
    #[error("countdown must be between 0 and {COUNTDOWN_MAX_SECONDS} s")]
    CountdownRange,
    #[error("unsupported schema version {0}")]
    SchemaVersion(u32),
}

impl ValidationError {
    /// Stable identifier for i18n (`errors.<code>`).
    pub fn code(&self) -> &'static str {
        match self {
            Self::NameLength => "NAME_LENGTH",
            Self::IntervalRange => "INTERVAL_RANGE",
            Self::VariationBlockCount => "VARIATION_BLOCK_COUNT",
            Self::VariationBlockRange { .. } => "VARIATION_BLOCK_RANGE",
            Self::StopAfterTooShort => "STOP_AFTER_TOO_SHORT",
            Self::AmplitudeRange => "AMPLITUDE_RANGE",
            Self::WiggleSpeed => "WIGGLE_SPEED",
            Self::LineSpacingRange => "LINE_SPACING_RANGE",
            Self::ScanSpeed => "SCAN_SPEED",
            Self::ScanAreaTooSmall => "SCAN_AREA_TOO_SMALL",
            Self::ScanAreaTooLarge => "SCAN_AREA_TOO_LARGE",
            Self::PointCoordinateRange { .. } => "POINT_COORDINATE_RANGE",
            Self::TooManyPoints => "TOO_MANY_POINTS",
            Self::PointDelayRange { .. } => "POINT_DELAY_RANGE",
            Self::CountdownRange => "COUNTDOWN_RANGE",
            Self::SchemaVersion(_) => "SCHEMA_VERSION",
        }
    }
}

fn interval_ok(ms: u32) -> bool {
    (INTERVAL_MS_MIN..=INTERVAL_MS_MAX).contains(&ms)
}

/// All rule violations of a profile (empty when valid).
pub fn validate_profile(p: &Profile) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let name_len = p.name.trim().chars().count();
    if name_len == 0 || name_len > NAME_MAX_CHARS {
        errors.push(ValidationError::NameLength);
    }
    if !interval_ok(p.click.interval_ms) {
        errors.push(ValidationError::IntervalRange);
    }
    let blocks = &p.click.variation.blocks_ms;
    if blocks.is_empty() || blocks.len() > VARIATION_BLOCKS_MAX {
        errors.push(ValidationError::VariationBlockCount);
    }
    for (index, &ms) in blocks.iter().enumerate() {
        if !interval_ok(ms) {
            errors.push(ValidationError::VariationBlockRange { index });
        }
    }
    if p.stop_after.seconds < STOP_AFTER_MIN_SECONDS {
        errors.push(ValidationError::StopAfterTooShort);
    }
    let w = &p.modes.wiggle;
    if !(0.0..=100.0).contains(&w.amplitude_pct) || w.amplitude_pct.is_nan() {
        errors.push(ValidationError::AmplitudeRange);
    }
    if ![4, 8, 16].contains(&w.speed_hz) {
        errors.push(ValidationError::WiggleSpeed);
    }
    let s = &p.modes.scan;
    if !(LINE_SPACING_MIN..=LINE_SPACING_MAX).contains(&s.line_spacing_px) {
        errors.push(ValidationError::LineSpacingRange);
    }
    if ![600, 1200, 2400].contains(&s.speed_px_s) {
        errors.push(ValidationError::ScanSpeed);
    }
    if s.area
        .is_some_and(|a| a.w < SCAN_AREA_MIN || a.h < SCAN_AREA_MIN)
    {
        errors.push(ValidationError::ScanAreaTooSmall);
    }
    if s.area.is_some_and(|a| {
        a.w > SCAN_AREA_MAX || a.h > SCAN_AREA_MAX || a.x.abs() > COORD_MAX || a.y.abs() > COORD_MAX
    }) {
        errors.push(ValidationError::ScanAreaTooLarge);
    }
    let points = &p.modes.click.points;
    if points.len() > POINTS_MAX {
        errors.push(ValidationError::TooManyPoints);
    }
    for (index, pt) in points.iter().enumerate() {
        if pt.delay_ms > DELAY_MS_MAX {
            errors.push(ValidationError::PointDelayRange { index });
        }
        if pt.x.abs() > COORD_MAX || pt.y.abs() > COORD_MAX {
            errors.push(ValidationError::PointCoordinateRange { index });
        }
    }
    errors
}

pub fn validate_settings(s: &Settings) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    if s.countdown_sec > COUNTDOWN_MAX_SECONDS {
        errors.push(ValidationError::CountdownRange);
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Area, Locale, ScreenPoint};

    fn profile() -> Profile {
        Profile::new_default("p", "Coleta rápida", "2026-10-04T00:00:00Z")
    }

    #[test]
    fn defaults_are_valid() {
        assert!(validate_profile(&profile()).is_empty());
        assert!(validate_settings(&Settings::new_default(Locale::PtBr)).is_empty());
    }

    #[test]
    fn flags_every_rule() {
        let mut p = profile();
        p.name = "   ".into();
        p.click.interval_ms = 0;
        p.click.variation.blocks_ms = vec![5, 0, 70_000];
        p.stop_after.seconds = 10;
        p.modes.wiggle.amplitude_pct = 120.0;
        p.modes.wiggle.speed_hz = 5;
        p.modes.scan.line_spacing_px = 4;
        p.modes.scan.speed_px_s = 1000;
        p.modes.scan.area = Some(Area {
            x: 0,
            y: 0,
            w: 10,
            h: 200,
        });
        p.modes.click.points = (0..65)
            .map(|i| ScreenPoint {
                id: format!("p{i}"),
                x: 0,
                y: 0,
                delay_ms: if i == 3 { 70_000 } else { 0 },
            })
            .collect();
        let codes: Vec<_> = validate_profile(&p)
            .iter()
            .map(ValidationError::code)
            .collect();
        assert_eq!(
            codes,
            vec![
                "NAME_LENGTH",
                "INTERVAL_RANGE",
                "VARIATION_BLOCK_RANGE",
                "VARIATION_BLOCK_RANGE",
                "STOP_AFTER_TOO_SHORT",
                "AMPLITUDE_RANGE",
                "WIGGLE_SPEED",
                "LINE_SPACING_RANGE",
                "SCAN_SPEED",
                "SCAN_AREA_TOO_SMALL",
                "TOO_MANY_POINTS",
                "POINT_DELAY_RANGE",
            ]
        );
    }

    #[test]
    fn variation_needs_between_one_and_thirty_two_blocks() {
        let mut p = profile();
        p.click.variation.blocks_ms.clear();
        assert_eq!(
            validate_profile(&p),
            vec![ValidationError::VariationBlockCount]
        );
        p.click.variation.blocks_ms = vec![10; 33];
        assert_eq!(
            validate_profile(&p),
            vec![ValidationError::VariationBlockCount]
        );
    }

    #[test]
    fn names_are_limited_to_48_characters() {
        let mut p = profile();
        p.name = "ç".repeat(48);
        assert!(validate_profile(&p).is_empty());
        p.name.push('x');
        assert_eq!(validate_profile(&p), vec![ValidationError::NameLength]);
    }

    #[test]
    fn countdown_is_capped_at_ten_seconds() {
        let mut s = Settings::new_default(Locale::EnUs);
        s.countdown_sec = 11;
        assert_eq!(validate_settings(&s), vec![ValidationError::CountdownRange]);
    }

    #[test]
    fn every_error_has_a_code_and_message() {
        let all = [
            ValidationError::NameLength,
            ValidationError::IntervalRange,
            ValidationError::VariationBlockCount,
            ValidationError::VariationBlockRange { index: 1 },
            ValidationError::StopAfterTooShort,
            ValidationError::AmplitudeRange,
            ValidationError::WiggleSpeed,
            ValidationError::LineSpacingRange,
            ValidationError::ScanSpeed,
            ValidationError::ScanAreaTooSmall,
            ValidationError::ScanAreaTooLarge,
            ValidationError::PointCoordinateRange { index: 0 },
            ValidationError::TooManyPoints,
            ValidationError::PointDelayRange { index: 2 },
            ValidationError::CountdownRange,
            ValidationError::SchemaVersion(9),
        ];
        for e in all {
            assert!(!e.code().is_empty());
            assert!(!e.to_string().is_empty());
        }
    }
}
