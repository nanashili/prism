use super::app::Prism;
use super::footer_rows::{self, Lead, Tail};
use super::{agent_picker, icons::Icon, popup, settings_navigation::SettingsPage};
use eframe::egui;

impl Prism {
    pub(super) fn footer_agents(&mut self, ui: &mut egui::Ui) {
        self.agent_catalog.start(ui.ctx());
        popup::heading(ui, "Agents");
        let id = agent_picker::id(&self.settings).to_owned();
        let name = agent_picker::label(&self.settings).to_owned();
        let status = if self.chat.busy {
            "Working"
        } else if self.chat.options.is_null() {
            "Not started"
        } else {
            "Connected"
        };
        ui.add_enabled_ui(!self.chat.busy, |ui| {
            if footer_rows::row(ui, Lead::Agent(&id), &name, status, Tail::Check(true)).clicked() {
                ui.close();
            }
            let installed: Vec<_> = self
                .agent_catalog
                .entries
                .iter()
                .filter(|entry| entry.id != id && self.agent_catalog.installed.contains(&entry.id))
                .map(|entry| (entry.id.clone(), entry.name.clone()))
                .collect();
            for (id, name) in installed {
                if footer_rows::row(ui, Lead::Agent(&id), &name, "Ready", Tail::Check(false))
                    .clicked()
                {
                    self.pick_agent(ui.ctx(), id, name);
                    ui.close();
                }
            }
        });
        ui.add_space(8.0);
        if footer_rows::row(
            ui,
            Lead::Icon(Icon::External),
            "Browse the agent registry",
            "",
            Tail::None,
        )
        .clicked()
        {
            ui.ctx()
                .open_url(egui::OpenUrl::new_tab("https://agentclientprotocol.com"));
            ui.close();
        }
        if footer_rows::row(ui, Lead::Icon(Icon::Gear), "Agent settings", "", Tail::None).clicked()
        {
            self.open_settings_page(SettingsPage::Agents);
            ui.close();
        }
    }

    pub(super) fn footer_playbooks(&mut self, ui: &mut egui::Ui) {
        let name = self
            .workspace
            .project
            .as_ref()
            .map_or("this file", |project| project.target.name.as_str());
        popup::heading(ui, &format!("Run on {name}"));
        ui.add_enabled_ui(!self.chat.busy, |ui| {
            for playbook in self.playbooks.enabled(&self.settings).into_iter().take(12) {
                if footer_rows::row(
                    ui,
                    Lead::Icon(Icon::Bolt),
                    &playbook.title,
                    &playbook.description,
                    Tail::None,
                )
                .clicked()
                {
                    self.run_playbook(&playbook);
                    ui.close();
                }
            }
        });
        if ui.link("Browse all skills").clicked() {
            self.open_settings_page(SettingsPage::Skills);
            ui.close();
        }
    }
}
