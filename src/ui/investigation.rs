use super::app::Prism;
use crate::error::Result;
use crate::investigation::{Budget, Graph, Status, Task, TaskState};
use eframe::egui::{self, RichText};
use serde_json::json;

#[derive(Default)]
pub(super) struct Panel {
    pub(super) graph: Option<Graph>,
    pub(super) objective: String,
    budget: Budget,
}

#[derive(Clone, Copy)]
enum Action {
    Start,
    Continue,
    Pause,
    Budget,
    Retry(u32),
}

impl Panel {
    fn show(&mut self, ui: &mut egui::Ui, can_launch: bool) -> Option<Action> {
        let title = self.graph.as_ref().map_or_else(
            || "Investigate a goal".into(),
            |graph| {
                format!(
                    "Investigation · {} · {}/{}",
                    label(graph.status()),
                    graph.completed(),
                    graph.tasks.len()
                )
            },
        );
        let mut action = None;
        egui::CollapsingHeader::new(title)
            .id_salt("prism-investigation")
            .show(ui, |ui| {
                if let Some(graph) = &self.graph {
                    action = details(ui, graph);
                    if budget_controls(ui, &mut self.budget) {
                        action = Some(Action::Budget);
                    }
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .add_enabled(
                                can_launch
                                    && matches!(graph.status(), Status::Active | Status::Paused),
                                egui::Button::new("Continue investigation"),
                            )
                            .clicked()
                        {
                            action = Some(Action::Continue);
                        }
                        if !graph.paused
                            && graph.status() != Status::Complete
                            && ui.small_button("Pause").clicked()
                        {
                            action = Some(Action::Pause);
                        }
                    });
                }
                ui.add_space(4.0);
                ui.add(
                    egui::TextEdit::singleline(&mut self.objective)
                        .hint_text(
                            "Understand how this application loads, validates, and saves projects",
                        )
                        .char_limit(4000)
                        .desired_width(f32::INFINITY),
                );
                if ui
                    .add_enabled(
                        can_launch && !self.objective.trim().is_empty(),
                        egui::Button::new("Start investigation"),
                    )
                    .clicked()
                {
                    action = Some(Action::Start);
                }
            });
        action
    }
}

impl Prism {
    pub(super) fn refresh_investigation(&mut self, id: Option<&str>) -> Result<()> {
        let graph = self
            .workspace
            .project
            .as_ref()
            .map(|project| match id {
                Some(id) => self
                    .store
                    .investigation(&project.target.sha256, id)
                    .map(Some),
                None => self.store.latest_investigation(&project.target.sha256),
            })
            .transpose()?
            .flatten();
        self.investigation.budget = graph
            .as_ref()
            .map_or_else(Budget::default, |graph| graph.budget.clone());
        self.investigation.graph = graph;
        Ok(())
    }

    pub(super) fn investigation_panel(&mut self, ui: &mut egui::Ui) {
        if self.workspace.project.is_none() {
            return;
        }
        let can_launch = !self.chat.busy
            && self.pending == 0
            && self.chat.draft.trim().is_empty()
            && self.chat.attachments.is_empty()
            && self.chat.queue.is_empty()
            && self.settings.conversation.rea_tools;
        if let Some(action) = self.investigation.show(ui, can_launch) {
            let result = self.update_investigation(action).and_then(|()| {
                if matches!(action, Action::Continue) {
                    self.launch_investigation()
                } else {
                    Ok(())
                }
            });
            if let Err(error) = result {
                self.report_error(error);
            }
        }
    }

    fn update_investigation(&mut self, action: Action) -> Result<()> {
        let project = &self
            .workspace
            .project
            .as_ref()
            .ok_or("Open a project first")?
            .target
            .sha256;
        if matches!(action, Action::Start) {
            return self.submit(crate::worker::Job::StartInvestigation {
                project: project.clone(),
                objective: self.investigation.objective.clone(),
                budget: self.investigation.budget.clone(),
            });
        }

        let id = &self
            .investigation
            .graph
            .as_ref()
            .ok_or("No saved investigation")?
            .id;
        let arguments = match action {
            Action::Continue => json!({"action":"resume","investigation":id}),
            Action::Pause => json!({"action":"pause","investigation":id}),
            Action::Budget => {
                json!({"action":"budget","investigation":id,"budget":self.investigation.budget})
            }
            Action::Retry(task) => json!({"action":"retry","investigation":id,"task":task}),
            Action::Start => return Err("A new investigation must have an objective".into()),
        };
        crate::investigation::call(&self.store, project, &arguments)?;
        self.investigation.graph = Some(self.store.investigation(project, id)?);
        Ok(())
    }

    fn launch_investigation(&mut self) -> Result<()> {
        if self.chat.busy
            || !self.chat.draft.trim().is_empty()
            || !self.chat.attachments.is_empty()
            || !self.chat.queue.is_empty()
        {
            return Err("Investigation saved. Finish the current chat or clear the composer before continuing it.".into());
        }
        let graph = self
            .investigation
            .graph
            .as_ref()
            .ok_or("No saved investigation")?;
        self.chat.draft = format!(
            "Continue Prism investigation {} for the open project. Read investigation status, then repeatedly request next, perform the selected step, and record evidence or blockers until the graph is complete, paused, blocked, or limited. Recover existing decompile_batch jobs rather than repeating work. Objective: {}",
            graph.id, graph.objective,
        );
        self.prepare_prompt()
    }

    pub(super) fn receive_investigation(&mut self, graph: Graph) {
        if self
            .workspace
            .project
            .as_ref()
            .is_none_or(|project| project.target.sha256 != graph.project)
        {
            return;
        }
        if self.investigation.objective.trim() == graph.objective {
            self.investigation.objective.clear();
        }
        self.investigation.budget = graph.budget.clone();
        self.investigation.graph = Some(graph);
        if let Err(error) = self.launch_investigation() {
            self.report_error(error);
        }
    }
}

fn details(ui: &mut egui::Ui, graph: &Graph) -> Option<Action> {
    let mut action = None;
    ui.label(RichText::new(&graph.objective).strong());
    ui.weak(format!(
        "Steps {}/{} · function inspections {}/{} · evidence {} / {} KiB · depth {}",
        graph.usage.steps,
        graph.budget.steps,
        graph.usage.functions,
        graph.budget.functions,
        graph.usage.evidence_bytes / 1024,
        graph.budget.evidence_bytes / 1024,
        graph.budget.depth,
    ));
    if graph.status() == Status::Limited {
        ui.label("A planning budget is exhausted. Review the saved graph and adjust its budget explicitly to continue.");
    }
    egui::ScrollArea::vertical()
        .id_salt(("investigation-tasks", &graph.id))
        .max_height(210.0)
        .show(ui, |ui| {
            for task in &graph.tasks {
                if task_row(ui, task) {
                    action = Some(Action::Retry(task.id));
                }
            }
        });
    ui.weak("Saved locally with this target. Agent sessions can resume this graph.");
    action
}

fn task_row(ui: &mut egui::Ui, task: &Task) -> bool {
    let mut retry = false;
    let state = match &task.state {
        TaskState::Pending => "Pending",
        TaskState::Running => "In progress",
        TaskState::Complete => "Complete",
        TaskState::Blocked { .. } => "Blocked",
    };
    egui::CollapsingHeader::new(format!("{} · {state} · {}", task.id, task.title))
        .id_salt(task.id)
        .show(ui, |ui| {
            ui.weak(format!(
                "Depends on {:?} · attempts {} · depth {}",
                task.dependencies, task.attempts, task.depth
            ));
            if let TaskState::Blocked { reason } = &task.state {
                ui.label(reason);
                retry = ui.small_button("Retry after resolving blocker").clicked();
            }
            for finding in &task.findings {
                ui.weak(format!("Attempt {}", finding.attempt));
                ui.add(egui::Label::new(&finding.text).selectable(true).wrap());
            }
            if task.findings.is_empty() {
                ui.label(task.kind.instruction());
            }
        });
    retry
}

fn budget_controls(ui: &mut egui::Ui, budget: &mut Budget) -> bool {
    egui::CollapsingHeader::new("Resource budgets")
        .show(ui, |ui| {
            egui::Grid::new("investigation-budget").show(ui, |ui| {
                ui.label("Steps");
                ui.add(egui::DragValue::new(&mut budget.steps).range(1..=1000));
                ui.end_row();
                ui.label("Function inspections");
                ui.add(egui::DragValue::new(&mut budget.functions).range(1..=256));
                ui.end_row();
                ui.label("Relationship depth");
                ui.add(egui::DragValue::new(&mut budget.depth).range(0..=16));
                ui.end_row();
                ui.label("Evidence bytes");
                ui.add(egui::DragValue::new(&mut budget.evidence_bytes).range(1024..=1048576));
                ui.end_row();
            });
            ui.small_button("Apply budgets").clicked()
        })
        .body_returned
        .unwrap_or(false)
}

fn label(status: Status) -> &'static str {
    match status {
        Status::Active => "Active",
        Status::Paused => "Paused",
        Status::Blocked => "Blocked",
        Status::Limited => "Budget reached",
        Status::Complete => "Complete",
    }
}
