pub const NORMALIZED_MAX: i32 = 65535;
const NORMALIZED_SPAN: i64 = 65536;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub const fn new(left: i32, top: i32, width: i32, height: i32) -> Self {
        Self {
            left,
            top,
            width,
            height,
        }
    }

    pub const fn from_edges(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self::new(left, top, right - left, bottom - top)
    }

    pub const fn right(&self) -> i32 {
        self.left + self.width
    }

    pub const fn bottom(&self) -> i32 {
        self.top + self.height
    }

    pub const fn is_empty(&self) -> bool {
        self.width <= 0 || self.height <= 0
    }

    pub const fn contains(&self, p: Point) -> bool {
        p.x >= self.left && p.x < self.right() && p.y >= self.top && p.y < self.bottom()
    }

    pub const fn contains_rect(&self, other: &Rect) -> bool {
        other.left >= self.left
            && other.top >= self.top
            && other.right() <= self.right()
            && other.bottom() <= self.bottom()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Normalized {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum GeometryError {
    #[error("point ({x}, {y}) is outside the virtual desktop")]
    OutsideDesktop { x: i32, y: i32 },
    #[error("virtual desktop is empty or wider than the normalized range")]
    InvalidDesktop,
}

pub fn normalize(p: Point, desktop: Rect) -> Result<Normalized, GeometryError> {
    if desktop.is_empty()
        || i64::from(desktop.width) > NORMALIZED_SPAN
        || i64::from(desktop.height) > NORMALIZED_SPAN
    {
        return Err(GeometryError::InvalidDesktop);
    }
    if !desktop.contains(p) {
        return Err(GeometryError::OutsideDesktop { x: p.x, y: p.y });
    }
    Ok(Normalized {
        x: normalize_axis(p.x - desktop.left, desktop.width),
        y: normalize_axis(p.y - desktop.top, desktop.height),
    })
}

pub fn denormalize(n: Normalized, desktop: Rect) -> Point {
    Point {
        x: desktop.left + denormalize_axis(n.x, desktop.width),
        y: desktop.top + denormalize_axis(n.y, desktop.height),
    }
}

fn normalize_axis(offset: i32, size: i32) -> i32 {
    let scaled = i64::from(offset) * NORMALIZED_SPAN;
    let size = i64::from(size);
    let value = (scaled + size - 1) / size;
    value as i32
}

fn denormalize_axis(value: i32, size: i32) -> i32 {
    ((i64::from(value) * i64::from(size)) / NORMALIZED_SPAN) as i32
}

#[derive(Debug, Clone, PartialEq)]
pub struct Monitor {
    pub id: String,
    pub rect: Rect,
    pub work_area: Rect,
    pub scale: f32,
    pub primary: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DisplayLayout {
    pub virtual_desktop: Rect,
    pub monitors: Vec<Monitor>,
}

impl DisplayLayout {
    pub fn monitor_at(&self, p: Point) -> Option<&Monitor> {
        self.monitors.iter().find(|m| m.rect.contains(p))
    }

    pub fn primary(&self) -> Option<&Monitor> {
        self.monitors.iter().find(|m| m.primary)
    }

    pub fn fingerprint(&self) -> String {
        let mut rects: Vec<Rect> = self.monitors.iter().map(|m| m.rect).collect();
        rects.sort_by_key(|r| (r.left, r.top));
        let parts: Vec<String> = rects
            .iter()
            .map(|r| format!("{}x{}@{},{}", r.width, r.height, r.left, r.top))
            .collect();
        format!("{}mon:{}", rects.len(), parts.join(";"))
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    const FHD: Rect = Rect::new(0, 0, 1920, 1080);

    #[test]
    fn rect_edges_are_exclusive() {
        let r = Rect::from_edges(-1680, -180, 0, 870);
        assert_eq!((r.width, r.height), (1680, 1050));
        assert!(r.contains(Point::new(-1680, -180)));
        assert!(r.contains(Point::new(-1, 869)));
        assert!(!r.contains(Point::new(0, 0)));
        assert!(!r.contains(Point::new(-1, 870)));
    }

    #[test]
    fn contains_rect_checks_every_edge() {
        let outer = Rect::new(0, 0, 100, 100);
        assert!(outer.contains_rect(&Rect::new(0, 0, 100, 100)));
        assert!(outer.contains_rect(&Rect::new(10, 10, 20, 20)));
        assert!(!outer.contains_rect(&Rect::new(-1, 0, 10, 10)));
        assert!(!outer.contains_rect(&Rect::new(95, 95, 10, 10)));
    }

    #[test]
    fn normalizes_the_corners_of_a_full_hd_desktop() {
        assert_eq!(
            normalize(Point::new(0, 0), FHD),
            Ok(Normalized { x: 0, y: 0 })
        );
        let last = normalize(Point::new(1919, 1079), FHD).expect("inside");
        assert!(last.x <= NORMALIZED_MAX && last.y <= NORMALIZED_MAX);
        assert_eq!(denormalize(last, FHD), Point::new(1919, 1079));
    }

    #[test]
    fn handles_negative_origins() {
        let desk = Rect::from_edges(-1680, -180, 4480, 1440);
        let p = Point::new(-840, 610);
        assert_eq!(denormalize(normalize(p, desk).expect("inside"), desk), p);
    }

    #[test]
    fn rejects_points_outside_the_desktop() {
        assert_eq!(
            normalize(Point::new(1920, 0), FHD),
            Err(GeometryError::OutsideDesktop { x: 1920, y: 0 })
        );
        assert_eq!(
            normalize(Point::new(0, 0), Rect::new(0, 0, 0, 10)),
            Err(GeometryError::InvalidDesktop)
        );
        assert_eq!(
            normalize(Point::new(0, 0), Rect::new(0, 0, 70_000, 10)),
            Err(GeometryError::InvalidDesktop)
        );
    }

    #[test]
    fn finds_monitors_by_point() {
        let layout = DisplayLayout {
            virtual_desktop: Rect::from_edges(-1680, 0, 1920, 1080),
            monitors: vec![
                Monitor {
                    id: "\\\\.\\DISPLAY1".into(),
                    rect: FHD,
                    work_area: Rect::new(0, 0, 1920, 1040),
                    scale: 1.0,
                    primary: true,
                },
                Monitor {
                    id: "\\\\.\\DISPLAY2".into(),
                    rect: Rect::new(-1680, 0, 1680, 1050),
                    work_area: Rect::new(-1680, 0, 1680, 1050),
                    scale: 1.25,
                    primary: false,
                },
            ],
        };
        assert_eq!(
            layout.monitor_at(Point::new(-10, 10)).map(|m| m.scale),
            Some(1.25)
        );
        assert_eq!(layout.monitor_at(Point::new(-10, 1060)), None);
        assert_eq!(
            layout.primary().map(|m| m.id.as_str()),
            Some("\\\\.\\DISPLAY1")
        );
        assert_eq!(layout.fingerprint(), "2mon:1680x1050@-1680,0;1920x1080@0,0");
    }

    fn desktop_and_point() -> impl Strategy<Value = (Rect, Point)> {
        (
            -20_000i32..20_000,
            -20_000i32..20_000,
            1i32..=65_536,
            1i32..=65_536,
        )
            .prop_flat_map(|(left, top, width, height)| {
                let desk = Rect::new(left, top, width, height);
                (Just(desk), (left..left + width, top..top + height))
                    .prop_map(|(d, (x, y))| (d, Point::new(x, y)))
            })
    }

    proptest! {
        #[test]
        fn round_trip_is_exact((desk, p) in desktop_and_point()) {
            let n = normalize(p, desk).expect("point is inside");
            prop_assert!((0..=NORMALIZED_MAX).contains(&n.x));
            prop_assert!((0..=NORMALIZED_MAX).contains(&n.y));
            prop_assert_eq!(denormalize(n, desk), p);
        }
    }
}
