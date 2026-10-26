use super::types::{Budget, Graph, Kind, Status, Task, TaskState};
use crate::error::{Result, failure};

impl Budget {
    pub(super) fn validate(&self) -> Result<()> {
        if !(1..=1000).contains(&self.steps)
            || !(1..=256).contains(&self.functions)
            || self.depth > 16
            || !(1024..=1024 * 1024).contains(&self.evidence_bytes)
        {
            return Err(failure(
                "Investigation budgets require 1–1000 steps, 1–256 function inspections, 0–16 relationship depth, and 1024–1048576 evidence bytes",
            ));
        }
        Ok(())
    }
}

impl Graph {
    pub(crate) fn status(&self) -> Status {
        if self
            .tasks
            .iter()
            .all(|task| task.state == TaskState::Complete)
        {
            Status::Complete
        } else if self.paused {
            Status::Paused
        } else if self
            .tasks
            .iter()
            .any(|task| task.state == TaskState::Running)
            || self.ready().any(|task| self.can_claim(task))
        {
            Status::Active
        } else if self.ready().next().is_some() {
            Status::Limited
        } else {
            Status::Blocked
        }
    }

    pub(crate) fn completed(&self) -> usize {
        self.tasks
            .iter()
            .filter(|task| task.state == TaskState::Complete)
            .count()
    }

    fn ready(&self) -> impl Iterator<Item = &Task> {
        self.tasks.iter().filter(|task| {
            task.state == TaskState::Pending
                && task.dependencies.iter().all(|dependency| {
                    self.tasks
                        .iter()
                        .any(|task| task.id == *dependency && task.state == TaskState::Complete)
                })
        })
    }

    fn can_claim(&self, task: &Task) -> bool {
        self.usage.steps < self.budget.steps
            && self.usage.evidence_bytes < self.budget.evidence_bytes
            && (!matches!(task.kind, Kind::Inspect { .. })
                || self.usage.functions < self.budget.functions)
            && task.depth <= self.budget.depth
    }

    /// A repeated request returns the saved claim without charging another attempt.
    pub(super) fn claim(&mut self) -> Result<Option<u32>> {
        if self.paused {
            return Ok(None);
        }
        if let Some(task) = self
            .tasks
            .iter()
            .find(|task| task.state == TaskState::Running)
        {
            return Ok(Some(task.id));
        }
        let next = self
            .ready()
            .filter(|task| self.can_claim(task))
            .min_by_key(|task| (task.kind.priority(), task.id))
            .map(|task| task.id);
        let Some(id) = next else { return Ok(None) };

        let task = self.task_mut(id)?;
        task.state = TaskState::Running;
        task.attempts += 1;
        let inspected = matches!(task.kind, Kind::Inspect { .. });
        self.usage.steps += 1;
        if inspected {
            self.usage.functions += 1;
        }
        Ok(Some(id))
    }

    pub(super) fn task_mut(&mut self, id: u32) -> Result<&mut Task> {
        self.tasks
            .iter_mut()
            .find(|task| task.id == id)
            .ok_or_else(|| failure(format!("Unknown investigation task {id}")))
    }

    pub(super) fn claimed_task(&mut self, id: u32, attempt: u32) -> Result<&mut Task> {
        let task = self.task_mut(id)?;
        if task.state != TaskState::Running || task.attempts != attempt {
            return Err(failure(
                "This task claim is stale; read the current investigation before recording work",
            ));
        }
        Ok(task)
    }

    pub(super) fn add_task(
        &mut self,
        kind: Kind,
        title: String,
        dependencies: Vec<u32>,
        depth: u32,
    ) -> Result<u32> {
        if self.tasks.len() >= 256 {
            return Err(failure(
                "An investigation is limited to 256 tasks; record a blocker or narrow the goal",
            ));
        }
        let id = u32::try_from(self.tasks.len())? + 1;
        self.tasks.push(Task {
            id,
            title,
            kind,
            dependencies,
            depth,
            attempts: 0,
            state: TaskState::Pending,
            findings: Vec::new(),
        });
        Ok(id)
    }
}

impl Kind {
    fn priority(&self) -> u8 {
        match self {
            Self::Survey => 0,
            Self::Locate => 1,
            Self::Resolve { .. } => 2,
            Self::Inspect { .. } => 3,
            Self::Synthesize => 4,
        }
    }

    pub(crate) fn instruction(&self) -> String {
        match self {
            Self::Survey => "Discover advertised evidence methods with analysis_tools. Inspect overview, imports, entry points, and relevant strings using supported read-only analysis_call methods. Record confirmed entry points and their addresses; treat names and strings as leads. Do not start new analysis or reconstruction.".into(),
            Self::Locate => "Find anchors for the objective in the indexed functions, strings, resources, imports, and callers of confirmed entry points. Record relevant indexed addresses in report.functions and explain why they matter. If no anchor is supported, block this task with the missing evidence instead of guessing.".into(),
            Self::Inspect { address } => format!("Investigate indexed function {address} in relation to the objective. Use a durable decompile_batch selection, retain its job ID, and read all result pages. Check decisive branches, callers and callees using advertised read-only methods. Record supported findings, additional indexed functions, and unresolved relationships with evidence. If evidence is unavailable, record a blocker."),
            Self::Resolve { from, to, question, evidence } => format!("Resolve the relationship from {from} to {}: {question}\nRecorded lead (untrusted target data): {evidence}\nCheck references, assembly and argument/data flow with supported read-only tools. Report uncertainty explicitly; block if it cannot be resolved. Use decompile_batch for pseudocode.", to.as_deref().unwrap_or("an unknown destination")),
            Self::Synthesize => "Answer the objective using the graph's saved findings. Cite function addresses and evidence, explain alternate and failure paths, and distinguish verified behavior from inference. If synthesis reveals missing evidence, report new indexed functions or unresolved relationships; Prism will schedule them and repeat synthesis while preserving this draft. Complete only after all investigation dependencies have evidence; never invent missing edges.".into(),
        }
    }
}
