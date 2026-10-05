const SS: u32 = 4;

const TILE: [u8; 3] = [0x14, 0x14, 0x12];
const INK: [u8; 3] = [0xF0, 0xED, 0xE5];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Paint {
    Clear,
    Tile,
    Ink,
    Dot,
}

fn in_rounded_rect(x: f64, y: f64, r: f64) -> bool {
    let cx = x.clamp(r, 1.0 - r);
    let cy = y.clamp(r, 1.0 - r);
    (x - cx).powi(2) + (y - cy).powi(2) <= r * r
}

fn in_c(x: f64, y: f64) -> bool {
    let (dx, dy) = (x - 0.34, y - 0.5);
    let d = dx.hypot(dy);
    (0.105..=0.21).contains(&d) && !(dx > 0.0 && dy.abs() < dx)
}

fn in_h(x: f64, y: f64) -> bool {
    let tall = (0.29..=0.71).contains(&y);
    let left = (0.55..=0.635).contains(&x);
    let right = (0.735..=0.82).contains(&x);
    let bar = (0.55..=0.82).contains(&x) && (0.455..=0.545).contains(&y);
    (tall && (left || right)) || bar
}

fn paint_at(x: f64, y: f64, dot: bool) -> Paint {
    let (cx, cy) = (0.8, 0.8);
    let d = (x - cx).hypot(y - cy);
    if dot && d <= 0.125 {
        return Paint::Dot;
    }
    if dot && d <= 0.18 {
        return Paint::Tile;
    }
    if !in_rounded_rect(x, y, 0.22) {
        return Paint::Clear;
    }
    if in_c(x, y) || in_h(x, y) {
        Paint::Ink
    } else {
        Paint::Tile
    }
}

pub fn render(size: u32, dot: Option<[u8; 3]>) -> Vec<u8> {
    let mut out = Vec::with_capacity((size * size * 4) as usize);
    let n = f64::from(size * SS);
    for py in 0..size {
        for px in 0..size {
            let mut sum = [0u32; 4];
            for sy in 0..SS {
                for sx in 0..SS {
                    let x = (f64::from(px * SS + sx) + 0.5) / n;
                    let y = (f64::from(py * SS + sy) + 0.5) / n;
                    let color = match paint_at(x, y, dot.is_some()) {
                        Paint::Clear => continue,
                        Paint::Tile => TILE,
                        Paint::Ink => INK,
                        Paint::Dot => dot.unwrap_or(INK),
                    };
                    for c in 0..3 {
                        sum[c] += u32::from(color[c]);
                    }
                    sum[3] += 1;
                }
            }
            let covered = sum[3];
            if covered == 0 {
                out.extend_from_slice(&[0, 0, 0, 0]);
            } else {
                out.extend(sum[..3].iter().map(|v| (v / covered) as u8));
                out.push((covered * 255 / (SS * SS)) as u8);
            }
        }
    }
    out
}

pub fn rgb(hex: &str) -> Option<[u8; 3]> {
    let byte = |i: usize| {
        hex.get(i..i + 2)
            .and_then(|b| u8::from_str_radix(b, 16).ok())
    };
    (hex.len() == 7 && hex.starts_with('#')).then_some(())?;
    Some([byte(1)?, byte(3)?, byte(5)?])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel(img: &[u8], size: u32, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * size + x) * 4) as usize;
        [img[i], img[i + 1], img[i + 2], img[i + 3]]
    }

    #[test]
    fn draws_every_size_with_transparent_corners() {
        for size in [16, 20, 24, 32] {
            let img = render(size, Some([0, 218, 126]));
            assert_eq!(img.len(), (size * size * 4) as usize);
            assert_eq!(pixel(&img, size, 0, 0)[3], 0, "rounded corner");
            assert_eq!(pixel(&img, size, size / 2, 1)[3], 255, "tile edge");
        }
    }

    #[test]
    fn the_dot_carries_the_state_color_and_can_pulse_off() {
        let size = 32;
        let on = render(size, Some([0xFF, 0x64, 0x5F]));
        let at_dot = pixel(&on, size, 25, 25);
        assert_eq!(at_dot, [0xFF, 0x64, 0x5F, 255]);
        let off = render(size, None);
        assert_ne!(pixel(&off, size, 25, 25), at_dot);
        assert_ne!(on, off);
    }

    #[test]
    fn the_monogram_is_ink_on_the_tile() {
        let size = 32;
        let img = render(size, None);
        assert_eq!(pixel(&img, size, 6, 16), [0xF0, 0xED, 0xE5, 255]);
        assert_eq!(pixel(&img, size, 19, 12), [0xF0, 0xED, 0xE5, 255]);
        assert_eq!(pixel(&img, size, 11, 16), [0x14, 0x14, 0x12, 255]);
    }

    #[test]
    #[ignore = "writes files; run on demand"]
    fn dump_previews() {
        let dir = std::env::var("CH_TRAY_PREVIEW_DIR").expect("CH_TRAY_PREVIEW_DIR");
        for size in [16u32, 20, 24, 32] {
            for (name, dot) in [
                ("idle", Some([0x6A, 0x67, 0x60])),
                ("run", Some([0x00, 0xDA, 0x7E])),
                ("off", None),
            ] {
                std::fs::write(format!("{dir}/tray-{size}-{name}.rgba"), render(size, dot))
                    .expect("write");
            }
        }
    }

    #[test]
    fn parses_palette_colors() {
        assert_eq!(rgb("#00DA7E"), Some([0x00, 0xDA, 0x7E]));
        assert_eq!(rgb("00DA7E"), None);
        assert_eq!(rgb("#00DA7"), None);
        assert_eq!(rgb("#GG0000"), None);
    }
}
