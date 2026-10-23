use super::geometry::Layout;
use crate::error::{Result, failure};
use liquid_glass_rs::GlassViewManager;
use objc2::{MainThreadMarker, rc::Retained};
use objc2_app_kit::{NSView, NSWindowOrderingMode};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use objc2_quartz_core::CATransaction;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

mod contours;
mod surface;

pub(super) struct PrismGlass {
    surface: surface::Surface,
    contours: contours::Contours,
    content: Retained<NSView>,
    container: Retained<NSView>,
}

impl PrismGlass {
    pub fn new(context: &eframe::CreationContext<'_>) -> Result<Option<Self>> {
        let main_thread = MainThreadMarker::new().ok_or("The glass prism needs the main thread")?;
        let manager = GlassViewManager::new();
        if !manager.is_supported() {
            return Ok(None);
        }
        let window = context.window_handle()?;
        let RawWindowHandle::AppKit(handle) = window.as_raw() else {
            return Err(failure("The window has no AppKit content view"));
        };
        // SAFETY: eframe lends a live NSView on the main thread; retain it for the intro.
        let content = unsafe { Retained::retain(handle.ns_view.cast::<NSView>().as_ptr()) }
            .ok_or("The window has no content view")?;
        Self::install(manager, content, main_thread).map(Some)
    }

    fn install(
        manager: GlassViewManager,
        content: Retained<NSView>,
        main_thread: MainThreadMarker,
    ) -> Result<Self> {
        // SAFETY: the retained content belongs to the live eframe window.
        let container =
            unsafe { content.superview() }.ok_or("The content view has no container")?;
        let surface = surface::Surface::new(manager, main_thread)?;
        let contours = contours::Contours::new(&surface, main_thread)?;

        // The prism samples the animated rays painted by Glow beneath it.
        container.addSubview_positioned_relativeTo(
            &surface.host,
            NSWindowOrderingMode::Above,
            Some(&content),
        );
        Ok(Self {
            surface,
            contours,
            content,
            container,
        })
    }

    pub fn update(&mut self, layout: &Layout, opacity: f32, pixels_per_point: f32) {
        if layout.viewport.width() <= 0.0 || layout.viewport.height() <= 0.0 {
            self.surface.host.setAlphaValue(0.0);
            return;
        }
        let frame = self.frame(layout);
        let content_scale =
            f64::from(pixels_per_point * layout.prism_bounds().width()) / frame.size.width.max(1.0);

        CATransaction::begin();
        CATransaction::setDisableActions(true);
        self.surface.host.setFrame(frame);
        self.surface.view.setFrame(self.surface.host.bounds());
        self.contours
            .update(self.surface.host.bounds(), content_scale);
        self.surface.host.setAlphaValue(f64::from(opacity));
        CATransaction::commit();
    }

    fn frame(&self, layout: &Layout) -> NSRect {
        let source = self.content.bounds();
        let bounds = layout.prism_bounds();
        let scale_x = source.size.width / f64::from(layout.viewport.width());
        let scale_y = source.size.height / f64::from(layout.viewport.height());
        let x = source.origin.x + f64::from(bounds.left() - layout.viewport.left()) * scale_x;
        let y = if self.content.isFlipped() {
            source.origin.y + f64::from(bounds.top() - layout.viewport.top()) * scale_y
        } else {
            source.origin.y + source.size.height
                - f64::from(bounds.bottom() - layout.viewport.top()) * scale_y
        };
        let rect = NSRect::new(
            NSPoint::new(x, y),
            NSSize::new(
                f64::from(bounds.width()) * scale_x,
                f64::from(bounds.height()) * scale_y,
            ),
        );
        self.content.convertRect_toView(rect, Some(&self.container))
    }
}
