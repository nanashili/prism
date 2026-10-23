use super::app::Prism;
use eframe::egui::{self, Pos2, Rect};

const POSITION_ID: &str = "native-attachment-drop-position";

pub(super) fn update_position(context: &egui::Context, frame: &eframe::Frame) {
    let hovering = context.input(|input| !input.raw.hovered_files.is_empty());
    let dropping = context.input(|input| !input.raw.dropped_files.is_empty());
    if !hovering && !dropping {
        return;
    }
    let position = native_position(frame)
        .map(|position| position / context.zoom_factor())
        .or_else(|| context.input(|input| input.pointer.latest_pos()));
    context.data_mut(|data| data.insert_temp(egui::Id::new(POSITION_ID), position));
    if hovering {
        context.request_repaint_after(std::time::Duration::from_millis(50));
    }
}

pub(super) fn hovered(context: &egui::Context, rect: Rect) -> bool {
    context.input(|input| !input.raw.hovered_files.is_empty()) && contains_pointer(context, rect)
}

fn contains_pointer(context: &egui::Context, rect: Rect) -> bool {
    context
        .data(|data| data.get_temp::<Option<Pos2>>(egui::Id::new(POSITION_ID)))
        .flatten()
        .is_some_and(|position| rect.contains(position))
}

impl Prism {
    pub(super) fn composer_drop(&mut self, context: &egui::Context, rect: Rect) {
        if !contains_pointer(context, rect) {
            return;
        }
        let dropped = context.input_mut(|input| std::mem::take(&mut input.raw.dropped_files));
        if dropped.is_empty() {
            return;
        }
        let mut paths = Vec::new();
        let mut missing_path = false;
        for file in dropped {
            if let Some(path) = file.path {
                paths.push(path);
            } else {
                missing_path = true;
            }
        }
        self.attach_paths(paths);
        if missing_path {
            self.report_error(crate::error::failure(
                "This dragged item has no file location. Save it first, then use the plus button to attach the file.",
            ));
        }
        context.request_repaint();
    }
}

#[cfg(target_os = "macos")]
fn native_position(frame: &eframe::Frame) -> Option<Pos2> {
    use objc2_app_kit::NSView;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let handle = frame.window_handle().ok()?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return None;
    };
    // SAFETY: eframe lends a live AppKit view on the main UI thread. No native
    // references escape this function or outlive the borrowed window handle.
    let view = unsafe { handle.ns_view.cast::<NSView>().as_ref() };
    let window = view.window()?;
    let point = window.mouseLocationOutsideOfEventStream();
    // Winit's content view uses a flipped, top-left coordinate system.
    let point = view.convertPoint_fromView(point, None);
    Some(egui::pos2(point.x as f32, point.y as f32))
}

#[cfg(not(target_os = "macos"))]
fn native_position(_: &eframe::Frame) -> Option<Pos2> {
    None
}
