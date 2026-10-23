use crate::error::{Result, failure};
use objc2::MainThreadMarker;
use objc2_app_kit::NSView;
use raw_window_handle::{AppKitWindowHandle, HasWindowHandle, RawWindowHandle, WindowHandle};
use std::ptr::NonNull;
use window_vibrancy::{NSVisualEffectMaterial, NSVisualEffectState, apply_vibrancy};

pub(super) fn apply(context: &eframe::CreationContext<'_>) -> Result<()> {
    let _main_thread = MainThreadMarker::new().ok_or("Window effects need the main thread")?;
    let window = context.window_handle()?;
    let RawWindowHandle::AppKit(handle) = window.as_raw() else {
        return Err(failure("The window has no AppKit content view"));
    };

    // SAFETY: the borrowed eframe handle keeps this NSView alive on the main thread.
    let content = unsafe { handle.ns_view.cast::<NSView>().as_ref() };
    let container =
        unsafe { content.superview() }.ok_or("The window content view has no container")?;
    let handle = AppKitWindowHandle::new(NonNull::from(&*container).cast());
    // SAFETY: the retained container remains alive throughout this synchronous call.
    let background = unsafe { WindowHandle::borrow_raw(RawWindowHandle::AppKit(handle)) };
    apply_vibrancy(
        background,
        NSVisualEffectMaterial::UnderWindowBackground,
        Some(NSVisualEffectState::Active),
        None,
    )?;
    Ok(())
}
