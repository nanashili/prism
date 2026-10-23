use eframe::egui::Color32;

#[cfg(target_os = "macos")]
mod macos;

pub(super) struct WindowEffects {
    #[cfg(target_os = "macos")]
    vibrant: bool,
}

impl WindowEffects {
    pub fn new(context: &eframe::CreationContext<'_>) -> Self {
        #[cfg(target_os = "macos")]
        {
            let vibrant = match macos::apply(context) {
                Ok(()) => true,
                Err(error) => {
                    eprintln!("Prism: couldn't apply the window material: {error}");
                    false
                }
            };
            Self { vibrant }
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = context;
            Self {}
        }
    }

    fn is_translucent(&self) -> bool {
        #[cfg(target_os = "macos")]
        {
            self.vibrant
        }
        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    }

    pub fn sidebar(&self, window: Color32, fallback: Color32, transparency: f32) -> Color32 {
        if self.is_translucent() {
            let alpha = ((1.0 - 0.7 * transparency / 100.0) * 255.0).round() as u8;
            Color32::from_rgba_unmultiplied(window.r(), window.g(), window.b(), alpha)
        } else {
            fallback
        }
    }
}
