use super::{app::Prism, icons::Icon, theme::Palette, widgets};
use eframe::egui::{self, Align2, Color32, FontId, Frame, Rect, Sense};

const NEWS: [(&str, &str, &str); 3] = [
    (
        "Pick your agent",
        "Search the ACP agent catalog, see which agents are installed, and recognize each agent by its own logo.",
        "agents",
    ),
    (
        "Apps look like apps",
        "Opened apps use their Finder icons. Android, shaders and model files have recognizable format icons.",
        "apps",
    ),
    (
        "Your chats, close at hand",
        "Open, rename, copy or delete a saved chat from its context menu. Project chats stay together in the sidebar.",
        "chats",
    ),
];

impl Prism {
    pub(super) fn whats_new(&mut self, context: &egui::Context) {
        let Some(index) = self.news else {
            return;
        };
        let (title, body, art) = NEWS[index];
        let p = Palette::for_context(context);
        egui::Area::new(egui::Id::new("whats-new"))
            .order(egui::Order::Foreground)
            .anchor(Align2::LEFT_BOTTOM, [10.0, -10.0])
            .show(context, |ui| {
                Frame::NONE
                    .fill(p.raised)
                    .corner_radius(16)
                    .stroke(egui::Stroke::new(1.0, p.line))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 16],
                        blur: 40,
                        spread: 0,
                        color: Color32::from_black_alpha(89),
                    })
                    .show(ui, |ui| {
                        ui.set_width(298.0);
                        self.news_art(ui, art);
                        Frame::NONE
                            .inner_margin(egui::Margin {
                                left: 16,
                                right: 16,
                                top: 18,
                                bottom: 12,
                            })
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "What's new · {} of {}",
                                        index + 1,
                                        NEWS.len()
                                    ))
                                    .size(12.5)
                                    .color(p.muted),
                                );
                                ui.add_space(2.0);
                                ui.label(egui::RichText::new(title).size(18.0).color(p.text));
                                ui.add_space(6.0);
                                ui.label(egui::RichText::new(body).size(13.5).color(p.text3));
                                ui.add_space(16.0);
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::vec2(ui.available_width(), 36.0),
                                    Sense::click(),
                                );
                                ui.painter().rect_filled(rect, 10.0, p.sel);
                                ui.painter().text(
                                    rect.center(),
                                    Align2::CENTER_CENTER,
                                    if index + 1 == NEWS.len() {
                                        "Done"
                                    } else {
                                        "Next"
                                    },
                                    FontId::proportional(13.5),
                                    p.text,
                                );
                                if response.clicked() {
                                    self.news = (index + 1 < NEWS.len()).then_some(index + 1);
                                }
                                ui.add_space(12.0);
                                self.news_pager(ui, index);
                            });
                    });
            });
    }

    fn news_art(&mut self, ui: &mut egui::Ui, art: &str) {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(298.0, 172.0), Sense::hover());
        ui.painter().rect_filled(
            rect,
            egui::CornerRadius {
                nw: 15,
                ne: 15,
                sw: 0,
                se: 0,
            },
            Color32::from_rgb(242, 242, 241),
        );
        for y in (6..172).step_by(12) {
            for x in (6..298).step_by(12) {
                ui.painter().circle_filled(
                    rect.min + egui::vec2(x as f32, y as f32),
                    0.8,
                    Color32::from_gray(214),
                );
            }
        }
        for (index, name) in (if art == "agents" {
            ["claude-acp", "codex-acp"]
        } else {
            ["", ""]
        })
        .iter()
        .enumerate()
        {
            let center = rect.center() + egui::vec2((index as f32 - 0.5) * 70.0, 0.0);
            ui.painter().circle_filled(center, 30.0, Color32::WHITE);
            let icon = Rect::from_center_size(center, egui::vec2(30.0, 30.0));
            if !name.is_empty() {
                super::agent_logos::paint(ui, name, icon);
            } else {
                (if art == "apps" {
                    Icon::App
                } else {
                    Icon::Thread
                })
                .paint(ui, icon, Color32::from_gray(40));
            }
        }
        let close = Rect::from_min_size(
            rect.right_top() + egui::vec2(-32.0, 10.0),
            egui::vec2(22.0, 22.0),
        );
        Icon::Close.paint(ui, close.shrink(4.0), Color32::from_gray(184));
        if ui
            .interact(close, ui.id().with("close-news"), Sense::click())
            .clicked()
        {
            self.news = None;
        }
    }

    fn news_pager(&mut self, ui: &mut egui::Ui, index: usize) {
        ui.horizontal(|ui| {
            if ui
                .add_enabled_ui(index > 0, |ui| {
                    widgets::icon_button(ui, Icon::Back, "Previous", 26.0)
                })
                .inner
                .clicked()
            {
                self.news = Some(index - 1);
            }
            let width = ui.available_width() - 26.0;
            let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 26.0), Sense::hover());
            for at in 0..NEWS.len() {
                let center = rect.center() + egui::vec2((at as f32 - 1.0) * 12.0, 0.0);
                let dot = Rect::from_center_size(center, egui::vec2(12.0, 18.0));
                ui.painter().circle_filled(
                    center,
                    3.0,
                    if at == index {
                        Palette::for_context(ui.ctx()).text
                    } else {
                        Palette::for_context(ui.ctx()).line3
                    },
                );
                if ui.interact(dot, ui.id().with(at), Sense::click()).clicked() {
                    self.news = Some(at);
                }
            }
            if ui
                .add_enabled_ui(index + 1 < NEWS.len(), |ui| {
                    widgets::icon_button(ui, Icon::Forward, "Next", 26.0)
                })
                .inner
                .clicked()
            {
                self.news = Some(index + 1);
            }
        });
    }
}
