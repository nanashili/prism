use super::surface::Surface;
use crate::error::Result;
use crate::ui::splash::geometry::{FACET, SILHOUETTE};
use objc2::{MainThreadMarker, MainThreadOnly, rc::Retained};
use objc2_app_kit::{NSView, NSWindowOrderingMode};
use objc2_core_foundation::CFRetained;
use objc2_core_graphics::{CGColor, CGMutablePath};
use objc2_foundation::NSRect;
use objc2_quartz_core::CAShapeLayer;

pub(super) struct Contours {
    overlay: Retained<NSView>,
    mask: Retained<CAShapeLayer>,
    rim: Retained<CAShapeLayer>,
    facet: Retained<CAShapeLayer>,
}

impl Contours {
    pub fn new(surface: &Surface, main_thread: MainThreadMarker) -> Result<Self> {
        surface.view.setWantsLayer(true);
        let glass_layer = surface.view.layer().ok_or("The glass view has no layer")?;
        let mask = CAShapeLayer::new();
        mask.setFillColor(Some(&CGColor::new_srgb(1.0, 1.0, 1.0, 1.0)));
        // SAFETY: this new mask belongs exclusively to the glass layer, with no cycles.
        unsafe { glass_layer.setMask(Some(&mask)) };

        let overlay = NSView::initWithFrame(NSView::alloc(main_thread), NSRect::ZERO);
        overlay.setWantsLayer(true);
        let layer = overlay.layer().ok_or("The prism outline has no layer")?;
        let rim = outline(0.0, 0.55);
        let facet = outline(0.035, 0.24);
        layer.addSublayer(&facet);
        layer.addSublayer(&rim);
        surface.host.addSubview_positioned_relativeTo(
            &overlay,
            NSWindowOrderingMode::Above,
            Some(&surface.view),
        );

        Ok(Self {
            overlay,
            mask,
            rim,
            facet,
        })
    }

    pub fn update(&self, bounds: NSRect, content_scale: f64) {
        self.overlay.setFrame(bounds);
        let silhouette = path(&SILHOUETTE, bounds);
        let facet = path(&FACET, bounds);
        self.mask.setPath(Some(&silhouette));
        self.rim.setPath(Some(&silhouette));
        self.facet.setPath(Some(&facet));

        for layer in [&self.mask, &self.rim, &self.facet] {
            layer.setFrame(bounds);
            layer.setContentsScale(content_scale);
            layer.setLineWidth(bounds.size.width / 330.0);
        }
    }
}

fn outline(fill_alpha: f64, stroke_alpha: f64) -> Retained<CAShapeLayer> {
    let layer = CAShapeLayer::new();
    layer.setFillColor(Some(&CGColor::new_srgb(0.75, 0.85, 1.0, fill_alpha)));
    layer.setStrokeColor(Some(&CGColor::new_srgb(0.8, 0.9, 1.0, stroke_alpha)));
    layer
}

fn path(points: &[(f32, f32)], bounds: NSRect) -> CFRetained<CGMutablePath> {
    let path = CGMutablePath::new();
    for (index, &(x, y)) in points.iter().enumerate() {
        let x = f64::from(x + 165.0) * bounds.size.width / 330.0;
        let y = f64::from(150.0 - y) * bounds.size.height / 330.0;
        // SAFETY: a null transform uses these local coordinates unchanged.
        unsafe {
            if index == 0 {
                CGMutablePath::move_to_point(Some(&path), std::ptr::null(), x, y);
            } else {
                CGMutablePath::add_line_to_point(Some(&path), std::ptr::null(), x, y);
            }
        }
    }
    CGMutablePath::close_subpath(Some(&path));
    path
}
