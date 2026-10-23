use crate::storage::navigation::Preferences;
use crate::ui::app::Prism;

impl Prism {
    pub(in crate::ui) fn restore_sidebar_preferences(&mut self) {
        let Some(project) = &self.workspace.project else {
            return;
        };
        match self.store.sidebar_preferences(&project.target.sha256) {
            Ok(preferences) => {
                let navigation = &mut self.workspace.navigation;
                navigation.query.sort = preferences.sort;
                navigation.query.show = preferences.show;
                navigation.hidden = preferences.hidden;
                navigation.closed = ["chats", "work", "resources", "functions"]
                    .into_iter()
                    .filter(|key| preferences.closed.iter().any(|closed| closed == key))
                    .collect();
            }
            Err(error) => self.error = Some(format!("Could not restore the sidebar: {error}")),
        }
    }

    pub(in crate::ui) fn save_sidebar_preferences(&mut self) {
        let Some(project) = &self.workspace.project else {
            return;
        };
        let navigation = &self.workspace.navigation;
        let preferences = Preferences {
            closed: navigation.closed.iter().map(|key| (*key).into()).collect(),
            sort: navigation.query.sort,
            show: navigation.query.show,
            hidden: navigation.hidden,
        };
        if let Err(error) = self
            .store
            .save_sidebar_preferences(&project.target.sha256, &preferences)
        {
            self.error = Some(format!("Could not save the sidebar: {error}"));
        }
    }
}
