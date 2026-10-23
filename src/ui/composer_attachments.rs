use super::app::Prism;
use crate::{error::Result, model::Attachment};

impl Prism {
    pub(super) fn pick_attachments(&mut self) {
        let dialog = rfd::FileDialog::new().set_title("Add attachments");

        #[cfg(target_os = "macos")]
        let paths = dialog.pick_files_or_folders();
        #[cfg(not(target_os = "macos"))]
        let paths = dialog.pick_files();

        if let Some(paths) = paths {
            self.attach_paths(paths);
        }
    }

    pub(super) fn attach_paths(&mut self, paths: Vec<std::path::PathBuf>) {
        let mut failures = Vec::new();
        if paths.len() > Attachment::LIMIT {
            failures.push("Only the first 32 dropped or selected items were considered".into());
        }
        for path in paths.into_iter().take(Attachment::LIMIT) {
            let result =
                Attachment::from_path(&path).and_then(|attachment| self.attach_item(attachment));
            if let Err(error) = result {
                failures.push(format!("{}: {error}", path.display()));
            }
        }
        if !failures.is_empty() {
            self.report_error(crate::error::failure(failures.join("\n")));
        }
    }

    pub(super) fn attach_item(&mut self, attachment: Attachment) -> Result<()> {
        if self
            .chat
            .attachments
            .iter()
            .any(|item| item.uri == attachment.uri)
        {
            return Ok(());
        }
        if self.chat.attachments.len() >= Attachment::LIMIT {
            return Err("A message can have up to 32 attachments".into());
        }
        let fresh = self.chat.id.is_none();
        let id = self.chat_for_prompt(&attachment.name)?;
        self.chat.attachments.push(attachment);
        self.save_chat_draft()?;

        if fresh {
            self.store.remember_chat(
                self.workspace
                    .project
                    .as_ref()
                    .map(|project| project.target.sha256.as_str()),
                Some(id),
            )?;
            self.refresh_chats()?;
        }
        Ok(())
    }
}
