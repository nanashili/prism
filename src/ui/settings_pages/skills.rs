use super::model::{Command, Control, Group, Path, Row};
use crate::settings::Settings;
use crate::ui::playbooks::Playbook;
use std::sync::Arc;

pub(in crate::ui) fn groups<'a>(
    s: &'a mut Settings,
    playbooks: &[Arc<Playbook>],
) -> Vec<Group<'a>> {
    let built_in = playbooks
        .iter()
        .filter(|entry| entry.built_in)
        .map(|entry| skill(s, entry))
        .collect();
    let mut yours: Vec<_> = playbooks
        .iter()
        .filter(|entry| !entry.built_in)
        .map(|entry| skill(s, entry))
        .collect();
    let folder = super::paths::skills_dir(s);
    yours.push(super::paths::row(
        "Skills folder",
        &folder.to_string_lossy(),
        Path::Skills,
    ));
    let mut new_skill = Row::new(
        "New skill",
        "A Markdown file with a name, a one line description and the steps.",
    )
    .button("New skill…", Command::NewSkill);
    new_skill.buttons[0].primary = true;
    yours.push(new_skill);
    vec![Group::new("Built in", built_in), Group::new("Yours", yours)]
}

fn skill(s: &Settings, entry: &Playbook) -> Row<'static> {
    let description = if entry.relevant(s) {
        entry.description.to_string()
    } else {
        format!(
            "{} Hidden by workspace or library choices.",
            entry.description
        )
    };
    let mut row = Row::new(entry.title.as_ref(), description);
    row.control = Control::Skill(
        entry.setting_key(&s.desktop.disabled_skills).into(),
        !entry.disabled(&s.desktop.disabled_skills),
    );
    row
}
