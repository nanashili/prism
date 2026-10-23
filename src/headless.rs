use crate::bridge::Bridge;
use crate::error::{Result, failure};
use crate::events::{Event, Sink};
use crate::settings::Settings;
use crate::worker::{Job, Worker};
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::thread::JoinHandle;

pub(crate) fn run(path: PathBuf) -> Result<()> {
    let _data_lock = crate::migration::lock_data(&crate::settings::support_dir())?;
    let settings = Settings::load()?;
    let (sink, events) = Sink::channel(&eframe::egui::Context::default());
    let worker = Worker::start(settings, sink.clone());
    worker.submit(Job::Open(path))?;
    let events = wait_for_open(events)?;

    let bridge = Bridge::start_headless(worker.port()?, sink.clone(), worker.runtime())?;
    let (address, token) = bridge.endpoint();
    let arguments = [
        OsString::from("--headless"),
        OsString::from(address),
        OsString::from(token),
    ];
    let drain = drain_events(events)?;
    let result = prism::mcp::run_cli(arguments.into_iter());

    drop(bridge);
    drop(worker);
    drop(sink);
    let drain_result = drain
        .join()
        .map_err(|_| failure("The headless event drain stopped unexpectedly"));
    match (result, drain_result) {
        (Err(error), Err(cleanup)) => Err(failure(format!("{error}; event drain: {cleanup}"))),
        (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
        (Ok(()), Ok(())) => Ok(()),
    }
}

fn wait_for_open(events: Receiver<Event>) -> Result<Receiver<Event>> {
    loop {
        match events.recv() {
            Ok(Event::Finished(result)) => {
                result.map(|_| ())?;
                return Ok(events);
            }
            Ok(event) => reject_ui_event(event),
            Err(_) => {
                return Err(failure(
                    "The headless worker closed while opening the project",
                ));
            }
        }
    }
}

fn drain_events(events: Receiver<Event>) -> Result<JoinHandle<()>> {
    std::thread::Builder::new()
        .name("prism-headless-events".into())
        .spawn(move || {
            while let Ok(event) = events.recv() {
                reject_ui_event(event);
            }
        })
        .map_err(Into::into)
}

fn reject_ui_event(event: Event) {
    match event {
        Event::Question { answer, .. } | Event::Navigate { answer, .. } => {
            let _ = answer.send(Err(failure(
                "User interaction is unavailable in headless mode",
            )));
        }
        Event::Permission { answer, .. } => {
            let _ = answer.send(None);
        }
        _ => {}
    }
}
