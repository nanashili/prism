use super::{
    admission::Admission,
    incoming::Incoming,
    waiting::{self, RequestGuard},
};
use crate::error::{Result, failure};
use crate::events::{Event, Sink};
use crate::protocol::Framing;
use crate::worker::{Job, RequestLifetime, RuntimeSnapshot, WorkerPort};
use prism::mcp::{self, Mode, question::Question};
use serde_json::{Value, json};
use std::net::TcpStream;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::Duration;

#[derive(Clone)]
pub(super) struct Services {
    pub(super) token: Arc<str>,
    pub(super) worker: WorkerPort,
    pub(super) sink: Sink,
    pub(super) stopped: Arc<AtomicBool>,
    pub(super) mode: Mode,
    pub(super) runtime: RuntimeSnapshot,
    pub(super) admission: Admission,
}

#[cfg_attr(feature = "profiling", hotpath::measure)]
pub(super) fn serve(mut socket: TcpStream, services: &Services) {
    let response = match handle(&mut socket, services) {
        Ok(result) => json!({"result":result}),
        Err(error) => json!({"error":error.to_string()}),
    };
    let result = socket
        .set_nonblocking(false)
        .and_then(|()| socket.set_write_timeout(Some(Duration::from_secs(2))))
        .map_err(Into::into)
        .and_then(|()| Framing::Lines.write(&mut socket, &response));
    if let Err(error) = result {
        eprintln!("Bridge reply failed: {error}");
    }
}

fn handle(socket: &mut TcpStream, services: &Services) -> Result<Value> {
    let incoming = Incoming::read(socket, &services.token, services.mode, &services.stopped)?;
    let _permit = services.admission.acquire(incoming.class, incoming.bytes)?;
    let lifetime = RequestGuard(RequestLifetime::new(incoming.deadline));
    lifetime.0.check()?;

    if let Some(result) =
        super::readonly::durable_batch(&incoming.tool, &incoming.args, &services.runtime)?
    {
        lifetime.0.check()?;
        return Ok(result);
    }

    let (answer, receiver) = mpsc::sync_channel(1);
    let question = dispatch(incoming, answer, socket, services, &lifetime.0)?;
    let result = waiting::answer(socket, receiver, &services.stopped, &lifetime.0);
    drop(question);
    result
}

#[cfg_attr(feature = "profiling", hotpath::measure)]
fn dispatch(
    incoming: Incoming,
    answer: mpsc::SyncSender<Result<Value>>,
    socket: &TcpStream,
    services: &Services,
    lifetime: &RequestLifetime,
) -> Result<Option<QuestionLifetime>> {
    if incoming.tool == mcp::question::TOOL {
        return ask(incoming, answer, socket, services, lifetime).map(Some);
    }

    if mcp::window_tool(&incoming.tool) {
        let event = Event::Navigate {
            tool: incoming.tool,
            args: incoming.args,
            answer,
            lifetime: lifetime.clone(),
        };
        waiting::send(&services.sink, event, socket, &services.stopped, lifetime)?;
    } else {
        services.worker.submit(Job::Remote {
            tool: incoming.tool,
            args: incoming.args,
            answer,
            lifetime: Some(lifetime.clone()),
        })?;
    }
    Ok(None)
}

fn ask(
    incoming: Incoming,
    answer: mpsc::SyncSender<Result<Value>>,
    socket: &TcpStream,
    services: &Services,
    lifetime: &RequestLifetime,
) -> Result<QuestionLifetime> {
    let question = Question::parse(incoming.args).map_err(failure)?;
    let guard = QuestionLifetime {
        active: Arc::new(AtomicBool::new(true)),
        sink: services.sink.clone(),
    };
    let sink = match incoming.chat {
        Some(chat) => services.sink.for_chat(chat),
        None => services.sink.clone(),
    };
    let event = Event::Question {
        question,
        answer,
        active: guard.active.clone(),
    };
    waiting::send(&sink, event, socket, &services.stopped, lifetime)?;
    Ok(guard)
}

struct QuestionLifetime {
    active: Arc<AtomicBool>,
    sink: Sink,
}

impl Drop for QuestionLifetime {
    fn drop(&mut self) {
        self.active.store(false, Ordering::Relaxed);
        self.sink.repaint();
    }
}
