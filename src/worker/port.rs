use super::Job;
use crate::error::{Result, failure};
use std::sync::mpsc::SyncSender;

#[derive(Clone)]
pub(crate) struct WorkerPort {
    pub(super) ordinary: SyncSender<Job>,
    pub(super) controls: SyncSender<Job>,
}

impl WorkerPort {
    pub(crate) fn submit(&self, job: Job) -> Result<()> {
        let sender = if matches!(&job, Job::Remote { tool, args, .. } if prism::mcp::durable_control(tool, args))
        {
            &self.controls
        } else {
            &self.ordinary
        };
        sender
            .try_send(job)
            .map_err(|error| failure(format!("The analysis queue is busy: {error}")))
    }
}
