use eframe::egui::ColorImage;
use std::path::Path;

#[cfg(target_os = "macos")]
pub(super) fn load(path: &Path) -> Option<ColorImage> {
    use objc2::{AnyThread, MainThreadMarker, rc::autoreleasepool};
    use objc2_app_kit::{NSBitmapImageRep, NSGraphicsContext, NSWorkspace};
    use objc2_foundation::NSString;

    MainThreadMarker::new()?;
    autoreleasepool(|_| {
        let image = NSWorkspace::sharedWorkspace().iconForFile(&NSString::from_str(path.to_str()?));
        // SAFETY: null planes asks AppKit to allocate and own an interleaved
        // 64 × 64, four-channel bitmap. No caller-owned memory is passed in.
        let bitmap = unsafe {
            NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
                NSBitmapImageRep::alloc(),
                std::ptr::null_mut(),
                64,
                64,
                8,
                4,
                true,
                false,
                objc2_app_kit::NSDeviceRGBColorSpace,
                256,
                32,
            )?
        };
        let pixels = bitmap.bitmapData();
        if pixels.is_null() || bitmap.bytesPerRow() != 256 {
            return None;
        }
        // SAFETY: this bitmap owns the interleaved 64 × 256 byte buffer. Clear
        // transparent pixels before AppKit draws; no uninitialized bytes escape.
        unsafe { std::ptr::write_bytes(pixels, 0, 64 * 256) };
        let graphics = NSGraphicsContext::graphicsContextWithBitmapImageRep(&bitmap)?;
        draw(&image, &graphics);
        // SAFETY: the retained bitmap owns 64 rows of 256 bytes, verified above.
        // ColorImage copies these bytes before the bitmap is released.
        let pixels = unsafe { std::slice::from_raw_parts(pixels, 64 * 256) };
        Some(ColorImage::from_rgba_premultiplied([64, 64], pixels))
    })
}

#[cfg(target_os = "macos")]
fn draw(image: &objc2_app_kit::NSImage, graphics: &objc2_app_kit::NSGraphicsContext) {
    use objc2_app_kit::{NSCompositingOperation, NSGraphicsContext};
    use objc2_foundation::{NSPoint, NSRect, NSSize};

    NSGraphicsContext::saveGraphicsState_class();
    NSGraphicsContext::setCurrentContext(Some(graphics));
    image.drawInRect_fromRect_operation_fraction(
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(64.0, 64.0)),
        NSRect::ZERO,
        NSCompositingOperation::Copy,
        1.0,
    );
    NSGraphicsContext::restoreGraphicsState_class();
}

#[cfg(not(target_os = "macos"))]
pub(super) fn load(_: &Path) -> Option<ColorImage> {
    None
}
