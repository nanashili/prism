use crate::error::Result;
use crate::model::ConnectionState;
use crate::model::GoalStatus;
use crate::ui::{app::Prism, icons::Icon, theme::Palette};
use eframe::egui::{self, FontId, Rect, Sense, Stroke};

impl Prism {
    pub(in crate::ui) fn send_goal_control(&mut self) -> bool {
        if crate::agent::picks::key(&self.settings) != "codex-acp" {
            return false;
        }
        let words: Vec<_> = self.chat.draft.split_whitespace().take(3).collect();
        if words.len() != 2
            || !words[0].eq_ignore_ascii_case("/goal")
            || !self.chat.attachments.is_empty()
        {
            return false;
        }
        let action = if words[1].eq_ignore_ascii_case("pause") {
            "pause"
        } else if words[1].eq_ignore_ascii_case("clear") {
            "clear"
        } else {
            return false;
        };
        match self.control_goal(action) {
            Ok(()) => {
                self.chat.draft.clear();
                if let Err(error) = self.save_chat_draft() {
                    self.report_error(error);
                }
            }
            Err(error) => self.report_error(error),
        }
        true
    }

    fn control_goal(&self, action: &'static str) -> Result<()> {
        if !self.chat.connected || crate::agent::picks::key(&self.settings) != "codex-acp" {
            return Err("Start a turn with Codex before controlling its goal".into());
        }
        self.agent
            .as_ref()
            .ok_or("The agent session has closed")?
            .control_goal(action)
    }

    pub(super) fn goal_bar(&mut self, ui: &mut egui::Ui) {
        let Some(goal) = &self.chat.goal else { return };
        if goal.status == GoalStatus::Active && self.chat.connected {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_secs(1));
        }
        let p = Palette::for_context(ui.ctx());
        let id = ui.id().with(("goal", self.chat_tabs.active));
        let mut open = ui.data(|data| data.get_temp::<bool>(id)).unwrap_or(false);
        let mut action = None;
        egui::Frame::new()
            .fill(p.window)
            .stroke(Stroke::new(1.0, p.line))
            .corner_radius(12)
            .inner_margin(egui::Margin::symmetric(12, 4))
            .show(ui, |ui| {
                let title = format!("{} · {}", goal.status.label(), goal.objective);
                let elapsed_ms = goal
                    .elapsed(ConnectionState::from(self.chat.connected))
                    .saturating_mul(1000);
                let elapsed = crate::ui::activity_view::duration(
                    i64::try_from(elapsed_ms).unwrap_or(i64::MAX),
                );
                if header(ui, &title, &elapsed, open).clicked() {
                    open = !open;
                    ui.data_mut(|data| data.insert_temp(id, open));
                }
                if open {
                    action = details(ui, goal, self.chat.connected, id);
                }
            });
        if let Some(action) = action
            && let Err(error) = self.control_goal(action)
        {
            self.report_error(error);
        }
    }
}

fn details(
    ui: &mut egui::Ui,
    goal: &crate::model::Goal,
    connected: bool,
    id: egui::Id,
) -> Option<&'static str> {
    egui::ScrollArea::vertical()
        .id_salt(id)
        .max_height(160.0)
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .show(ui, |ui| {
            ui.add(egui::Label::new(&goal.objective).selectable(true).wrap());
        });
    ui.horizontal(|ui| {
        if !connected {
            ui.weak("Saved goal · session closed");
            return None;
        }
        if goal.status == GoalStatus::Active && ui.small_button("Pause goal").clicked() {
            return Some("pause");
        }
        ui.small_button("Clear goal").clicked().then_some("clear")
    })
    .inner
}

fn header(ui: &mut egui::Ui, title: &str, elapsed: &str, open: bool) -> egui::Response {
    let p = Palette::for_context(ui.ctx());
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 28.0), Sense::click());
    let clock = ui
        .painter()
        .layout_no_wrap(elapsed.into(), FontId::monospace(11.5), p.muted);
    let clock_left = (rect.right() - 20.0 - clock.size().x).max(rect.left());
    let mut text =
        egui::text::LayoutJob::simple_singleline(title.into(), FontId::proportional(12.5), p.text3);
    text.wrap.max_rows = 1;
    text.wrap.max_width = (clock_left - rect.left() - 8.0).max(0.0);
    text.wrap.overflow_character = Some('…');
    let galley = ui.fonts_mut(|fonts| fonts.layout_job(text));
    ui.painter()
        .with_clip_rect(rect.intersect(ui.clip_rect()))
        .galley(
            rect.left_center() - egui::vec2(0.0, galley.size().y / 2.0),
            galley,
            p.text3,
        );
    ui.painter()
        .with_clip_rect(rect.intersect(ui.clip_rect()))
        .galley(
            egui::pos2(clock_left, rect.center().y - clock.size().y / 2.0),
            clock,
            p.muted,
        );
    let icon = if open {
        Icon::ChevronUp
    } else {
        Icon::Disclosure
    };
    icon.paint(
        ui,
        Rect::from_center_size(
            rect.right_center() - egui::vec2(5.0, 0.0),
            egui::vec2(11.0, 11.0),
        ),
        p.faint,
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::CollapsingHeader,
            ui.is_enabled(),
            format!("{title} · {elapsed}"),
        )
    });
    response.on_hover_text(title)
}
