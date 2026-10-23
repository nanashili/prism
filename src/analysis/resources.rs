use crate::error::{Result, failure};
use crate::model::Target;
use crate::project::files;
use std::path::{Component, Path, PathBuf};

pub(crate) fn root(target: &Target) -> Option<PathBuf> {
    target
        .icon_path()
        .ancestors()
        .chain(target.path.ancestors())
        .find(|path| path.extension().is_some_and(|extension| extension == "app") && path.is_dir())
        .map(Path::to_path_buf)
}

pub(crate) fn read(target: &Target, relative: &str) -> Result<String> {
    let root = root(target)
        .ok_or("This target has no resource directory")?
        .canonicalize()?;
    let path = Path::new(relative);
    if path
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(failure("Choose a resource inside this app"));
    }
    let path = root.join(path).canonicalize()?;
    if !path.starts_with(root) || !path.is_file() {
        return Err(failure("The resource is outside this app or is not a file"));
    }
    let header = files::bytes(&path, 0, 4096)?;
    if header.contains(&0) || std::str::from_utf8(&header).is_err() {
        return Ok(format!(
            "{}\n{} bytes\n\n{}",
            relative,
            std::fs::metadata(&path)?.len(),
            files::hex(&path, 0)?
        ));
    }
    files::text(&path)
}
