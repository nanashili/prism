use super::types::{Finding, Graph, Kind, Relationship, Report, TaskState};
use crate::error::{Result, failure};

impl Graph {
    pub(super) fn record(&mut self, id: u32, attempt: u32, report: Report) -> Result<()> {
        let task = self.claimed_task(id, attempt)?;
        let synthesis = task.kind == Kind::Synthesize;
        validate_report(&report)?;

        let bytes = u32::try_from(serde_json::to_vec(&report)?.len())?;
        let used = self
            .usage
            .evidence_bytes
            .checked_add(bytes)
            .ok_or("Investigation evidence size overflowed")?;
        if used > self.budget.evidence_bytes {
            return Err(failure(
                "The report exceeds the evidence budget; shorten it, record a blocker, or increase the budget",
            ));
        }

        let expanded = self.expand(id, &report)?;
        self.usage.evidence_bytes = used;
        let task = self.task_mut(id)?;
        task.findings.push(Finding {
            attempt,
            text: report.findings,
        });
        task.state = if synthesis && expanded {
            TaskState::Pending
        } else {
            TaskState::Complete
        };
        Ok(())
    }

    fn expand(&mut self, parent: u32, report: &Report) -> Result<bool> {
        let (dependencies, depth) = self.discovery_context(parent)?;
        let mut additions = self.add_function_leads(&dependencies, depth, &report.functions)?;
        additions.extend(self.add_relationship_leads(&dependencies, depth, &report.unresolved)?);

        let synthesis = self
            .tasks
            .iter_mut()
            .find(|task| task.kind == Kind::Synthesize)
            .ok_or("The investigation has no synthesis task")?;
        let expanded = !additions.is_empty();
        synthesis.dependencies.extend(additions);
        Ok(expanded)
    }

    fn discovery_context(&self, parent: u32) -> Result<(Vec<u32>, u32)> {
        let parent_task = self
            .tasks
            .iter()
            .find(|task| task.id == parent)
            .ok_or("Unknown investigation task")?;
        // Synthesis can discover missing evidence. Depend on its completed inputs,
        // so the new work can run before synthesis is claimed again without a cycle.
        let dependencies = if parent_task.kind == Kind::Synthesize {
            parent_task.dependencies.clone()
        } else {
            vec![parent]
        };
        let depth = self
            .tasks
            .iter()
            .filter(|task| dependencies.contains(&task.id))
            .map(|task| task.depth)
            .max()
            .unwrap_or(parent_task.depth);
        Ok((dependencies, depth + 1))
    }

    fn add_function_leads(
        &mut self,
        dependencies: &[u32],
        depth: u32,
        addresses: &[String],
    ) -> Result<Vec<u32>> {
        let mut additions = Vec::new();
        for address in addresses {
            if self.tasks.iter().any(
                |task| matches!(&task.kind, Kind::Inspect { address: saved } if saved == address),
            ) {
                continue;
            }
            additions.push(self.add_task(
                Kind::Inspect {
                    address: address.clone(),
                },
                format!("Investigate discovered function {address}"),
                dependencies.to_vec(),
                depth,
            )?);
        }
        Ok(additions)
    }

    fn add_relationship_leads(
        &mut self,
        dependencies: &[u32],
        depth: u32,
        relationships: &[Relationship],
    ) -> Result<Vec<u32>> {
        let mut additions = Vec::new();
        for relationship in relationships {
            let exists = self.tasks.iter().any(|task| match &task.kind {
                Kind::Resolve {
                    from, to, question, ..
                } => {
                    from == &relationship.from
                        && to == &relationship.to
                        && question == &relationship.question
                }
                _ => false,
            });
            if exists {
                continue;
            }
            additions.push(self.add_task(
                Kind::Resolve {
                    from: relationship.from.clone(),
                    to: relationship.to.clone(),
                    question: relationship.question.clone(),
                    evidence: relationship.evidence.clone(),
                },
                relationship.question.clone(),
                dependencies.to_vec(),
                depth,
            )?);
        }
        Ok(additions)
    }

    pub(super) fn block(&mut self, id: u32, attempt: u32, reason: &str) -> Result<()> {
        if reason.trim().is_empty() || reason.chars().count() > 4000 {
            return Err(failure("A blocker requires 1–4000 characters"));
        }
        self.claimed_task(id, attempt)?.state = TaskState::Blocked {
            reason: reason.trim().into(),
        };
        Ok(())
    }

    pub(super) fn retry(&mut self, id: u32) -> Result<()> {
        let task = self.task_mut(id)?;
        if !matches!(task.state, TaskState::Blocked { .. }) {
            return Err(failure("Only blocked tasks can be retried"));
        }
        task.state = TaskState::Pending;
        Ok(())
    }
}

fn validate_report(report: &Report) -> Result<()> {
    if report.findings.trim().is_empty() || report.findings.len() > 64 * 1024 {
        return Err(failure(
            "Findings require 1–65536 bytes of evidence and interpretation",
        ));
    }
    if report.functions.len() > 16 || report.unresolved.len() > 16 {
        return Err(failure(
            "Record at most 16 functions and 16 unresolved relationships per report",
        ));
    }
    for relationship in &report.unresolved {
        if relationship.question.trim().is_empty()
            || relationship.question.chars().count() > 2000
            || relationship.evidence.trim().is_empty()
            || relationship.evidence.len() > 8000
        {
            return Err(failure(
                "Each unresolved relationship requires a question (1–2000 characters) and supporting evidence (1–8000 bytes)",
            ));
        }
    }
    Ok(())
}
