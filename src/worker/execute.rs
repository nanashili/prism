use super::{Job, State};
use crate::error::{Result, failure};
use crate::events::Event;
use crate::project::files;
use std::io::Write;

impl State {
    pub(super) fn execute(&mut self, job: Job) -> Result<String> {
        match job {
            Job::Open(path) => self.open(path),
            Job::Index => self.index(),
            Job::Sidebar {
                project,
                generation,
                query,
            } => self.sidebar(project, generation, query),
            Job::Resources {
                project,
                generation,
                parent,
                offset,
            } => self.resource_page(project, generation, parent, offset),
            Job::View { request, job } => self.execute_view(request, job),
            Job::CopyCode { view, address } => {
                let text = self.read_view(view, &address, 0)?;
                self.sink.send(Event::Clipboard(text))?;
                Ok(format!("Copied {}", view.label().to_lowercase()))
            }
            Job::Save {
                path,
                mut project,
                messages,
            } => {
                crate::project::save(&path, &project, &messages)?;
                project.source_document = Some(path.clone());
                self.store.save_project(&project)?;
                self.project = Some(project);
                self.sink.send(Event::Saved(path))?;
                Ok("Project saved".into())
            }
            Job::Patch { path, project } => {
                crate::project::patch::export(&project, &path)?;
                self.sink.send(Event::Exported(path.clone()))?;
                Ok(format!("Patched copy written to {}", path.display()))
            }
            Job::Export { path, text } => {
                if let Some(project) = &self.project {
                    files::preserve_original(&project.target.path, &path)?;
                }
                crate::storage::atomic::write(&path, |file| {
                    file.write_all(text.as_bytes())?;
                    Ok(())
                })?;
                self.sink.send(Event::Exported(path.clone()))?;
                Ok(format!("Exported to {}", path.display()))
            }
            Job::ExportAnalysis { path, kind } => self.export_analysis(&path, kind),
            Job::Configure(settings) => self.configure(*settings),
            Job::Reconstruct { tool, args } => {
                let result = self.reconstruct(&tool, &args)?;
                if result["succeeded"] == false {
                    return Err(failure(format!(
                        "Reconstruction build failed. {}",
                        result["output"].as_str().unwrap_or_default()
                    )));
                }
                let location = result["snapshot"]
                    .as_str()
                    .or_else(|| result["report"].as_str());
                Ok(location.map_or_else(
                    || "Reconstruction updated".into(),
                    |path| format!("Reconstruction output: {path}"),
                ))
            }
            Job::Remote { .. } => Err(failure("Remote requests must use the reply channel")),
        }
    }

    fn configure(&mut self, settings: crate::settings::Settings) -> Result<String> {
        let _transition = self.runtime.begin_transition()?;
        let backend_changed = !settings.same_analysis_backend(&self.settings);
        let context = match &self.project {
            Some(project) if backend_changed => Some(crate::decompilation::Context::new(
                &project.target.sha256,
                &settings,
            )?),
            Some(_) => Some(self.decompilation_context()?),
            None => None,
        };

        if settings.cache_mib != self.settings.cache_mib {
            self.store.trim_cache(settings.cache_mib * 1024 * 1024)?;
        }
        if backend_changed {
            self.pause_decompilation()?;
            self.engine = None;
        }
        self.settings = settings;
        if let Some(context) = context {
            self.runtime.set_context(context)?;
        }
        Ok("Settings applied".into())
    }
}
