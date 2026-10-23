use crate::agent::options::OptionMenu;
use crate::ui::{app::Prism, icons::Icon, playbooks::Playbook};
use std::sync::Arc;

#[derive(Clone)]
pub(super) struct Item {
    pub label: String,
    pub description: String,
    pub hint: String,
    pub icon: Icon,
    pub action: Action,
}

#[derive(Clone)]
pub(super) enum Action {
    Skill(Arc<Playbook>),
    Command(String),
    Option(OptionMenu, String),
    Attach,
}

impl Prism {
    pub(super) fn palette_items(&mut self, query: &str) -> Vec<Item> {
        let mut items = Vec::new();
        self.palette_options(&mut items);
        if self.settings.desktop.skills_in_composer {
            items.extend(
                self.playbooks
                    .enabled(&self.settings)
                    .into_iter()
                    .map(|skill| Item {
                        label: skill.title.to_string(),
                        description: format!("{} · {}", skill.id, skill.description),
                        hint: "Skill".into(),
                        icon: Icon::Spark,
                        action: Action::Skill(skill),
                    }),
            );
        }
        items.extend(self.chat.commands.iter().map(|command| Item {
            label: format!("/{}", command.name),
            description: format!("{} {}", command.description, command.hint),
            hint: "Command".into(),
            icon: Icon::Slash,
            action: Action::Command(command.name.clone()),
        }));
        items.push(Item {
            label: "Files".into(),
            description: "Add files, images or folders to look at".into(),
            hint: "Attach".into(),
            icon: Icon::Source,
            action: Action::Attach,
        });
        let query = query.trim().to_lowercase();
        items.retain(|item| {
            item.label.to_lowercase().contains(&query)
                || item.description.to_lowercase().contains(&query)
        });
        items.truncate(80);
        items
    }

    fn palette_options(&self, items: &mut Vec<Item>) {
        for mut menu in crate::agent::options::menus(&self.chat.options) {
            if let Some(pick) = self.chat.picks.iter().find(|pick| pick.matches(&menu)) {
                menu.current.clone_from(&pick.value);
            }
            if menu.category == "mode" {
                for choice in &menu.choices {
                    items.push(Item {
                        label: choice.name.clone(),
                        description: choice.description.clone(),
                        hint: if menu.current == choice.value {
                            "Selected"
                        } else {
                            "Mode"
                        }
                        .into(),
                        icon: crate::ui::agent_option_rows::mode_icon(&choice.value, &choice.name),
                        action: Action::Option(menu.clone(), choice.value.clone()),
                    });
                }
            } else if is_switch(&menu) {
                let next = menu
                    .choices
                    .iter()
                    .find(|choice| choice.value != menu.current);
                if let Some(next) = next {
                    items.push(Item {
                        label: menu.name.clone(),
                        description: next.description.clone(),
                        hint: menu
                            .choices
                            .iter()
                            .find(|choice| choice.value == menu.current)
                            .map_or("", |choice| choice.name.as_str())
                            .into(),
                        icon: Icon::Bolt,
                        action: Action::Option(menu.clone(), next.value.clone()),
                    });
                }
            }
        }
    }
}

fn is_switch(menu: &OptionMenu) -> bool {
    menu.choices.len() == 2
        && menu.choices.iter().all(|choice| {
            matches!(
                choice.value.as_str(),
                "true" | "false" | "on" | "off" | "enabled" | "disabled"
            )
        })
}
