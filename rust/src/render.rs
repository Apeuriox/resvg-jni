use resvg::tiny_skia::{self, ColorU8, Pixmap};
use resvg::usvg::{self};

/// JPEG quality used when the caller does not pass one. Range 1–100.
pub const DEFAULT_JPEG_QUALITY: u8 = 90;

/// Opaque background used when the caller does not pass one. JPEG has no alpha.
pub const DEFAULT_JPEG_BACKGROUND: [u8; 3] = [255, 255, 255];

#[derive(Clone, Copy)]
pub struct JpegParams {
    pub quality: u8,
    pub background: [u8; 3],
}

impl Default for JpegParams {
    fn default() -> Self {
        Self {
            quality: DEFAULT_JPEG_QUALITY,
            background: DEFAULT_JPEG_BACKGROUND,
        }
    }
}

impl JpegParams {
    pub fn new(quality: u8, background: [u8; 3]) -> Self {
        Self {
            quality: quality.clamp(1, 100),
            background,
        }
    }
}

/// `quality` is 1–100. Non-finite values use [`DEFAULT_JPEG_QUALITY`].
pub fn jpeg_quality_from_f32(quality: f32) -> u8 {
    if !quality.is_finite() {
        return DEFAULT_JPEG_QUALITY;
    }
    quality.round().clamp(1.0, 100.0) as u8
}

/// Low 24 bits are `0xRRGGBB`.
pub fn background_from_rgb(rgb: u32) -> [u8; 3] {
    [
        ((rgb >> 16) & 0xff) as u8,
        ((rgb >> 8) & 0xff) as u8,
        (rgb & 0xff) as u8,
    ]
}

#[derive(Clone, Copy)]
pub enum RenderType {
    Png,
    Jpeg(JpegParams),
}

pub fn render(
    xmldata: &str,
    opt: &usvg::Options,
    scale: f32,
    render_type: RenderType,
) -> anyhow::Result<Vec<u8>> {
    let svg = usvg::Tree::from_str(xmldata, opt)?;
    let size = svg
        .size()
        .to_int_size()
        .scale_by(scale)
        .ok_or_else(|| anyhow::anyhow!("svg size scaled to zero"))?;
    let mut canvas = Pixmap::new(size.width(), size.height())
        .ok_or_else(|| anyhow::anyhow!("failed to allocate pixmap"))?;
    resvg::render(
        &svg,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut canvas.as_mut(),
    );

    Ok(match render_type {
        // PNG keeps straight alpha. Do not flatten onto a background.
        RenderType::Png => canvas.encode_png()?,
        RenderType::Jpeg(params) => encode_jpg(&canvas, params)?,
    })
}

/// Un-premultiply tiny-skia's pixels, composite onto an opaque background, then
/// encode. Both platforms use `image`'s JPEG encoder, whose frame header writes
/// horizontal and vertical sampling factors of 1 (4:4:4).
pub fn encode_jpg(map: &Pixmap, params: JpegParams) -> anyhow::Result<Vec<u8>> {
    let rgb = unpremultiply_and_composite(map, params.background);
    let mut writer = std::io::Cursor::new(Vec::new());
    let mut encoder =
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut writer, params.quality);
    encoder.encode(
        &rgb,
        map.width(),
        map.height(),
        image::ExtendedColorType::Rgb8,
    )?;
    Ok(writer.into_inner())
}

fn unpremultiply_and_composite(pixmap: &Pixmap, background: [u8; 3]) -> Vec<u8> {
    let mut rgb = Vec::with_capacity(pixmap.pixels().len() * 3);
    for pixel in pixmap.pixels() {
        let straight = pixel.demultiply();
        let (r, g, b) = composite_onto(straight, background);
        rgb.push(r);
        rgb.push(g);
        rgb.push(b);
    }
    rgb
}

fn composite_onto(straight: ColorU8, background: [u8; 3]) -> (u8, u8, u8) {
    let alpha = u32::from(straight.alpha());
    if alpha == 255 {
        return (straight.red(), straight.green(), straight.blue());
    }
    if alpha == 0 {
        return (background[0], background[1], background[2]);
    }
    let inverse = 255 - alpha;
    let blend = |src: u8, bg: u8| -> u8 {
        ((u32::from(src) * alpha + u32::from(bg) * inverse + 127) / 255) as u8
    };
    (
        blend(straight.red(), background[0]),
        blend(straight.green(), background[1]),
        blend(straight.blue(), background[2]),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render_option::RenderOptions;
    use resvg::tiny_skia::PremultipliedColorU8;
    use std::sync::{Arc, Barrier};
    use std::thread;
    use std::time::Duration;

    fn fill(pixmap: &mut Pixmap, color: PremultipliedColorU8) {
        for pixel in pixmap.pixels_mut() {
            *pixel = color;
        }
    }

    #[test]
    fn transparent_pixels_composite_to_white_not_black() {
        let mut pixmap = Pixmap::new(16, 16).unwrap();
        fill(
            &mut pixmap,
            PremultipliedColorU8::from_rgba(0, 0, 0, 0).unwrap(),
        );
        let jpeg = encode_jpg(&pixmap, JpegParams::default()).unwrap();
        let image = image::load_from_memory(&jpeg).unwrap().to_rgb8();
        for pixel in image.pixels() {
            assert!(pixel[0] >= 250 && pixel[1] >= 250 && pixel[2] >= 250);
        }
    }

    #[test]
    fn half_red_is_unpremultiplied_before_white_composite() {
        // Straight (255, 0, 0, 128) stored premultiplied is about (128, 0, 0, 128).
        // Dropping alpha without un-premultiplying yields a dark red.
        let mut pixmap = Pixmap::new(16, 16).unwrap();
        fill(
            &mut pixmap,
            PremultipliedColorU8::from_rgba(128, 0, 0, 128).unwrap(),
        );
        let jpeg = encode_jpg(&pixmap, JpegParams::default()).unwrap();
        let image = image::load_from_memory(&jpeg).unwrap().to_rgb8();
        let pixel = image.get_pixel(8, 8).0;
        assert!(pixel[0] > 220, "red {}", pixel[0]);
        assert!((90..170).contains(&pixel[1]), "green {}", pixel[1]);
        assert!((90..170).contains(&pixel[2]), "blue {}", pixel[2]);
    }

    #[test]
    fn jpeg_is_444_at_quality_90() {
        let pixmap = Pixmap::new(8, 8).unwrap();
        let jpeg = encode_jpg(&pixmap, JpegParams::default()).unwrap();
        assert_eq!(jpeg_sampling_factors(&jpeg), vec![0x11, 0x11, 0x11]);
        assert_eq!(jpeg_luma_quant_first(&jpeg), 3);
    }

    #[test]
    fn custom_background_and_quality_are_honored() {
        let mut pixmap = Pixmap::new(8, 8).unwrap();
        fill(
            &mut pixmap,
            PremultipliedColorU8::from_rgba(0, 0, 0, 0).unwrap(),
        );
        let jpeg = encode_jpg(
            &pixmap,
            JpegParams::new(75, background_from_rgb(0x2040ff)),
        )
        .unwrap();
        let image = image::load_from_memory(&jpeg).unwrap().to_rgb8();
        let pixel = image.get_pixel(4, 4).0;
        assert!(pixel[2] > pixel[0] && pixel[2] > pixel[1]);
        assert_eq!(jpeg_luma_quant_first(&jpeg), 8);
    }

    #[test]
    fn png_keeps_transparency_and_jpeg_flattens_edges() {
        let options = RenderOptions::new(String::new());
        load_text_fonts(&options);
        let svg = sample_svg();

        let png = options
            .with_options(|opt| render(svg, opt, 2.0, RenderType::Png))
            .unwrap();
        let jpeg = options
            .with_options(|opt| render(svg, opt, 2.0, RenderType::Jpeg(JpegParams::default())))
            .unwrap();

        let png_image = image::load_from_memory(&png).unwrap().to_rgba8();
        let jpeg_image = image::load_from_memory(&jpeg).unwrap().to_rgb8();
        assert_eq!(png_image.dimensions(), jpeg_image.dimensions());

        let corner = png_image.get_pixel(0, 0).0;
        assert_eq!(corner[3], 0, "png corner alpha");
        let jpeg_corner = jpeg_image.get_pixel(0, 0).0;
        assert!(
            jpeg_corner[0] >= 245 && jpeg_corner[1] >= 245 && jpeg_corner[2] >= 245,
            "jpeg corner {jpeg_corner:?}"
        );

        // Center of the translucent blue rect, away from the red overlay.
        // Dropping premultiplied alpha would leave this near (15, 46, 115).
        let blue = jpeg_image.get_pixel(48, 48).0;
        assert!(
            blue[2] > 220 && blue[0] > 100 && blue[0] < 200,
            "translucent blue flattened dark: {blue:?}"
        );

        let mut transparent = 0u32;
        let mut edge = 0u32;
        let mut max_edge_error = 0u8;
        let mut painted_text = 0u32;
        let mut darkest_clear = 255u8;
        let mut worst_fringe = 0u8;
        let mut strong_fringe = 0u32;
        for (png_pixel, jpeg_pixel) in png_image.pixels().zip(jpeg_image.pixels()) {
            let [r, g, b, a] = png_pixel.0;
            if a == 0 {
                transparent += 1;
                let floor = jpeg_pixel[0].min(jpeg_pixel[1]).min(jpeg_pixel[2]);
                darkest_clear = darkest_clear.min(floor);
                let fringe = channel_spread(jpeg_pixel.0);
                worst_fringe = worst_fringe.max(fringe);
                if fringe > 36 {
                    strong_fringe += 1;
                }
            } else if a < 255 {
                edge += 1;
                let expected = composite_onto(ColorU8::from_rgba(r, g, b, a), [255, 255, 255]);
                let error = max3(
                    expected.0.abs_diff(jpeg_pixel[0]),
                    expected.1.abs_diff(jpeg_pixel[1]),
                    expected.2.abs_diff(jpeg_pixel[2]),
                );
                max_edge_error = max_edge_error.max(error);
            }
            if a > 200 && r > 140 && r > g.saturating_add(40) && r > b.saturating_add(40) {
                painted_text += 1;
                assert!(jpeg_pixel[0] > jpeg_pixel[1] && jpeg_pixel[0] > jpeg_pixel[2]);
            }
        }

        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/verify");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("alpha.png"), &png).unwrap();
        std::fs::write(dir.join("alpha.jpg"), &jpeg).unwrap();

        assert!(transparent > 100, "transparent pixels {transparent}");
        assert!(edge > 10, "edge pixels {edge}");
        assert!(
            darkest_clear >= 210,
            "transparent region went dark: {darkest_clear}"
        );
        assert!(
            max_edge_error <= 40,
            "edge drifted from un-premultiplied composite by {max_edge_error}"
        );
        assert!(painted_text > 20, "thin red text was not painted");
        assert_eq!(
            strong_fringe, 0,
            "transparent pixels picked up chroma, worst {worst_fringe}"
        );
    }

    #[test]
    fn font_load_waits_while_renders_share_options() {
        let options = Arc::new(RenderOptions::new(String::new()));
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="12"><rect width="20" height="12" fill="#abc"/></svg>"##;
        let start = Arc::new(Barrier::new(4));
        let mut handles = Vec::new();
        for _ in 0..3 {
            let options = Arc::clone(&options);
            let start = Arc::clone(&start);
            handles.push(thread::spawn(move || {
                start.wait();
                for _ in 0..8 {
                    options
                        .with_options(|opt| render(svg, opt, 1.0, RenderType::Png))
                        .unwrap();
                    thread::sleep(Duration::from_millis(2));
                }
            }));
        }
        {
            let options = Arc::clone(&options);
            let start = Arc::clone(&start);
            handles.push(thread::spawn(move || {
                start.wait();
                let missing = std::env::temp_dir().join("resvg-jni-no-such-fonts");
                options.load_fonts_dir(missing.to_str().unwrap());
                options.try_load_font(missing.to_str().unwrap());
            }));
        }
        for handle in handles {
            handle.join().unwrap();
        }
    }

    fn sample_svg() -> &'static str {
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="320" height="120" viewBox="0 0 320 120">
  <rect x="16" y="16" width="88" height="72" rx="4" fill="#2266ff" fill-opacity="0.45"/>
  <rect x="48" y="36" width="36" height="28" fill="#ff2040" fill-opacity="0.55"/>
  <text x="128" y="42" font-family="Arial" font-size="10" fill="#d00020">lilli 1.25</text>
  <text x="128" y="62" font-family="Arial" font-size="8" fill="#1030c0">score +12.5%</text>
  <text x="128" y="92" font-family="Microsoft YaHei, Arial" font-size="11" fill="#c01040">细字 98.6</text>
</svg>"##
    }

    fn load_text_fonts(options: &RenderOptions) {
        for path in [
            r"C:\Windows\Fonts\arial.ttf",
            r"C:\Windows\Fonts\msyh.ttc",
        ] {
            if std::path::Path::new(path).exists() {
                options.try_load_font(path);
            }
        }
    }

    fn channel_spread(pixel: [u8; 3]) -> u8 {
        let max = pixel[0].max(pixel[1]).max(pixel[2]);
        let min = pixel[0].min(pixel[1]).min(pixel[2]);
        max - min
    }

    fn max3(a: u8, b: u8, c: u8) -> u8 {
        a.max(b).max(c)
    }

    fn jpeg_sampling_factors(data: &[u8]) -> Vec<u8> {
        let payload = jpeg_segment(data, 0xC0);
        let count = payload[5] as usize;
        let mut factors = Vec::with_capacity(count);
        for index in 0..count {
            factors.push(payload[7 + index * 3]);
        }
        factors
    }

    fn jpeg_luma_quant_first(data: &[u8]) -> u8 {
        let payload = jpeg_segment(data, 0xDB);
        assert_eq!(payload[0] & 0x0f, 0, "first quant table is luma");
        payload[1]
    }

    fn jpeg_segment(data: &[u8], marker: u8) -> &[u8] {
        assert_eq!(&data[..2], &[0xFF, 0xD8]);
        let mut index = 2;
        while index + 4 < data.len() {
            while data[index] == 0xFF {
                index += 1;
            }
            let found = data[index];
            index += 1;
            if found == 0xD9 {
                break;
            }
            let length = u16::from_be_bytes([data[index], data[index + 1]]) as usize;
            let start = index + 2;
            let end = index + length;
            if found == marker {
                return &data[start..end];
            }
            index = end;
        }
        panic!("jpeg marker {marker:#x} not found");
    }
}
