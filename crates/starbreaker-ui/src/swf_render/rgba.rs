use image::RgbaImage;
use tiny_skia::Pixmap;

/// Composite straight-alpha RGBA image over premultiplied pixmap.
///
/// Pixels from `src` (straight alpha) are blended on top of `dst`
/// (premultiplied alpha).  Only pixels with non-zero source alpha are written,
/// so a sparse text-only image composites efficiently.
pub(super) fn composite_rgba_over_pixmap(src: &RgbaImage, dst: &mut Pixmap) {
    let dst_w = dst.width();
    let dst_h = dst.height();
    let w = src.width().min(dst_w);
    let h = src.height().min(dst_h);
    let dst_data = dst.data_mut();

    for y in 0..h {
        for x in 0..w {
            let px = src.get_pixel(x, y);
            let sa = px[3] as u32;
            if sa == 0 {
                continue;
            }
            let idx = ((y * dst_w + x) as usize) * 4;
            // Convert src from straight to premultiplied
            let r_pm = px[0] as u32 * sa / 255;
            let g_pm = px[1] as u32 * sa / 255;
            let b_pm = px[2] as u32 * sa / 255;

            // Compose src_pm over dst_pm: out = src_pm + dst_pm * (1 - sa/255)
            let inv = 255 - sa;
            dst_data[idx] = (r_pm + dst_data[idx] as u32 * inv / 255).min(255) as u8;
            dst_data[idx + 1] = (g_pm + dst_data[idx + 1] as u32 * inv / 255).min(255) as u8;
            dst_data[idx + 2] = (b_pm + dst_data[idx + 2] as u32 * inv / 255).min(255) as u8;
            dst_data[idx + 3] = (sa + dst_data[idx + 3] as u32 * inv / 255).min(255) as u8;
        }
    }
}

/// Composite premultiplied pixmap over straight-alpha RGBA image.
pub(super) fn composite_pixmap_over_rgba(pixmap: &Pixmap, img: &mut RgbaImage) {
    let w = img.width();
    let h = img.height();
    let pix = pixmap.data();
    for y in 0..h {
        for x in 0..w {
            let idx = ((y * w + x) as usize) * 4;
            let a_top = pix[idx + 3] as u32;
            if a_top == 0 {
                continue;
            }
            let r_top = ((pix[idx] as u32 * 255) / a_top.max(1)).min(255);
            let g_top = ((pix[idx + 1] as u32 * 255) / a_top.max(1)).min(255);
            let b_top = ((pix[idx + 2] as u32 * 255) / a_top.max(1)).min(255);

            let base = img.get_pixel(x, y);
            let ba = base[3] as u32;

            let out_a = (a_top + ba * (255 - a_top) / 255).min(255);
            if out_a == 0 {
                img.put_pixel(x, y, image::Rgba([0, 0, 0, 0]));
            } else {
                let blend = |top: u32, bot: u32| -> u8 {
                    ((top * a_top / 255 + bot * ba * (255 - a_top) / 255 / 255) * 255 / out_a)
                        .min(255) as u8
                };
                img.put_pixel(
                    x,
                    y,
                    image::Rgba([
                        blend(r_top, base[0] as u32),
                        blend(g_top, base[1] as u32),
                        blend(b_top, base[2] as u32),
                        out_a as u8,
                    ]),
                );
            }
        }
    }
}
