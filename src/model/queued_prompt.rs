use super::Attachment;
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct QueuedPrompt {
    pub text: String,
    pub attachments: Vec<Attachment>,
}

impl QueuedPrompt {
    pub const LIMIT: usize = 8;
    pub const BYTE_LIMIT: usize = 2 * 1024 * 1024;

    pub fn decode(bytes: &[u8]) -> Result<VecDeque<Self>> {
        if bytes.len() > Self::BYTE_LIMIT {
            return Err("The saved follow-up queue exceeds its size limit".into());
        }
        let mut queue: VecDeque<Self> = serde_json::from_slice(bytes)?;
        if queue.len() > Self::LIMIT {
            return Err("The saved chat has too many queued follow-ups".into());
        }
        for item in &mut queue {
            if item.text.chars().count() > 48_000 || item.attachments.len() > Attachment::LIMIT {
                return Err("A queued follow-up exceeds its size limit".into());
            }
            item.attachments = item
                .attachments
                .iter()
                .map(|attachment| Attachment::from_location(&attachment.uri))
                .collect::<Result<_>>()?;
        }
        Ok(queue)
    }
}
