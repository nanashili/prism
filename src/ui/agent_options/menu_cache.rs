use crate::agent::options::OptionMenu;
use crate::settings::Settings;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
pub(in crate::ui) struct MenuCache {
    current: RefCell<Option<Snapshot>>,
}

struct Snapshot {
    tab: u64,
    composer_revision: u64,
    models_revision: u64,
    agent_id: String,
    command: String,
    arguments: String,
    menus: Rc<[OptionMenu]>,
}

impl MenuCache {
    pub fn get(
        &self,
        tab: u64,
        composer_revision: u64,
        models_revision: u64,
        settings: &Settings,
        build: impl FnOnce() -> Vec<OptionMenu>,
    ) -> Rc<[OptionMenu]> {
        let mut current = self.current.borrow_mut();
        if let Some(snapshot) = current.as_ref()
            && snapshot.matches(tab, composer_revision, models_revision, settings)
        {
            return Rc::clone(&snapshot.menus);
        }

        let menus: Rc<[OptionMenu]> = build().into();
        *current = Some(Snapshot {
            tab,
            composer_revision,
            models_revision,
            agent_id: settings.agent_id.clone(),
            command: settings.agent.clone(),
            arguments: settings.agent_args.clone(),
            menus: Rc::clone(&menus),
        });
        menus
    }
}

impl Snapshot {
    fn matches(
        &self,
        tab: u64,
        composer_revision: u64,
        models_revision: u64,
        settings: &Settings,
    ) -> bool {
        self.tab == tab
            && self.composer_revision == composer_revision
            && self.models_revision == models_revision
            && self.agent_id == settings.agent_id
            && self.command == settings.agent
            && self.arguments == settings.agent_args
    }
}
