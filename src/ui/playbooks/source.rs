use super::Playbook;
use std::path::Path;

pub(super) fn built_in() -> Vec<Playbook> {
    [
        include_str!("../../../skills/first-look/SKILL.md"),
        include_str!("../../../skills/explain-function/SKILL.md"),
        include_str!("../../../skills/find/SKILL.md"),
        include_str!("../../../skills/name-functions/SKILL.md"),
        include_str!("../../../skills/trace-feature/SKILL.md"),
        include_str!("../../../skills/network-map/SKILL.md"),
        include_str!("../../../skills/decompile/SKILL.md"),
        include_str!("../../../skills/recompile/SKILL.md"),
        include_str!("../../../skills/shader-check/SKILL.md"),
    ]
    .into_iter()
    .map(|source| {
        let (metadata, body) = sections(source);
        let id = field(metadata, "name:");
        Playbook {
            id: id.into(),
            title: title(id).into(),
            description: field(metadata, "description:").into(),
            prompt: body.trim().into(),
            built_in: true,
            categories: super::classification::built_in(id),
            file: None,
            library: None,
            archived: false,
        }
    })
    .collect()
}

pub(super) fn read_custom(path: &Path) -> crate::error::Result<String> {
    use std::io::Read;
    const MAX_BYTES: u64 = 128 * 1024;

    if std::fs::symlink_metadata(path)?.is_symlink() {
        return Err("Symbolic links are not loaded as skills".into());
    }
    let file = std::fs::File::open(path)?;
    if file.metadata()?.len() > MAX_BYTES {
        return Err("Skill exceeds 128 KB".into());
    }
    let mut source = String::new();
    file.take(MAX_BYTES + 1).read_to_string(&mut source)?;
    if source.len() > MAX_BYTES as usize {
        return Err("Skill exceeds 128 KB".into());
    }
    Ok(source.replace("\r\n", "\n"))
}

pub(super) fn sections(source: &str) -> (&str, &str) {
    source
        .strip_prefix("---\n")
        .and_then(|text| {
            text.split_once("\n---\n")
                .or_else(|| text.strip_suffix("\n---").map(|metadata| (metadata, "")))
        })
        .unwrap_or(("", source))
}

pub(super) fn field<'a>(metadata: &'a str, name: &str) -> &'a str {
    metadata
        .lines()
        .find_map(|line| line.trim_start().strip_prefix(name))
        .unwrap_or("")
        .trim()
        .trim_matches(['"', '\''])
}

pub(super) fn title(id: &str) -> String {
    match id {
        "explain-function" => "Explain a function".into(),
        "trace-feature" => "Trace a feature".into(),
        _ => {
            let title = id.replace(['-', '_'], " ");
            let Some(first) = title.chars().next() else {
                return title;
            };
            format!("{}{}", first.to_uppercase(), &title[first.len_utf8()..])
        }
    }
}
