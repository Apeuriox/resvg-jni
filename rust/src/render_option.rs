use std::path::PathBuf;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use resvg::usvg::{self, ImageRendering, ShapeRendering, TextRendering};

/// One process-wide render setup.
///
/// Font loads and rendering-mode changes take the write lock, so a load that
/// overlaps a render waits until that render drops its read lock. Rendering
/// only borrows the prepared [`usvg::Options`]: the font database stays an
/// `Arc` and is not cloned, and fields that do not change are left in place.
pub struct RenderOptions {
    inner: RwLock<Prepared>,
}

struct Prepared {
    options: usvg::Options<'static>,
}

impl RenderOptions {
    pub fn new(resources_dir: String) -> Self {
        let mut options = usvg::Options::default();
        options.resources_dir = Some(PathBuf::from(resources_dir));
        Self {
            inner: RwLock::new(Prepared { options }),
        }
    }

    pub fn load_system_fonts(&self) {
        self.write().options.fontdb_mut().load_system_fonts();
    }

    pub fn try_load_font(&self, path: &str) {
        let _ = self.write().options.fontdb_mut().load_font_file(path);
    }

    pub fn load_fonts_dir(&self, path: &str) {
        self.write().options.fontdb_mut().load_fonts_dir(path);
    }

    pub fn set_shape_rendering(&self, mode: ShapeRendering) {
        self.write().options.shape_rendering = mode;
    }

    pub fn set_text_rendering(&self, mode: TextRendering) {
        self.write().options.text_rendering = mode;
    }

    pub fn set_image_rendering(&self, mode: ImageRendering) {
        self.write().options.image_rendering = mode;
    }

    /// Borrows the prepared options for the whole call.
    ///
    /// The read guard stays alive until `f` returns, which is the whole render.
    pub fn with_options<R>(&self, f: impl FnOnce(&usvg::Options) -> R) -> R {
        let prepared = self.read();
        f(&prepared.options)
    }

    fn read(&self) -> RwLockReadGuard<'_, Prepared> {
        self.inner.read().unwrap_or_else(|err| err.into_inner())
    }

    fn write(&self) -> RwLockWriteGuard<'_, Prepared> {
        self.inner.write().unwrap_or_else(|err| err.into_inner())
    }
}
