mod classification;
pub(super) mod download;
mod invocation;
pub(super) mod library;
mod source;
#[cfg(test)]
mod tests;

use crate::settings::{Settings, research::Category, skill_library::SkillLibrary};
use std::borrow::Cow;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{
    Arc,
    mpsc::{self, Receiver},
};

#[derive(Clone, PartialEq)]
struct Sources {
    folder: PathBuf,
    libraries: Vec<SkillLibrary>,
}

#[derive(Default)]
pub(super) struct Catalog {
    sources: Option<Sources>,
    pending: Option<Receiver<library::Index>>,
    entries: Vec<Arc<Playbook>>,
    pub warnings: Vec<String>,
}

impl Catalog {
    pub fn all(&mut self, settings: &Settings) -> Vec<Arc<Playbook>> {
        self.refresh(settings);
        self.entries.clone()
    }

    fn refresh(&mut self, settings: &Settings) {
        let sources = Sources {
            folder: super::settings_pages::paths::skills_dir(settings),
            libraries: settings
                .desktop
                .skill_libraries
                .iter()
                .filter(|entry| entry.enabled)
                .cloned()
                .collect(),
        };
        if self.sources.as_ref() != Some(&sources) {
            self.start(sources);
        }
        self.poll();
    }

    fn start(&mut self, sources: Sources) {
        self.entries = source::built_in().into_iter().map(Arc::new).collect();
        self.warnings.clear();
        self.pending = None;
        self.sources = Some(sources.clone());
        let (sender, receiver) = mpsc::channel();
        let launched = std::thread::Builder::new()
            .name("skill-index".into())
            .spawn(move || {
                let index = library::index(&sources.folder, &sources.libraries);
                // A changed source configuration can retire this receiver while indexing.
                let _ = sender.send(index);
            });
        match launched {
            Ok(_) => self.pending = Some(receiver),
            Err(error) => self
                .warnings
                .push(format!("Could not index skills: {error}")),
        }
    }

    pub fn poll(&mut self) {
        let Some(receiver) = &self.pending else {
            return;
        };
        let index = match receiver.try_recv() {
            Ok(index) => index,
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.pending = None;
                self.warnings
                    .push("The skill index worker stopped. Refresh to try again.".into());
                return;
            }
        };
        self.pending = None;
        self.warnings = index.warnings;
        let mut identifiers: HashSet<_> = self
            .entries
            .iter()
            .map(|entry| entry.id.to_string())
            .collect();
        for entry in index.entries {
            if !identifiers.insert(entry.id.to_string()) {
                self.warnings.push(format!(
                    "Duplicate skill name: {}. Rename the custom skill to keep both.",
                    entry.id
                ));
            } else {
                self.entries.push(Arc::new(entry));
            }
        }
        self.entries.sort_by(|a, b| {
            b.built_in
                .cmp(&a.built_in)
                .then_with(|| a.title.cmp(&b.title))
        });
    }

    pub fn loading(&self) -> bool {
        self.pending.is_some()
    }

    pub fn enabled(&mut self, settings: &Settings) -> Vec<Arc<Playbook>> {
        self.refresh(settings);
        self.entries
            .iter()
            .filter(|entry| entry.available(settings))
            .cloned()
            .collect()
    }

    pub fn invalidate(&mut self) {
        self.sources = None;
    }
}

pub(super) struct Playbook {
    pub id: Cow<'static, str>,
    pub title: Cow<'static, str>,
    pub description: Cow<'static, str>,
    pub prompt: Cow<'static, str>,
    pub built_in: bool,
    pub categories: Vec<Category>,
    pub file: Option<PathBuf>,
    pub library: Option<PathBuf>,
    pub archived: bool,
}

impl Playbook {
    pub fn relevant(&self, settings: &Settings) -> bool {
        if !settings.research.matches(&self.categories) {
            return false;
        }
        match &self.library {
            None => true,
            Some(path) => settings.desktop.skill_libraries.iter().any(|library| {
                library.path == *path
                    && library.enabled
                    && (!self.archived || library.include_archives)
            }),
        }
    }

    pub fn available(&self, settings: &Settings) -> bool {
        self.relevant(settings) && !self.disabled(&settings.desktop.disabled_skills)
    }

    pub fn read_prompt(&self) -> crate::error::Result<String> {
        let Some(path) = &self.file else {
            return Ok(self.prompt.to_string());
        };
        let text = source::read_custom(path)?;
        let (_, body) = source::sections(&text);
        if body.trim().is_empty() {
            return Err("This skill has no instructions".into());
        }
        Ok(format!(
            "Skill folder: {}\nCheck this folder for supporting files when needed. Downloaded libraries contain skill Markdown and license files only; scripts and source archives may be absent.\n\n{}",
            path.parent().unwrap_or(path).display(),
            body.trim()
        ))
    }

    pub fn disabled(&self, disabled: &[String]) -> bool {
        disabled
            .iter()
            .any(|id| id == self.id.as_ref() || id == self.title.as_ref())
    }

    pub fn setting_key<'a>(&'a self, disabled: &[String]) -> &'a str {
        if disabled.iter().any(|id| id == self.title.as_ref()) {
            &self.title
        } else {
            &self.id
        }
    }
}

pub(super) fn shown_text(text: &str) -> Cow<'_, str> {
    let Some((heading, body)) = text.split_once('\n') else {
        return text.into();
    };
    let name = heading
        .strip_prefix("Follow this Prism playbook, \"")
        .and_then(|name| name.strip_suffix("\"."));
    let Some(name) = name else {
        return text.into();
    };
    let title = source::title(name);
    match body.split_once("\n\nThe user added: ") {
        Some((_, extra)) => format!("{title}: {extra}").into(),
        None => title.into(),
    }
}
