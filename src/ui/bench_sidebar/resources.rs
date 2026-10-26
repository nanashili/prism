use crate::storage::resources::ResourcePage;
use crate::ui::app::Prism;
use crate::ui::presentation::Disclosure;
use crate::worker::Job;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub(super) struct Resources {
    pub(super) generation: u64,
    pub(super) pages: BTreeMap<String, (ResourcePage, usize)>,
    pub(super) expanded: BTreeSet<String>,
    pub(super) errors: BTreeMap<String, String>,
    pub(super) pending: BTreeSet<String>,
}

impl Resources {
    fn discard_descendants(&mut self, parent: &str) {
        let prefix = format!("{parent}/");
        let keep = |key: &str| key == parent || (!parent.is_empty() && !key.starts_with(&prefix));

        self.expanded.retain(|key| keep(key));
        self.pending.retain(|key| keep(key));
        self.errors.retain(|key, _| keep(key));
        self.pages.retain(|key, _| keep(key));
    }

    fn discard_branch(&mut self, parent: &str) {
        self.discard_descendants(parent);

        self.expanded.remove(parent);
        self.pending.remove(parent);
        self.errors.remove(parent);
        self.pages.remove(parent);
    }
}

impl Prism {
    pub(super) fn toggle_resource_folder(&mut self, path: &str, open: Disclosure) {
        let open = open == Disclosure::Open;

        let resources = &mut self.workspace.navigation.resources;
        if open {
            resources.discard_branch(path);
        } else if resources.pages.len() < 17 {
            resources.expanded.insert(path.into());
            self.request_resource_page(path.into(), 0);
        } else {
            self.status = "Close a resource folder before opening another.".into();
        }
    }

    pub(super) fn request_resource_page(&mut self, parent: String, offset: usize) {
        let Some(project) = &self.workspace.project else {
            return;
        };
        let job = Job::Resources {
            project: project.target.sha256.clone(),
            generation: self.workspace.navigation.resources.generation,
            parent: parent.clone(),
            offset,
        };
        match self.submit(job) {
            Ok(()) => {
                let resources = &mut self.workspace.navigation.resources;
                if resources
                    .pages
                    .get(&parent)
                    .is_some_and(|(_, previous)| *previous != offset)
                {
                    resources.discard_descendants(&parent);
                }
                resources.pending.insert(parent.clone());
                resources.errors.remove(&parent);
                resources.pages.insert(
                    parent,
                    (
                        ResourcePage {
                            available: true,
                            ..Default::default()
                        },
                        offset,
                    ),
                );
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    pub(in crate::ui) fn receive_resources(
        &mut self,
        project: &str,
        generation: u64,
        parent: String,
        offset: usize,
        page: std::result::Result<ResourcePage, String>,
    ) {
        if self
            .workspace
            .project
            .as_ref()
            .is_some_and(|current| current.target.sha256 == project)
            && self.workspace.navigation.resources.generation == generation
            && self
                .workspace
                .navigation
                .resources
                .pages
                .get(&parent)
                .is_some_and(|(_, requested)| *requested == offset)
        {
            let resources = &mut self.workspace.navigation.resources;
            resources.pending.remove(&parent);
            match page {
                Ok(page) => {
                    resources.errors.remove(&parent);
                    resources.pages.insert(parent, (page, offset));
                }
                Err(error) => {
                    resources.errors.insert(parent.clone(), error);
                    resources.pages.insert(
                        parent,
                        (
                            ResourcePage {
                                available: true,
                                ..Default::default()
                            },
                            offset,
                        ),
                    );
                }
            }
        }
    }
}
