use crate::ui::app::Prism;

#[derive(Clone)]
pub(in crate::ui) struct Mention {
    pub label: String,
    pub address: String,
    pub kind: &'static str,
}

impl Prism {
    pub(super) fn mention_items(&self, query: &str) -> crate::error::Result<Vec<Mention>> {
        let Some(project) = &self.workspace.project else {
            return Ok(Vec::new());
        };
        let query = query.trim().to_lowercase();
        let label = |address: &String| project.renamed.get(address).unwrap_or(address).clone();
        let mut entries = Vec::new();
        for address in project.bookmarks.iter().take(2000) {
            let name = label(address);
            if name.to_lowercase().contains(&query) || address.to_lowercase().contains(&query) {
                entries.push(Mention {
                    label: name,
                    address: address.clone(),
                    kind: "bookmark",
                });
            }
            if entries.len() == 50 {
                return Ok(entries);
            }
        }
        for (address, note) in project.notes.iter().take(2000) {
            let name = label(address);
            if name.to_lowercase().contains(&query) || note.to_lowercase().contains(&query) {
                entries.push(Mention {
                    label: name,
                    address: address.clone(),
                    kind: "note",
                });
            }
            if entries.len() == 50 {
                return Ok(entries);
            }
        }
        for function in self.store.functions(&project.target.sha256, &query, 0)? {
            let name = project
                .renamed
                .get(&function.address)
                .unwrap_or(&function.name);
            if !entries
                .iter()
                .any(|entry| entry.address == function.address)
            {
                entries.push(Mention {
                    label: name.clone(),
                    address: function.address,
                    kind: "function",
                });
            }
            if entries.len() == 50 {
                break;
            }
        }
        Ok(entries)
    }
}

pub(in crate::ui) fn with_mentions(text: &str, mentions: &[Mention]) -> String {
    let mut result = text.to_owned();
    for mention in mentions {
        let reference = format!(
            "@{} is the {} at {}.",
            mention.label, mention.kind, mention.address
        );
        if text.contains(&format!("@{}", mention.label)) && !text.contains(&reference) {
            result.push_str("\n\n");
            result.push_str(&reference);
        }
    }
    result
}
