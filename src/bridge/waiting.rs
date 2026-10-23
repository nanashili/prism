use crate::error::{Result, failure};
use crate::events::{Event, Sink};
use crate::worker::RequestLifetime;
use serde_json::Value;
use std::net::TcpStream;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::Duration;

pub(super) fn send(
    sink: &Sink,
    mut event: Event,
    socket: &TcpStream,
    stopped: &AtomicBool,
    lifetime: &RequestLifetime,
) -> Result<()> {
    loop {
        check(socket, stopped, lifetime)?;
        event = match sink.try_send(event)? {
            None => return Ok(()),
            Some(event) => event,
        };
        std::thread::sleep(Duration::from_millis(10));
    }
}

pub(super) fn answer(
    socket: &TcpStream,
    receiver: mpsc::Receiver<Result<Value>>,
    stopped: &AtomicBool,
    lifetime: &RequestLifetime,
) -> Result<Value> {
    loop {
        check(socket, stopped, lifetime)?;
        match receiver.recv_timeout(Duration::from_millis(50)) {
            Ok(result) => return result,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(failure("The chat or request handler closed"));
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    }
}

fn check(socket: &TcpStream, stopped: &AtomicBool, lifetime: &RequestLifetime) -> Result<()> {
    lifetime.check()?;
    if stopped.load(Ordering::Relaxed) {
        return Err(failure("Prism is closing"));
    }
    match socket.peek(&mut [0u8; 1]) {
        Ok(0) => Err(failure("The tool caller disconnected")),
        Ok(_) => Err(failure("Unexpected data after the tool request")),
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::Interrupted => Ok(()),
        Err(error) => Err(error.into()),
    }
}

pub(super) struct RequestGuard(pub(super) RequestLifetime);

impl Drop for RequestGuard {
    fn drop(&mut self) {
        if let Err(error) = self.0.cancel() {
            eprintln!("Cancelling an abandoned tool request failed: {error}");
        }
    }
}
