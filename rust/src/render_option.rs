use std::{path::PathBuf, sync::Arc};

use resvg::usvg::{self, fontdb, ImageRendering, ShapeRendering, TextRendering};

#[derive(Default, Clone)]
pub struct RenderOptions {
    pub resources_dir: String,
    pub fontdb: fontdb::Database,
    pub shape_rendering: ShapeRendering,
    pub text_rendering: TextRendering,
    pub image_rendering: ImageRendering
}

impl RenderOptions {
    pub fn new(resources_dir: String) -> Self {
        Self {
            resources_dir,
            fontdb: fontdb::Database::new(),
            ..Default::default()
        }
    }

    pub fn load_system_fonts(&mut self) {
        self.fontdb.load_system_fonts();
    }

    pub fn try_load_font(&mut self, path: &str) {
        let _ = self.fontdb.load_font_file(path);
    }

    pub fn load_fonts_dir(&mut self, path: &str) {
        self.fontdb.load_fonts_dir(path);
    }

    pub fn get_options(&self) -> usvg::Options {
        usvg::Options {
            resources_dir: Some(PathBuf::from(&self.resources_dir)),
            fontdb: Arc::new(self.fontdb.clone()),
            shape_rendering: self.shape_rendering,
            text_rendering: self.text_rendering,
            image_rendering: self.image_rendering,
            ..Default::default()
        }
    }
}
