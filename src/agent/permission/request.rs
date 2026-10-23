use crate::error::Result;
use crate::events::{Event, Sink};
use crate::model::Approval;
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, TryRecvError},
};

pub(super) struct Request {
    pub id: Value,
    params: Value,
    policy: super::Policy,
    prompt: Option<Approval>,
    answer: Option<Receiver<Option<String>>>,
    active: Arc<AtomicBool>,
}

impl Request {
    pub fn new(id: Value, params: &Value, policy: &super::Policy) -> Self {
        let mut prompt = Approval::parse(params);
        let metadata = approval_metadata(params, &mut prompt);
        Self {
            id,
            params: metadata,
            policy: policy.clone(),
            prompt: Some(prompt),
            answer: None,
            active: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn present(&mut self, sink: &Sink) -> Result<()> {
        let Some(request) = self.prompt.take() else {
            return Ok(());
        };
        let (answer, receiver) = mpsc::sync_channel(1);
        match sink.try_send(Event::Permission {
            request,
            answer,
            active: self.active.clone(),
        })? {
            None => self.answer = Some(receiver),
            Some(Event::Permission { request, .. }) => self.prompt = Some(request),
            Some(_) => unreachable!("the permission event is returned unchanged"),
        }
        Ok(())
    }

    pub fn response(&self) -> Option<Value> {
        let answer = match self.answer.as_ref()?.try_recv() {
            Ok(answer) => answer,
            Err(TryRecvError::Disconnected) => None,
            Err(TryRecvError::Empty) => return None,
        };
        let offered = answer.filter(|id| {
            self.params["options"]
                .as_array()
                .is_some_and(|options| options.iter().any(|option| option["optionId"] == *id))
        });
        Some(match offered {
            Some(id) => json!({"outcome":{"outcome":"selected","optionId":id}}),
            None => super::super::permission_policy::cancelled(),
        })
    }

    pub fn dismiss(&self) {
        self.active.store(false, Ordering::Relaxed);
    }

    pub fn remember(&self, response: &Value) -> Result<()> {
        self.policy.save_answer(&self.params, response)
    }
}

fn approval_metadata(params: &Value, prompt: &mut Approval) -> Value {
    let mut options = Vec::new();
    prompt.options.retain(|choice| {
        let option = params["options"]
            .as_array()
            .into_iter()
            .flatten()
            .take(16)
            .find(|option| option["optionId"] == choice.id);
        let Some(kind) = option.and_then(|option| option["kind"].as_str()) else {
            return false;
        };
        if !matches!(
            kind,
            "allow_once" | "allow_always" | "reject_once" | "reject_always"
        ) {
            return false;
        }
        options.push(json!({"optionId":choice.id,"kind":kind}));
        true
    });
    json!({"toolCall":{"title":super::title(params)},"options":options})
}

impl Drop for Request {
    fn drop(&mut self) {
        self.dismiss();
    }
}
