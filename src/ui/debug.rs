use super::app::Prism;
use crate::worker::ViewJob;
use eframe::egui;
use serde_json::json;

#[derive(Default)]
pub(super) struct DebugForm {
    pub args: String,
    pub pid: u32,
    pub breakpoint: String,
    pub expression: String,
    pub thread: u64,
}

impl Prism {
    pub(super) fn debug_screen(&mut self, ui: &mut egui::Ui) {
        ui.heading("Debugger");
        ui.label("LLDB runs the selected program when you choose Launch.");
        ui.horizontal(|ui| {
            ui.label("Arguments");
            ui.text_edit_singleline(&mut self.debug.args);
            if ui.button("Launch").clicked() {
                let path = self
                    .workspace
                    .project
                    .as_ref()
                    .map(|project| project.target.path.clone());
                if let Some(path) = path {
                    self.enqueue_view(ViewJob::Debug(crate::debug::Action::Launch {
                        path,
                        args: self.debug.args.clone(),
                    }));
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label("Process ID");
            ui.add(egui::DragValue::new(&mut self.debug.pid));
            if ui.button("Attach").clicked() {
                self.enqueue_view(ViewJob::Debug(crate::debug::Action::Attach(self.debug.pid)));
            }
            if ui.button("Disconnect").clicked() {
                self.enqueue_view(ViewJob::Debug(crate::debug::Action::Stop));
            }
        });
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut self.debug.breakpoint);
            if ui.button("Set function breakpoint").clicked() {
                self.debug_request(
                    "setFunctionBreakpoints",
                    json!({"breakpoints":[{"name":self.debug.breakpoint}]}),
                );
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("Thread");
            ui.add(egui::DragValue::new(&mut self.debug.thread).range(1..=u64::MAX));
            for (label, command) in [
                ("Continue", "continue"),
                ("Step over", "next"),
                ("Step in", "stepIn"),
                ("Step out", "stepOut"),
                ("Pause", "pause"),
                ("Stack", "stackTrace"),
            ] {
                if ui.button(label).clicked() {
                    self.debug_request(command, json!({"threadId":self.debug.thread}));
                }
            }
            if ui.button("Threads").clicked() {
                self.debug_request("threads", json!({}));
            }
        });
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut self.debug.expression);
            if ui.button("Evaluate").clicked() {
                self.debug_request(
                    "evaluate",
                    json!({"expression":self.debug.expression,"context":"repl"}),
                );
            }
        });
        self.code(ui);
    }

    fn debug_request(&mut self, command: &str, args: serde_json::Value) {
        self.enqueue_view(ViewJob::Debug(crate::debug::Action::Request {
            command: command.into(),
            args,
        }));
    }
}
