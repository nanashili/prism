use super::{
    PageView,
    rows::{Anchor, RowSpec},
};
use crate::ui::presentation::{ActivityState, ChoiceAccess, ThoughtDisplay};
use crate::ui::{
    activity_view,
    icons::Icon,
    message::{BodyCache, layout::LayoutKey, render_message_cached, style::ChatStyle},
    state::Chat,
};
use eframe::egui;

const MARGIN: f32 = 10.0;
const OVERSCAN: f32 = 128.0;

struct Position {
    anchor: Option<Anchor>,
    follow: bool,
    fallback: f32,
    viewport_height: f32,
}

pub(super) struct RenderFrame<'a> {
    pub chat: &'a Chat,
    pub thinking: bool,
    pub style: &'a ChatStyle,
    pub cache: &'a mut BodyCache,
    pub specs: &'a [RowSpec],
}

impl PageView {
    #[cfg_attr(feature = "profiling", hotpath::measure)]
    pub(super) fn show(&mut self, ui: &mut egui::Ui, mut frame: RenderFrame<'_>) {
        let historical = frame.chat.history.page.is_some();
        let output = egui::ScrollArea::vertical()
            .id_salt(("conversation", self.id))
            .stick_to_bottom(!historical)
            .animated(false)
            .auto_shrink([false, false])
            .show_viewport(ui, |ui, viewport| {
                let position = Position {
                    anchor: self.rows.anchor(viewport.top() - MARGIN),
                    follow: !historical
                        && !scrolling_away(ui)
                        && (self.rows.total() > 0.0 || viewport.top() <= 1.0)
                        && viewport.top() >= self.max_offset(viewport.height()) - 1.0,
                    fallback: viewport.top(),
                    viewport_height: viewport.height(),
                };
                self.rows.sync(
                    frame.specs,
                    LayoutKey::new(ui, frame.style, ThoughtDisplay::from(frame.thinking)),
                );
                let changed = self.render_rows(ui, viewport, &position, &mut frame);
                let target = self.offset(&position);
                ui.set_min_size(egui::vec2(
                    ui.available_width(),
                    self.rows.total() + 2.0 * MARGIN,
                ));
                let correction = target - viewport.top();
                if !position.follow && correction.abs() > 0.5 {
                    // Apply before native input and clamping so a shrinking history
                    // does not clamp the old offset and then apply compensation twice.
                    ui.scroll_with_delta(egui::vec2(0.0, -correction));
                }
                (correction, changed)
            });
        settle_geometry(ui.ctx(), output.inner);
    }

    fn render_rows(
        &mut self,
        ui: &mut egui::Ui,
        viewport: egui::Rect,
        position: &Position,
        frame: &mut RenderFrame<'_>,
    ) -> bool {
        let mut index = self
            .rows
            .first_intersecting(self.offset(position) - MARGIN - OVERSCAN);
        let mut changed = false;
        while index < frame.specs.len() {
            let offset = self.offset(position);
            let start = self.rows.start(index) + MARGIN;
            if start > offset + viewport.height() + OVERSCAN {
                break;
            }
            let top = ui.max_rect().top() + start + viewport.top() - offset;
            let rect = egui::Rect::from_min_size(
                egui::pos2(ui.max_rect().left() + 16.0, top),
                egui::vec2((ui.available_width() - 32.0).max(1.0), 0.0),
            );
            let height = frame.render_row(ui, rect, index);
            changed |= self.rows.measure(index, height);
            index += 1;
        }
        changed
    }

    fn offset(&self, position: &Position) -> f32 {
        let max = self.max_offset(position.viewport_height);
        if position.follow {
            return max;
        }
        let offset = match position.anchor {
            Some(anchor) => self
                .rows
                .position(anchor)
                .map_or(0.0, |offset| offset + MARGIN),
            None => position.fallback,
        };
        offset.clamp(0.0, max)
    }

    fn max_offset(&self, viewport_height: f32) -> f32 {
        (self.rows.total() + 2.0 * MARGIN - viewport_height).max(0.0)
    }
}

impl RenderFrame<'_> {
    fn render_row(&mut self, ui: &mut egui::Ui, rect: egui::Rect, index: usize) -> f32 {
        let messages = self
            .chat
            .history
            .page
            .as_ref()
            .map_or(self.chat.messages.as_slice(), |page| &page.messages);
        let mut row_ui = ui.new_child(
            egui::UiBuilder::new()
                .id(self.specs[index].id)
                .max_rect(rect),
        );
        row_ui.spacing_mut().item_spacing.y = 0.0;

        if let Some(message) = messages.get(index) {
            let last = index + 1 == messages.len();
            render_message_cached(
                &mut row_ui,
                message,
                ThoughtDisplay::from(self.thinking),
                ActivityState::from(self.chat.busy && self.chat.history.page.is_none() && last),
                ChoiceAccess::from(self.chat.questions.is_reply() && last),
                self.style,
                self.cache,
            );
        } else {
            show_status(&mut row_ui, self.chat, self.style);
        }
        row_ui.min_rect().height()
    }
}

fn scrolling_away(ui: &egui::Ui) -> bool {
    ui.rect_contains_pointer(ui.clip_rect()) && ui.input(|input| input.smooth_scroll_delta.y > 0.0)
}

fn settle_geometry(context: &egui::Context, (correction, changed): (f32, bool)) {
    if changed || correction.abs() > 0.5 {
        if context.current_pass_index() == 0 {
            context.request_discard("transcript row measurements changed");
        }
        context.request_repaint();
    }
}

fn show_status(ui: &mut egui::Ui, chat: &Chat, style: &ChatStyle) {
    ui.horizontal(|ui| {
        Icon::Spark.show(ui, 14.0, style.palette.accent);
        activity_view::shimmer::scope(ui, ActivityState::Active, |ui| {
            ui.label(style.text(&chat.status).weak());
        });
    });
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_secs(1));
}
