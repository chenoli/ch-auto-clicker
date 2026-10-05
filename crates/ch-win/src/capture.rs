use ch_core::geometry::Rect;
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CAPTUREBLT, CreateCompatibleBitmap,
    CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, GetDIBits, ReleaseDC,
    SRCCOPY, SelectObject,
};

const FILE_HEADER: usize = 14;
const INFO_HEADER: usize = 40;

pub fn capture_bmp(rect: Rect) -> windows::core::Result<Vec<u8>> {
    let (w, h) = (rect.width, rect.height);
    if w <= 0 || h <= 0 {
        return Err(windows::core::Error::empty());
    }
    let mut pixels = vec![0u8; w as usize * h as usize * 4];
    // SAFETY: every GDI object created here is selected out and deleted before returning; the
    // pixel buffer is exactly w*h*4 bytes, matching the 32 bpp DIB described in `info`.
    unsafe {
        let screen = GetDC(None);
        let memory = CreateCompatibleDC(Some(screen));
        let bitmap = CreateCompatibleBitmap(screen, w, h);
        let previous = SelectObject(memory, bitmap.into());
        let copied = BitBlt(
            memory,
            0,
            0,
            w,
            h,
            Some(screen),
            rect.left,
            rect.top,
            SRCCOPY | CAPTUREBLT,
        );
        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: INFO_HEADER as u32,
                biWidth: w,
                biHeight: -h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let lines = GetDIBits(
            memory,
            bitmap,
            0,
            h as u32,
            Some(pixels.as_mut_ptr().cast()),
            &mut info,
            DIB_RGB_COLORS,
        );
        SelectObject(memory, previous);
        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(memory);
        ReleaseDC(None, screen);
        copied?;
        if lines == 0 {
            return Err(windows::core::Error::from_thread());
        }
    }
    for px in pixels.as_chunks_mut::<4>().0 {
        px[3] = 0xFF;
    }
    Ok(bmp_file(w, h, &pixels))
}

fn bmp_file(w: i32, h: i32, pixels: &[u8]) -> Vec<u8> {
    let offset = FILE_HEADER + INFO_HEADER;
    let size = offset + pixels.len();
    let mut out = Vec::with_capacity(size);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&(size as u32).to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&(offset as u32).to_le_bytes());
    out.extend_from_slice(&(INFO_HEADER as u32).to_le_bytes());
    out.extend_from_slice(&w.to_le_bytes());
    out.extend_from_slice(&(-h).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(&[0; 24]);
    out.extend_from_slice(pixels);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::display::{display_layout, ensure_per_monitor_v2};

    #[test]
    fn captures_a_region_as_a_valid_bmp() {
        ensure_per_monitor_v2();
        let primary = display_layout().primary().expect("primary").rect;
        let rect = Rect::new(primary.left, primary.top, 64, 32);
        let bmp = capture_bmp(rect).expect("capture");
        assert_eq!(&bmp[..2], b"BM");
        assert_eq!(bmp.len(), 54 + 64 * 32 * 4);
        assert_eq!(
            u32::from_le_bytes(bmp[2..6].try_into().expect("size")),
            bmp.len() as u32
        );
        assert_eq!(i32::from_le_bytes(bmp[18..22].try_into().expect("w")), 64);
        assert_eq!(
            i32::from_le_bytes(bmp[22..26].try_into().expect("h")),
            -32,
            "top-down"
        );
        assert!(
            bmp[54..].as_chunks::<4>().0.iter().all(|px| px[3] == 0xFF),
            "opaque pixels"
        );
    }

    #[test]
    fn rejects_empty_regions() {
        assert!(capture_bmp(Rect::new(0, 0, 0, 10)).is_err());
    }
}
