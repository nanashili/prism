use super::{Catalog, Playbook};
use crate::error::{Result, failure};
use crate::settings::Settings;

impl Playbook {
    pub fn reference(&self) -> String {
        format!("${}", self.id)
    }

    pub fn with_reference(&self, draft: &str) -> String {
        if self.is_referenced(draft) {
            return draft.to_owned();
        }
        format!("{} {}", self.reference(), draft)
    }

    fn is_referenced(&self, text: &str) -> bool {
        let reference = self.reference();
        text.match_indices(&reference).any(|(start, _)| {
            let before = &text[..start];
            let after = &text[start + reference.len()..];
            (before.is_empty() || before.ends_with(char::is_whitespace))
                && (after.is_empty() || after.starts_with(char::is_whitespace))
        })
    }
}

impl Catalog {
    pub fn resolve_prompt(&mut self, settings: &Settings, text: &str) -> Result<String> {
        let mut prompt = text.to_owned();
        // Adapter commands must keep their original syntax and position.
        if !text.contains('$') || text.trim_start().starts_with('/') {
            return Ok(prompt);
        }

        self.refresh(settings);
        if self.loading() {
            return Err("Skills are still being indexed. Try again when indexing finishes.".into());
        }
        for skill in &self.entries {
            if !skill.is_referenced(text) {
                continue;
            }
            if !skill.available(settings) {
                return Err(failure(format!(
                    "The {} skill is unavailable for this workspace. Check its category and library in Settings or remove {}.",
                    skill.title,
                    skill.reference()
                )));
            }
            prompt.push_str("\n\nApply the following Prism skill to this request: ");
            prompt.push_str(&skill.id);
            prompt.push('\n');
            prompt.push_str(&skill.read_prompt()?);
        }
        for reference in text
            .split_whitespace()
            .filter(|word| word.starts_with("$library-"))
        {
            if !self
                .entries
                .iter()
                .any(|skill| skill.reference() == reference)
            {
                return Err(failure(format!(
                    "The skill {reference} is no longer available. Enable its library in Settings or remove the reference."
                )));
            }
        }
        Ok(prompt)
    }
}
