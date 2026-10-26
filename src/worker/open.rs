use super::State;
use crate::decompilation::Context;
use crate::error::Result;
use crate::events::Event;
use crate::model::{Project, View};
use crate::project::files;
use std::path::{Path, PathBuf};

impl State {
    pub(super) fn open(&mut self, path: PathBuf) -> Result<String> {
        let _transition = self.runtime.begin_transition()?;
        self.pause_decompilation()?;
        self.sink.analysis("Reading file", 0, None)?;
        self.sink.send(Event::Progress("Reading file".into()))?;
        self.engine = None;
        if let Some(project) = self.debug_project.take() {
            self.store.invalidate_runtime_evidence(&project)?;
        }

        let document = path
            .extension()
            .is_some_and(|extension| extension == "prism");
        let project = self.read_project(&path, document)?;
        let context = Context::new(&project.target.sha256, &self.settings)?;
        let analyzed = self.store.has_analysis(&project.target.sha256)?;
        self.store.save_project(&project)?;

        self.sink
            .send(Event::Opened(Box::new(project.clone()), document))?;
        if document {
            self.sink.send(Event::Saved(path))?;
        }
        let rows = self.store.functions(&project.target.sha256, "", 0)?;
        self.sink.send(Event::Functions { rows, offset: 0 })?;
        let kind = project.target.kind.clone();
        self.project = Some(project);
        self.runtime.set_context(context)?;
        self.sync_verification_context()?;

        match kind.as_str() {
            "app" | "binary" | "android" if !analyzed => self.index(),
            "app" | "binary" if self.settings.analysis_backend().retains_prepared_session() => {
                self.prepare_engine()?;
                Ok("Ghidra session ready".into())
            }
            "source" | "model" | "gpu" => {
                self.content(View::Pseudocode, String::new(), 0, None)?;
                Ok("File opened".into())
            }
            _ => Ok("Ready".into()),
        }
    }

    fn read_project(&self, path: &Path, document: bool) -> Result<Project> {
        if document {
            let mut protected = vec![path];
            if let Some(project) = &self.project {
                protected.push(&project.target.path);
            }
            return crate::project::open(
                path,
                &self.cancelled,
                self.settings.artifact_cache_mib * 1024 * 1024,
                &protected,
            );
        }

        let target = files::inspect(path, &self.cancelled)?;
        match self.store.project(&target.sha256) {
            Ok(mut saved) => {
                if saved.target.path != target.path {
                    saved.target = target;
                } else {
                    saved.target.size = target.size;
                }
                saved.source_document = None;
                Ok(saved)
            }
            Err(error)
                if error
                    .downcast_ref::<rusqlite::Error>()
                    .is_some_and(|error| matches!(error, rusqlite::Error::QueryReturnedNoRows)) =>
            {
                Ok(Project {
                    target,
                    ..Project::default()
                })
            }
            Err(error) => Err(error),
        }
    }
}
