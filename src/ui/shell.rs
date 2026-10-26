use super::{app::Prism, theme::Palette};
use crate::model::Screen;
use crate::ui::presentation::Disclosure;
use eframe::egui::{self, Frame, Margin, Stroke};

impl Prism {
    #[cfg_attr(feature = "profiling", hotpath::measure)]
    pub(super) fn shell(&mut self, context: &egui::Context) {
        let palette = Palette::for_context(context);
        let background = self.window_effects.sidebar(
            palette.window,
            palette.sidebar,
            self.settings.transparency,
        );
        let sidebar_expanded = super::panel_layout::expansion(
            context,
            "navigation-expansion",
            Disclosure::from(self.screen != Screen::Workbench || !self.workspace.navigation.hidden),
        );
        self.sidebar(context, sidebar_expanded);

        egui::CentralPanel::default()
            .frame(
                Frame::NONE
                    .fill(background)
                    .inner_margin(self.content_margin(context, sidebar_expanded)),
            )
            .show(context, |ui| {
                Frame::NONE
                    .fill(palette.card)
                    .stroke(Stroke::new(1.0, palette.sel))
                    .corner_radius(12)
                    .inner_margin(0)
                    .show(ui, |ui| {
                        ui.set_min_size(ui.available_size());
                        if self.screen != Screen::Workbench && self.analysis.is_some() {
                            egui::TopBottomPanel::bottom("analysis-status")
                                .exact_height(28.0)
                                .show_separator_line(false)
                                .frame(Frame::NONE)
                                .show_inside(ui, |ui| self.workbench_status(ui));
                        }
                        self.content(ui);
                    });
            });
    }

    fn content_margin(&self, context: &egui::Context, sidebar_expanded: f32) -> Margin {
        let collapsed = if self.screen == Screen::Setup {
            1.0
        } else if self.screen == Screen::Workbench {
            1.0 - sidebar_expanded
        } else {
            0.0
        };
        let reserve_titlebar = cfg!(target_os = "macos")
            && !context.input(|input| input.viewport().fullscreen.unwrap_or(false));

        Margin {
            left: (8.0 * collapsed).round() as i8,
            right: 8,
            top: if reserve_titlebar {
                (8.0 + 38.0 * collapsed).round() as i8
            } else {
                8
            },
            bottom: 8,
        }
    }
}
