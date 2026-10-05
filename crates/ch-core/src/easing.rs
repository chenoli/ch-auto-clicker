//! CSS `cubic-bezier()` timing curves, so native animations (window height) follow the same easing
//! as the UI (`--ease-out: cubic-bezier(.2,.7,.2,1)`).

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CubicBezier {
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
}

/// The design system's `--ease-out`.
pub const EASE_OUT: CubicBezier = CubicBezier::new(0.2, 0.7, 0.2, 1.0);

impl CubicBezier {
    pub const fn new(x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        Self { x1, y1, x2, y2 }
    }

    fn sample(a1: f64, a2: f64, t: f64) -> f64 {
        // Bernstein form with P0 = 0 and P3 = 1.
        let u = 1.0 - t;
        3.0 * u * u * t * a1 + 3.0 * u * t * t * a2 + t * t * t
    }

    fn slope(a1: f64, a2: f64, t: f64) -> f64 {
        let u = 1.0 - t;
        3.0 * u * u * a1 + 6.0 * u * t * (a2 - a1) + 3.0 * t * t * (1.0 - a2)
    }

    /// Eased progress for linear progress `x` in 0..=1.
    pub fn at(&self, x: f64) -> f64 {
        let x = x.clamp(0.0, 1.0);
        if x == 0.0 || x == 1.0 {
            return x;
        }
        // Newton's method on x(t) = x, falling back to bisection where the slope is flat.
        let mut t = x;
        for _ in 0..8 {
            let err = Self::sample(self.x1, self.x2, t) - x;
            if err.abs() < 1e-7 {
                return Self::sample(self.y1, self.y2, t);
            }
            let d = Self::slope(self.x1, self.x2, t);
            if d.abs() < 1e-6 {
                break;
            }
            t -= err / d;
        }
        let (mut lo, mut hi) = (0.0, 1.0);
        t = x;
        for _ in 0..60 {
            let v = Self::sample(self.x1, self.x2, t);
            if (v - x).abs() < 1e-7 {
                break;
            }
            if v < x {
                lo = t;
            } else {
                hi = t;
            }
            t = (lo + hi) / 2.0;
        }
        Self::sample(self.y1, self.y2, t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_are_exact() {
        assert_eq!(EASE_OUT.at(0.0), 0.0);
        assert_eq!(EASE_OUT.at(1.0), 1.0);
        assert_eq!(EASE_OUT.at(-3.0), 0.0);
        assert_eq!(EASE_OUT.at(7.0), 1.0);
    }

    #[test]
    fn linear_curve_is_identity() {
        let linear = CubicBezier::new(0.25, 0.25, 0.75, 0.75);
        for i in 0..=10 {
            let x = f64::from(i) / 10.0;
            assert!((linear.at(x) - x).abs() < 1e-6);
        }
    }

    #[test]
    fn ease_out_front_loads_the_motion() {
        // Reference values solved by hand: x(t) = 0.6t(1 - t) + t³, y(t) = 2.1(1 - t)²t + 3(1 - t)t² + t³.
        assert!(
            (EASE_OUT.at(0.25) - 0.7274).abs() < 0.001,
            "{}",
            EASE_OUT.at(0.25)
        );
        assert!(
            (EASE_OUT.at(0.5) - 0.9296).abs() < 0.001,
            "{}",
            EASE_OUT.at(0.5)
        );
        let mut prev = 0.0;
        for i in 1..=100 {
            let v = EASE_OUT.at(f64::from(i) / 100.0);
            assert!(v >= prev, "monotonic");
            prev = v;
        }
    }

    #[test]
    fn steep_curves_fall_back_to_bisection() {
        let steep = CubicBezier::new(0.0, 0.0, 0.0, 1.0);
        let v = steep.at(0.001);
        assert!((0.0..=1.0).contains(&v));
    }
}
