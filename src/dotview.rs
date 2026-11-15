//! Per-pixel alpha layered-window dot renderer for Windows (`UpdateLayeredWindow` + 32-bit DIB).
//!
//! Replaces the Linux GTK4/GSK `DotView` widget with a lightweight software SDF circle
//! rasterizer backed by a Win32 premultiplied-BGRA DIB section. Only the dirty bounding
//! rectangles of the previous frame's dots are cleared each tick, keeping CPU usage
//! well under 1% at 60 FPS with zero external DLL dependencies.

/// A dot to draw: centre x and y, diameter, and alpha — all in screen pixels.
pub type Dot = (f32, f32, f32, f32);

#[derive(Clone, Copy)]
struct DirtyRect {
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
}

#[cfg(windows)]
use windows::Win32::Foundation::{COLORREF, HWND, POINT, SIZE};
#[cfg(windows)]
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject,
    AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS,
    HBITMAP, HDC, HGDIOBJ,
};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{UpdateLayeredWindow, ULW_ALPHA};

#[cfg(windows)]
pub struct DotView {
    width: i32,
    height: i32,
    mem_dc: HDC,
    dib_bitmap: HBITMAP,
    old_obj: HGDIOBJ,
    pixels_ptr: *mut u32,
    dirty_rects: Vec<DirtyRect>,
}

#[cfg(windows)]
impl DotView {
    /// Creates a new 32-bit premultiplied ARGB DIB backing surface for the given screen dimensions.
    pub fn new(width: i32, height: i32) -> Option<Self> {
        let width = width.max(1);
        let height = height.max(1);

        unsafe {
            let screen_dc = GetDC(HWND::default());
            let mem_dc = CreateCompatibleDC(screen_dc);
            let _ = ReleaseDC(HWND::default(), screen_dc);
            if mem_dc.is_invalid() {
                return None;
            }

            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    // Negative height creates a top-down DIB with origin at top-left.
                    biHeight: -height,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
            let Ok(dib_bitmap) =
                CreateDIBSection(mem_dc, &bmi, DIB_RGB_COLORS, &mut bits, None, 0)
            else {
                let _ = DeleteDC(mem_dc);
                return None;
            };

            if dib_bitmap.is_invalid() || bits.is_null() {
                let _ = DeleteDC(mem_dc);
                return None;
            }

            let old_obj = SelectObject(mem_dc, dib_bitmap);

            Some(Self {
                width,
                height,
                mem_dc,
                dib_bitmap,
                old_obj,
                pixels_ptr: bits as *mut u32,
                dirty_rects: Vec::with_capacity(96),
            })
        }
    }

    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    /// Rasterizes anti-aliased circles for `dots` and pushes the updated layer to `hwnd`.
    pub fn render(
        &mut self,
        hwnd: HWND,
        dst_x: i32,
        dst_y: i32,
        color: (f32, f32, f32),
        dots: &[Dot],
    ) {
        let w = self.width;
        let h = self.height;
        let stride = w as usize;
        let total_len = (w as usize) * (h as usize);

        let buf = unsafe { std::slice::from_raw_parts_mut(self.pixels_ptr, total_len) };

        // 1. Clear only the dirty rectangles from the previous frame.
        for rect in self.dirty_rects.drain(..) {
            for y in rect.y0..=rect.y1 {
                let row_start = (y as usize) * stride;
                let start = row_start + (rect.x0 as usize);
                let end = row_start + (rect.x1 as usize);
                buf[start..=end].fill(0);
            }
        }

        let (cr, cg, cb) = (
            color.0.clamp(0.0, 1.0),
            color.1.clamp(0.0, 1.0),
            color.2.clamp(0.0, 1.0),
        );

        // 2. Rasterize each dot with 1px signed-distance anti-aliasing at the boundary.
        for &(cx, cy, diameter, alpha) in dots {
            let base_alpha = alpha.clamp(0.0, 1.0);
            if base_alpha <= 0.001 || diameter <= 0.5 {
                continue;
            }

            let radius = diameter * 0.5;
            let x0 = ((cx - radius - 1.0).floor() as i32).clamp(0, w - 1);
            let y0 = ((cy - radius - 1.0).floor() as i32).clamp(0, h - 1);
            let x1 = ((cx + radius + 1.0).ceil() as i32).clamp(0, w - 1);
            let y1 = ((cy + radius + 1.0).ceil() as i32).clamp(0, h - 1);

            if x0 > x1 || y0 > y1 {
                continue;
            }

            self.dirty_rects.push(DirtyRect { x0, y0, x1, y1 });

            let inner_r = (radius - 0.75).max(0.0);
            let outer_r = radius + 0.75;
            let inner_r2 = inner_r * inner_r;
            let outer_r2 = outer_r * outer_r;

            for py in y0..=y1 {
                let dy = (py as f32 + 0.5) - cy;
                let dy2 = dy * dy;
                let row_offset = (py as usize) * stride;

                for px in x0..=x1 {
                    let dx = (px as f32 + 0.5) - cx;
                    let dist2 = dx * dx + dy2;

                    if dist2 >= outer_r2 {
                        continue;
                    }

                    let coverage = if dist2 <= inner_r2 {
                        1.0
                    } else {
                        let dist = dist2.sqrt();
                        (radius + 0.5 - dist).clamp(0.0, 1.0)
                    };

                    let src_a = base_alpha * coverage;
                    if src_a <= 0.001 {
                        continue;
                    }

                    let a_u8 = (src_a * 255.0 + 0.5) as u32;
                    let r_u8 = (cr * src_a * 255.0 + 0.5) as u32;
                    let g_u8 = (cg * src_a * 255.0 + 0.5) as u32;
                    let b_u8 = (cb * src_a * 255.0 + 0.5) as u32;

                    let idx = row_offset + (px as usize);
                    let dst = buf[idx];
                    if dst == 0 || a_u8 == 255 {
                        buf[idx] = (a_u8 << 24) | (r_u8 << 16) | (g_u8 << 8) | b_u8;
                    } else {
                        // Premultiplied source-over blend when two particles overlap.
                        let inv_a = 255 - a_u8;
                        let dst_a = (dst >> 24) & 0xFF;
                        let dst_r = (dst >> 16) & 0xFF;
                        let dst_g = (dst >> 8) & 0xFF;
                        let dst_b = dst & 0xFF;

                        let out_a = (a_u8 + (dst_a * inv_a + 127) / 255).min(255);
                        let out_r = (r_u8 + (dst_r * inv_a + 127) / 255).min(255);
                        let out_g = (g_u8 + (dst_g * inv_a + 127) / 255).min(255);
                        let out_b = (b_u8 + (dst_b * inv_a + 127) / 255).min(255);

                        buf[idx] = (out_a << 24) | (out_r << 16) | (out_g << 8) | out_b;
                    }
                }
            }
        }

        // 3. Push the updated premultiplied ARGB surface to the Win32 layered window.
        unsafe {
            let screen_dc = GetDC(HWND::default());
            let dst_pt = POINT { x: dst_x, y: dst_y };
            let src_pt = POINT { x: 0, y: 0 };
            let size = SIZE { cx: w, cy: h };
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };

            let _ = UpdateLayeredWindow(
                hwnd,
                screen_dc,
                Some(&dst_pt as *const POINT),
                Some(&size as *const SIZE),
                self.mem_dc,
                Some(&src_pt as *const POINT),
                COLORREF(0),
                Some(&blend as *const BLENDFUNCTION),
                ULW_ALPHA,
            );

            let _ = ReleaseDC(HWND::default(), screen_dc);
        }
    }
}

#[cfg(windows)]
impl Drop for DotView {
    fn drop(&mut self) {
        unsafe {
            let _ = SelectObject(self.mem_dc, self.old_obj);
            let _ = DeleteObject(self.dib_bitmap);
            let _ = DeleteDC(self.mem_dc);
        }
    }
}
