use super::geometry::Layout;
use crate::error::Result;
use eframe::egui;

pub(super) enum Material {
    #[cfg(target_os = "macos")]
    Native(super::native::PrismGlass),
    Image(egui::TextureHandle),
}

impl Material {
    pub fn new(context: &eframe::CreationContext<'_>) -> Result<Self> {
        #[cfg(target_os = "macos")]
        match super::native::PrismGlass::new(context) {
            Ok(Some(glass)) => return Ok(Self::Native(glass)),
            Ok(None) => {}
            Err(error) => eprintln!("Prism: couldn't create the glass prism: {error}"),
        }

        Self::load_image(&context.egui_ctx)
    }

    fn load_image(context: &egui::Context) -> Result<Self> {
        let pixels =
            image::load_from_memory(include_bytes!("../../../resources/splash/glass-prism.png"))?
                .into_rgba8();
        let size = [pixels.width() as usize, pixels.height() as usize];
        let image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_raw());
        let texture = context.load_texture("splash-glass", image, egui::TextureOptions::LINEAR);
        Ok(Self::Image(texture))
    }

    pub fn texture(&self) -> Option<egui::TextureId> {
        match self {
            #[cfg(target_os = "macos")]
            Self::Native(_) => None,
            Self::Image(texture) => Some(texture.id()),
        }
    }

    pub fn update(&mut self, layout: &Layout, opacity: f32, pixels_per_point: f32) {
        #[cfg(target_os = "macos")]
        if let Self::Native(glass) = self {
            glass.update(layout, opacity, pixels_per_point);
        }
        #[cfg(not(target_os = "macos"))]
        let _ = (layout, opacity, pixels_per_point);
    }
}
