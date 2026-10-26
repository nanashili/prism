use crate::error::{Result, failure};
use liquid_glass_rs::{GlassOptions, GlassViewManager};
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained};
use objc2_app_kit::{NSGlassEffectView, NSGlassEffectViewStyle, NSView};
use objc2_foundation::{NSPoint, NSRect};
use std::ptr::NonNull;

define_class!(
    // SAFETY: NSView has no subclassing requirements. This class holds no ivars.
    #[unsafe(super = NSView)]
    #[thread_kind = MainThreadOnly]
    struct PrismGlassHost;

    impl PrismGlassHost {
        // SAFETY: this matches NSView's hitTest: signature and returns no view.
        #[unsafe(method(hitTest:))]
        fn hit_test(&self, _point: NSPoint) -> *mut NSView {
            std::ptr::null_mut()
        }
    }
);

pub(super) struct Surface {
    manager: GlassViewManager,
    id: i32,
    pub host: Retained<NSView>,
    pub view: Retained<NSGlassEffectView>,
}

impl Surface {
    pub fn new(manager: GlassViewManager, main_thread: MainThreadMarker) -> Result<Self> {
        // SAFETY: PrismGlassHost inherits NSView's initializer without extra ivars.
        let host: Retained<PrismGlassHost> =
            unsafe { msg_send![PrismGlassHost::alloc(main_thread), initWithFrame: NSRect::ZERO] };
        let host: Retained<NSView> = host.into_super();
        host.setAlphaValue(0.0);

        // An empty host identifies the library's sole inserted child unambiguously.
        let id = manager.add_glass_view(
            NonNull::from(&*host).cast().as_ptr(),
            GlassOptions::default(),
        )?;
        let view = Self::take_view(&manager, id, &host)?;
        view.setStyle(NSGlassEffectViewStyle::Clear);
        view.setCornerRadius(0.0);

        Ok(Self {
            manager,
            id,
            host,
            view,
        })
    }

    fn take_view(
        manager: &GlassViewManager,
        id: i32,
        host: &NSView,
    ) -> Result<Retained<NSGlassEffectView>> {
        let Some(view) = host.subviews().firstObject() else {
            return Err(remove_after_failure(
                manager,
                id,
                "Liquid Glass did not create a view",
            ));
        };

        // liquid-glass-rs 0.1.1 stores the +1 alloc/init reference as a raw pointer
        // and never releases it.
        // SAFETY: adopt that owned +1 reference exactly once; the NSArray retain is separate.
        let allocation = unsafe { Retained::from_raw(Retained::as_ptr(&view).cast_mut()) }
            .expect("A retained NSView is non-null");
        match allocation.downcast::<NSGlassEffectView>() {
            Ok(view) => Ok(view),
            Err(allocation) => {
                let error = remove_after_failure(manager, id, "Expected a native glass view");
                drop(allocation);
                Err(error)
            }
        }
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        if let Err(error) = self.manager.remove_view(self.id) {
            eprintln!("Prism: couldn't remove the glass prism: {error}");
        }
        self.host.removeFromSuperview();
    }
}

fn remove_after_failure(
    manager: &GlassViewManager,
    id: i32,
    message: &str,
) -> Box<dyn std::error::Error + Send + Sync> {
    let mut message = message.to_owned();
    if let Err(cleanup) = manager.remove_view(id) {
        message.push_str(&format!("; couldn't remove the effect: {cleanup}"));
    }
    failure(message)
}
