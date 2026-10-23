mod card;
mod rows;

use crate::error::Result;
use crate::ui::app::Prism;
use eframe::egui;
use prism::mcp::question::{Answer, Question};
use serde_json::Value;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::SyncSender,
};

#[derive(Default)]
pub(in crate::ui) struct Questions {
    pending: Option<Pending>,
    dismissed: Option<egui::Id>,
}

struct Pending {
    question: Question,
    draft: String,
    source: Source,
}

enum Source {
    Agent {
        answer: SyncSender<Result<Value>>,
        active: Arc<AtomicBool>,
    },
    Reply(egui::Id),
}

impl Questions {
    pub(in crate::ui) fn is_reply(&self) -> bool {
        self.pending
            .as_ref()
            .is_some_and(|pending| matches!(pending.source, Source::Reply(_)))
    }

    pub(in crate::ui) fn is_pending(&self) -> bool {
        self.pending
            .as_ref()
            .is_some_and(|pending| match &pending.source {
                Source::Agent { active, .. } => active.load(Ordering::Relaxed),
                Source::Reply(_) => true,
            })
    }

    fn awaiting_agent(&self) -> bool {
        self.pending.as_ref().is_some_and(|pending| {
            matches!(&pending.source, Source::Agent { active, .. } if active.load(Ordering::Relaxed))
        })
    }

    pub(in crate::ui) fn clear_agent_question(&mut self) {
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| matches!(pending.source, Source::Agent { .. }))
        {
            self.pending = None;
        }
    }
}

impl Prism {
    pub(in crate::ui) fn receive_question(
        &mut self,
        question: Question,
        answer: SyncSender<Result<Value>>,
        active: Arc<AtomicBool>,
    ) {
        if !active.load(Ordering::Relaxed) {
            return;
        }
        if self.chat.questions.awaiting_agent() {
            if answer
                .send(Err(
                    "This chat already has a question waiting for an answer".into(),
                ))
                .is_err()
            {
                self.status = "The question caller disconnected".into();
            }
            return;
        }
        if self.settings.conversation.notify_when_asked {
            self.notify_permission(&question.title);
        }
        self.chat.questions.pending = Some(Pending {
            question,
            draft: String::new(),
            source: Source::Agent { answer, active },
        });
    }

    pub(in crate::ui) fn question_card(&mut self, ui: &mut egui::Ui) {
        self.sync_question();
        let Some(pending) = &mut self.chat.questions.pending else {
            return;
        };
        let answer = ui
            .push_id(("user-question", self.chat_tabs.active), |ui| {
                card::show(ui, &pending.question, &mut pending.draft)
            })
            .inner;
        if let Some(answer) = answer {
            self.answer_question(answer);
        }
    }

    fn sync_question(&mut self) {
        if self.chat.questions.awaiting_agent() {
            return;
        }
        if !self.can_send_reply_choice() {
            self.chat.questions.pending = None;
            return;
        }
        let Some(message) = self.chat.messages.last() else {
            return;
        };
        if message.text.len() > crate::ui::message::choices::MAX_REPLY_BYTES {
            self.chat.questions.pending = None;
            return;
        }
        let identity = egui::Id::new((self.chat_tabs.active, message.id, message.text.as_str()));
        if self
            .chat
            .questions
            .pending
            .as_ref()
            .is_some_and(|pending| matches!(pending.source, Source::Reply(key) if key == identity))
        {
            return;
        }
        self.chat.questions.pending = None;
        if self.chat.questions.dismissed == Some(identity) {
            return;
        }
        let Some(choices) = crate::ui::message::choices::Choices::parse(&message.text) else {
            return;
        };
        self.chat.questions.pending = Some(Pending {
            question: Question {
                title: "Question".into(),
                question: choices
                    .intro
                    .lines()
                    .rev()
                    .find(|line| !line.trim().is_empty())
                    .unwrap_or_default()
                    .trim_matches(['#', '*', '_', '`', ' '])
                    .into(),
                options: choices.options,
            },
            draft: String::new(),
            source: Source::Reply(identity),
        });
    }

    fn answer_question(&mut self, response: Answer) {
        let Some(pending) = self.chat.questions.pending.take() else {
            return;
        };
        match pending.source {
            Source::Agent { answer, active } => {
                if active.load(Ordering::Relaxed) {
                    let result = serde_json::to_value(response).map_err(Into::into);
                    if answer.send(result).is_err() {
                        self.report_error("The agent stopped before your answer arrived");
                    }
                }
            }
            Source::Reply(identity) => {
                self.chat.questions.dismissed = Some(identity);
                if let Answer::Answered { answer, .. } = response {
                    self.send_reply_choice(answer);
                }
            }
        }
    }
}
