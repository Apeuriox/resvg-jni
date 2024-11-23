use resvg::{tiny_skia, usvg::{self}};

fn load_xml(text: &str) -> Result<roxmltree::Document, usvg::Error> {
    let xml_opt = roxmltree::ParsingOptions {
        allow_dtd: true,
        ..Default::default()
    };

    let doc =
        roxmltree::Document::parse_with_options(text, xml_opt).map_err(usvg::Error::ParsingFailed)?;

    Ok(doc)
}

pub fn render(xmldata: &str, opt: &usvg::Options, scale: f32) -> anyhow::Result<Vec<u8>> {
    let doc = load_xml(xmldata)?;
    let svg = usvg::Tree::from_xmltree(&doc, opt)?;
    let size = svg.size().to_int_size().scale_by(scale).unwrap();
    let mut canvas = tiny_skia::Pixmap::new(size.width(), size.height()).unwrap();
    resvg::render(&svg, tiny_skia::Transform::from_scale(scale, scale), &mut canvas.as_mut());
    Ok(canvas.encode_png()?)
}