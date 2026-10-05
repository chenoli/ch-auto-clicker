//! Scan area thumbnails (TASKS T7.4): the selected area cropped from the overlay's frozen capture,
//! scaled down and kept in `cache/thumbs/<profile>.bmp` for the Scan panel (Tela 01.C).

use std::path::PathBuf;

/// Twice the panel's 388 × 130 box, so the thumbnail stays sharp at 200 % scaling.
pub const MAX_W: u32 = 776;
pub const MAX_H: u32 = 260;

const FILE_HEADER: usize = 14;
const INFO_HEADER: usize = 40;

pub fn dir() -> PathBuf {
    crate::paths::cache_dir().join("thumbs")
}

/// Thumbnail path for a profile, or `None` for ids that are not safe file names.
pub fn path_for(profile_id: &str) -> Option<PathBuf> {
    let safe = !profile_id.is_empty()
        && profile_id.len() <= 64
        && profile_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    safe.then(|| dir().join(format!("{profile_id}.bmp")))
}

/// Writes a profile's thumbnail (temp file + rename, so the panel never reads half a file).
pub fn save(profile_id: &str, bytes: &[u8]) -> std::io::Result<()> {
    let path = path_for(profile_id).ok_or_else(|| std::io::Error::other("unsafe profile id"))?;
    save_to(&path, bytes)
}

fn save_to(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("bmp.tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

/// A decoded 32-bit top-down BMP, as written by `ch_win::capture`.
struct Image<'a> {
    width: u32,
    height: u32,
    pixels: &'a [u8],
}

fn read_i32(bytes: &[u8], at: usize) -> Option<i32> {
    Some(i32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

fn decode(bmp: &[u8]) -> Option<Image<'_>> {
    if bmp.get(0..2)? != b"BM" || u16::from_le_bytes(bmp.get(28..30)?.try_into().ok()?) != 32 {
        return None;
    }
    let offset = usize::try_from(read_i32(bmp, 10)?).ok()?;
    let width = u32::try_from(read_i32(bmp, 18)?).ok()?;
    // Top-down bitmaps have a negative height.
    let height = read_i32(bmp, 22)?
        .checked_neg()
        .and_then(|h| u32::try_from(h).ok())?;
    let len = width as usize * height as usize * 4;
    Some(Image {
        width,
        height,
        pixels: bmp.get(offset..offset + len)?,
    })
}

fn encode(width: u32, height: u32, pixels: &[u8]) -> Vec<u8> {
    let offset = FILE_HEADER + INFO_HEADER;
    let size = offset + pixels.len();
    let mut out = Vec::with_capacity(size);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&(size as u32).to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&(offset as u32).to_le_bytes());
    out.extend_from_slice(&(INFO_HEADER as u32).to_le_bytes());
    out.extend_from_slice(&(width as i32).to_le_bytes());
    out.extend_from_slice(&(-(height as i32)).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(&[0; 24]);
    out.extend_from_slice(pixels);
    out
}

/// Crops `(x, y, w, h)` (pixels inside the capture) and scales it to fit `max_w × max_h` by
/// averaging each block of source pixels. `None` when the BMP or the rectangle is unusable.
pub fn crop_scaled(
    bmp: &[u8],
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    max_w: u32,
    max_h: u32,
) -> Option<Vec<u8>> {
    let image = decode(bmp)?;
    if w == 0 || h == 0 || x.checked_add(w)? > image.width || y.checked_add(h)? > image.height {
        return None;
    }
    let ratio = (f64::from(w) / f64::from(max_w))
        .max(f64::from(h) / f64::from(max_h))
        .max(1.0);
    let out_w = ((f64::from(w) / ratio).round() as u32).max(1);
    let out_h = ((f64::from(h) / ratio).round() as u32).max(1);
    let mut out = Vec::with_capacity(out_w as usize * out_h as usize * 4);
    for oy in 0..out_h {
        let sy0 = y + oy * h / out_h;
        let sy1 = (y + (oy + 1) * h / out_h).max(sy0 + 1);
        for ox in 0..out_w {
            let sx0 = x + ox * w / out_w;
            let sx1 = (x + (ox + 1) * w / out_w).max(sx0 + 1);
            let mut sum = [0u64; 3];
            for sy in sy0..sy1 {
                let row = sy as usize * image.width as usize;
                for sx in sx0..sx1 {
                    let i = (row + sx as usize) * 4;
                    for (c, total) in sum.iter_mut().enumerate() {
                        *total += u64::from(image.pixels[i + c]);
                    }
                }
            }
            let n = u64::from(sy1 - sy0) * u64::from(sx1 - sx0);
            out.extend(sum.iter().map(|v| (v / n) as u8));
            out.push(0xFF);
        }
    }
    Some(encode(out_w, out_h, &out))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 4 × 2 image: left half red, right half blue (BGRA).
    fn sample() -> Vec<u8> {
        let mut px = Vec::new();
        for _ in 0..2 {
            for x in 0..4 {
                px.extend_from_slice(if x < 2 {
                    &[0, 0, 255, 255]
                } else {
                    &[255, 0, 0, 255]
                });
            }
        }
        encode(4, 2, &px)
    }

    #[test]
    fn crops_and_averages_blocks() {
        let bmp = sample();
        let whole = crop_scaled(&bmp, 0, 0, 4, 2, 2, 1).expect("thumb");
        let img = decode(&whole).expect("decode");
        assert_eq!((img.width, img.height), (2, 1));
        assert_eq!(&img.pixels[0..4], &[0, 0, 255, 255]);
        assert_eq!(&img.pixels[4..8], &[255, 0, 0, 255]);
        // Mixed block: one red and one blue column average to purple.
        let mixed = crop_scaled(&bmp, 1, 0, 2, 2, 1, 1).expect("thumb");
        assert_eq!(
            &decode(&mixed).expect("decode").pixels[0..4],
            &[127, 0, 127, 255]
        );
    }

    #[test]
    fn never_upscales_and_rejects_bad_input() {
        let bmp = sample();
        let same = crop_scaled(&bmp, 2, 1, 2, 1, MAX_W, MAX_H).expect("thumb");
        let img = decode(&same).expect("decode");
        assert_eq!((img.width, img.height), (2, 1));
        assert!(
            crop_scaled(&bmp, 3, 0, 2, 2, 10, 10).is_none(),
            "outside the capture"
        );
        assert!(crop_scaled(&bmp, 0, 0, 0, 2, 10, 10).is_none(), "empty");
        assert!(crop_scaled(b"nope", 0, 0, 1, 1, 10, 10).is_none());
        let mut bottom_up = bmp.clone();
        bottom_up[22..26].copy_from_slice(&2i32.to_le_bytes());
        assert!(crop_scaled(&bottom_up, 0, 0, 1, 1, 10, 10).is_none());
    }

    #[test]
    fn saves_into_a_fresh_directory_and_replaces_old_files() {
        let dir = std::env::temp_dir().join(format!("ch-thumbs-{}", std::process::id()));
        let path = dir.join("nested").join("p.bmp");
        save_to(&path, b"one").expect("first save");
        save_to(&path, b"two").expect("replace");
        assert_eq!(std::fs::read(&path).expect("read"), b"two");
        assert!(!path.with_extension("bmp.tmp").exists());
        std::fs::remove_dir_all(&dir).expect("cleanup");
        assert!(save("../evil", b"x").is_err());
    }

    #[test]
    fn only_safe_ids_get_a_path() {
        assert!(path_for("3f2a-uuid_01").is_some_and(|p| p.ends_with("3f2a-uuid_01.bmp")));
        assert!(path_for("../evil").is_none());
        assert!(path_for("").is_none());
        assert!(path_for(&"a".repeat(65)).is_none());
    }
}
