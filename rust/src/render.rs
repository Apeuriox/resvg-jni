use std::io::Write;

use resvg::{tiny_skia::{self, Pixmap, PixmapRef, PremultipliedColorU8}, usvg::{self}};
use image::{self, Rgb, Rgba};



fn load_xml(text: &str) -> Result<roxmltree::Document, usvg::Error> {
    let xml_opt = roxmltree::ParsingOptions {
        allow_dtd: true,
        ..Default::default()
    };

    let doc =
        roxmltree::Document::parse_with_options(text, xml_opt).map_err(usvg::Error::ParsingFailed)?;

    Ok(doc)
}

pub fn rgba8_to_rgb8(input: image::ImageBuffer<Rgba<u8>, Vec<u8>>) -> image::ImageBuffer<Rgb<u8>, Vec<u8>> {
    let width = input.width() as usize;
    let height = input.height() as usize;
    
    // Get the raw image data as a vector
    let input: &Vec<u8> = input.as_raw();
    
    // Allocate a new buffer for the RGB image, 3 bytes per pixel
    let mut output_data = vec![0u8; width * height * 3];
    
    // Iterate through 4-byte chunks of the image data (RGBA bytes)
    for (output, chunk) in {
        output_data.chunks_exact_mut(3).zip(input.chunks_exact(4))
    } {
        // ... and copy each of them to output, leaving out the A byte
        output.copy_from_slice(&chunk[0..3]);
    }
    
    // Construct a new image
    image::ImageBuffer::from_raw(width as u32, height as u32, output_data).unwrap()
}

pub fn encode_jpg(map: Pixmap) -> anyhow::Result<Vec<u8>> {
    // Skia uses skcms here, which is somewhat similar to RasterPipeline.
    let mut writer = std::io::Cursor::new(Vec::new());
    {
        let im = image::RgbaImage::from_raw(map.width(), map.height(), map.take()).unwrap();

        #[cfg(target_os = "linux")]
        {
            use turbojpeg;
            let jpeg_data = turbojpeg::compress_image(&im, 95, turbojpeg::Subsamp::Sub2x2)?;
            writer.write_all(jpeg_data.as_ref())?;
        }

        #[cfg(not(target_os = "linux"))]
        {
            let rgbim = rgba8_to_rgb8(im);
            rgbim.write_to(&mut writer, image::ImageFormat::Jpeg)?;
            
        }
    }

    Ok(writer.into_inner())
}

pub enum RenderType {
    Png,
    Jpeg
}

pub fn render(xmldata: &str, opt: &usvg::Options, scale: f32, render_type: RenderType) -> anyhow::Result<Vec<u8>> {
    let doc = load_xml(xmldata)?;
    let svg = usvg::Tree::from_xmltree(&doc, opt)?;
    let size = svg.size().to_int_size().scale_by(scale).unwrap();
    let mut canvas = tiny_skia::Pixmap::new(size.width(), size.height()).unwrap();
    resvg::render(&svg, tiny_skia::Transform::from_scale(scale, scale), &mut canvas.as_mut());
    
    Ok(match render_type {
        RenderType::Png => canvas.encode_png()?,
        RenderType::Jpeg => encode_jpg(canvas)?,
    })
}