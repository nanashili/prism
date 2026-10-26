use crate::events::Sink;
use crate::protocol::Framing;
use crate::worker::RuntimeSnapshot;
use crate::worker::WorkerPort;
use prism::mcp::Mode;
use serde_json::json;
use std::net::{TcpListener, TcpStream};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread::JoinHandle;
use std::time::Duration;

const MAX_CONNECTIONS: usize = 20;

pub(super) fn run(
    listener: TcpListener,
    token: String,
    worker: WorkerPort,
    sink: Sink,
    stopped: Arc<AtomicBool>,
    mode: Mode,
    runtime: RuntimeSnapshot,
) {
    let services = super::request::Services {
        token: token.into(),
        worker,
        sink,
        stopped: stopped.clone(),
        mode,
        runtime,
        admission: super::admission::Admission::default(),
    };
    let mut requests = Vec::new();
    while !stopped.load(Ordering::Relaxed) {
        reap_finished(&mut requests);
        let socket = match listener.accept() {
            Ok((socket, _)) => socket,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(25));
                continue;
            }
            Err(error) => {
                eprintln!("Bridge listener failed: {error}");
                break;
            }
        };
        if requests.len() >= MAX_CONNECTIONS {
            reject_busy(socket);
            continue;
        }

        let services = services.clone();
        let request = std::thread::Builder::new()
            .name("prism-window-request".into())
            .stack_size(512 * 1024)
            .spawn(move || super::request::serve(socket, &services));
        match request {
            Ok(request) => requests.push(request),
            Err(error) => eprintln!("Starting bridge request failed: {error}"),
        }
    }
    stopped.store(true, Ordering::Relaxed);
    for request in requests {
        join(request);
    }
}

fn reap_finished(requests: &mut Vec<JoinHandle<()>>) {
    let mut index = 0;
    while index < requests.len() {
        if requests[index].is_finished() {
            join(requests.swap_remove(index));
        } else {
            index += 1;
        }
    }
}

fn join(request: JoinHandle<()>) {
    if request.join().is_err() {
        eprintln!("A bridge request stopped unexpectedly");
    }
}

fn reject_busy(mut socket: TcpStream) {
    let response = json!({"error":"Prism has too many pending tool requests. Try again shortly."});
    let result = socket
        .set_write_timeout(Some(Duration::from_secs(2)))
        .and_then(|()| {
            Framing::Lines
                .write(&mut socket, &response)
                .map_err(std::io::Error::other)
        });
    if let Err(error) = result {
        eprintln!("Rejecting a busy bridge request failed: {error}");
    }
}
