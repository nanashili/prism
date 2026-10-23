use super::{Playbook, classification, source};
use crate::error::{Result, failure};
use crate::settings::skill_library::SkillLibrary;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

const MAX_ENTRIES: usize = 10_000;
const MAX_DEPTH: usize = 6;

pub(in crate::ui) struct Index {
    pub entries: Vec<Playbook>,
    pub warnings: Vec<String>,
}

pub(in crate::ui) fn index(custom: &Path, libraries: &[SkillLibrary]) -> Index {
    let mut index = Index {
        entries: Vec::new(),
        warnings: Vec::new(),
    };
    match std::fs::symlink_metadata(custom) {
        Ok(_) => index.add_folder(custom, None),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => index.warn(format!("{}: {error}", custom.display())),
    }
    for library in libraries.iter().filter(|library| library.enabled) {
        index.add_folder(&library.path, Some(library));
    }
    index
}

impl Index {
    fn add_folder(&mut self, folder: &Path, library: Option<&SkillLibrary>) {
        let result = discover(folder).and_then(|paths| {
            if paths.is_empty() {
                return Err(failure("No Markdown skills were found"));
            }
            for path in paths {
                match load(&path, library) {
                    Ok(entry) => self.entries.push(entry),
                    Err(error) => self.warn(format!("{}: {error}", path.display())),
                }
            }
            Ok(())
        });
        if let Err(error) = result {
            self.warn(format!("{}: {error}", folder.display()));
        }
    }

    fn warn(&mut self, message: String) {
        if self.warnings.len() < 20 {
            self.warnings.push(message);
        } else if self.warnings.len() == 20 {
            self.warnings.push(
                "Further skill errors were omitted. Check the library files and refresh.".into(),
            );
        }
    }
}

pub(in crate::ui) fn root(folder: &Path) -> Result<PathBuf> {
    let folder = folder.canonicalize()?;
    if !folder.is_dir() {
        return Err("Choose a skill folder or a repository containing a skills folder".into());
    }
    let skills = folder.join("skills");
    if skills.is_dir() {
        let root = skills.canonicalize()?;
        if !root.starts_with(&folder) {
            return Err("The repository's skills folder points outside the repository".into());
        }
        Ok(root)
    } else {
        Ok(folder)
    }
}

fn discover(folder: &Path) -> Result<Vec<PathBuf>> {
    let root = root(folder)?;
    let mut pending = vec![(root, 0)];
    let mut paths = Vec::new();
    let mut visited = 0;

    while let Some((folder, depth)) = pending.pop() {
        let skill = folder.join("SKILL.md");
        let is_skill = match std::fs::symlink_metadata(&skill) {
            Ok(metadata) => metadata.is_file(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(error) => return Err(error.into()),
        };
        if is_skill {
            paths.push(skill);
            if paths.len() > MAX_ENTRIES {
                return Err("Skill library exceeds the 10,000 skill limit".into());
            }
            continue;
        }
        for entry in std::fs::read_dir(&folder)? {
            let entry = entry?;
            visited += 1;
            if visited > MAX_ENTRIES * 4 {
                return Err("Skill library exceeds the 40,000 entry scan limit".into());
            }
            let kind = entry.file_type()?;
            let path = entry.path();
            if entry.file_name().to_string_lossy().starts_with('.') || kind.is_symlink() {
                continue;
            }
            if kind.is_dir() {
                if depth >= MAX_DEPTH {
                    return Err("Skill library exceeds the supported folder depth of 6".into());
                }
                pending.push((path, depth + 1));
            } else if kind.is_file() && path.extension().is_some_and(|ext| ext == "md") {
                paths.push(path);
            }
            if paths.len() > MAX_ENTRIES {
                return Err("Skill library exceeds the 10,000 skill limit".into());
            }
        }
    }
    paths.sort();
    Ok(paths)
}

fn load(path: &Path, library: Option<&SkillLibrary>) -> Result<Playbook> {
    let text = source::read_custom(path)?;
    let (metadata, _) = source::sections(&text);
    let fallback = if path.file_name().is_some_and(|name| name == "SKILL.md") {
        path.parent().and_then(Path::file_name)
    } else {
        path.file_stem()
    };
    let name = source::field(metadata, "name:");
    let name = if name.is_empty() {
        fallback.unwrap_or_default().to_string_lossy().into_owned()
    } else {
        name.to_owned()
    };
    if name.is_empty() || name.chars().any(char::is_whitespace) {
        return Err("Skill names must be nonempty and contain no spaces".into());
    }
    let mut categories = classification::categories(&name, metadata);
    if categories.is_empty()
        && let Some(library) = library
    {
        categories.clone_from(&library.fallback_categories);
    }
    let archived = name.starts_with("ags-")
        || source::field(metadata, "type:").contains("archive")
        || path
            .parent()
            .is_some_and(|folder| folder.join("source.txt").is_file());
    let id = if library.is_some() {
        {
            let fingerprint = format!("{:x}", Sha256::digest(path.to_string_lossy().as_bytes()));
            format!("library-{}:{name}", &fingerprint[..16])
        }
    } else {
        name.clone()
    };
    Ok(Playbook {
        id: id.into(),
        title: source::title(&name).into(),
        description: source::field(metadata, "description:").to_owned().into(),
        prompt: "".into(),
        built_in: false,
        categories,
        file: Some(path.to_path_buf()),
        library: library.map(|entry| entry.path.clone()),
        archived,
    })
}
