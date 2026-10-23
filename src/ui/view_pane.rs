use super::view_tab::ViewTab;

pub(super) const MAX_TABS: usize = 32;

pub(super) struct ViewPane {
    pub identity: u64,
    pub tabs: Vec<ViewTab>,
    pub active: usize,
    pub next_tab_identity: u64,
    pub reveal_active_tab: bool,
}

impl Default for ViewPane {
    fn default() -> Self {
        Self {
            identity: 0,
            tabs: vec![ViewTab {
                tab_closed: true,
                ..Default::default()
            }],
            active: 0,
            next_tab_identity: 0,
            reveal_active_tab: true,
        }
    }
}

impl ViewPane {
    pub fn duplicate_selection(&self) -> Self {
        Self {
            tabs: vec![self.tabs[self.active].duplicate_selection()],
            ..Default::default()
        }
    }

    pub fn select_tab(&mut self, index: usize) -> bool {
        if index == self.active || index >= self.tabs.len() {
            return false;
        }
        self.tabs[self.active].release_content();
        self.active = index;
        self.reveal_active_tab = true;
        true
    }

    pub fn open_tab(&mut self, mut tab: ViewTab) -> crate::error::Result<()> {
        let replace = self
            .tabs
            .iter()
            .position(|tab| tab.preview || tab.tab_closed);
        if replace.is_none() && self.tabs.len() == MAX_TABS {
            return Err(crate::error::failure(
                "Close a tab before opening more than 32 tabs in a pane",
            ));
        }
        self.tabs[self.active].release_content();
        self.next_tab_identity = self.next_tab_identity.wrapping_add(1);
        tab.identity = self.next_tab_identity;
        if let Some(index) = replace {
            self.tabs[index] = tab;
            self.active = index;
        } else {
            self.active = self.tabs.len();
            self.tabs.push(tab);
        }
        self.reveal_active_tab = true;
        Ok(())
    }

    pub fn close_tab(&mut self, index: usize) -> bool {
        if self.tabs.len() == 1 {
            return false;
        }
        self.tabs.remove(index);
        if index < self.active {
            self.active -= 1;
        } else if self.active >= self.tabs.len() {
            self.active = self.tabs.len() - 1;
        }
        self.reveal_active_tab = true;
        true
    }
}

impl std::ops::Deref for ViewPane {
    type Target = ViewTab;

    fn deref(&self) -> &Self::Target {
        &self.tabs[self.active]
    }
}

impl std::ops::DerefMut for ViewPane {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tabs[self.active]
    }
}
