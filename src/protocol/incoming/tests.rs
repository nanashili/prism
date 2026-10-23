use super::*;
use crate::protocol::Framing;
use std::io::{BufReader, Cursor};
use std::time::Instant;

fn frame(length: usize) -> Frame {
    let mut bytes = vec![b'x'; length.max(3)];
    bytes[0] = b'"';
    let last = bytes.len() - 1;
    bytes[last - 1] = b'"';
    bytes[last] = b'\n';
    Framing::Lines
        .read(&mut Cursor::new(bytes))
        .unwrap()
        .unwrap()
}

#[test]
fn dequeue_releases_byte_budget_and_preserves_order() {
    let (sender, receiver) = channel();
    sender.send(Ok(frame(MAX_QUEUED_BYTES))).unwrap();
    let (entered, started) = mpsc::channel();
    let (finished, done) = mpsc::channel();
    let producer = std::thread::spawn(move || {
        entered.send(()).unwrap();
        sender.send(Ok(frame(4096))).unwrap();
        finished.send(()).unwrap();
    });
    started.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(done.recv_timeout(Duration::from_millis(20)).is_err());
    assert_eq!(
        receiver
            .poll(Duration::from_secs(5))
            .unwrap()
            .unwrap()
            .as_str()
            .unwrap()
            .len(),
        MAX_QUEUED_BYTES - 3
    );
    done.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(
        receiver
            .poll(Duration::from_secs(5))
            .unwrap()
            .unwrap()
            .as_str()
            .unwrap()
            .len(),
        4093
    );
    producer.join().unwrap();
}

#[test]
fn dropping_receiver_unblocks_byte_and_message_limits() {
    for byte_limit in [false, true] {
        let (sender, receiver) = channel();
        for _ in 0..if byte_limit { 1 } else { MAX_MESSAGES } {
            sender
                .send(Ok(frame(if byte_limit { MAX_QUEUED_BYTES } else { 64 })))
                .unwrap();
        }
        let budget = sender.budget.clone();
        let (entered, started) = mpsc::channel();
        let (finished, done) = mpsc::channel();
        let producer = std::thread::spawn(move || {
            entered.send(()).unwrap();
            finished.send(sender.send(Ok(frame(64))).is_err()).unwrap();
        });
        started.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(done.recv_timeout(Duration::from_millis(20)).is_err());
        drop(receiver);
        assert!(done.recv_timeout(Duration::from_secs(5)).unwrap());
        producer.join().unwrap();
        let state = budget.state.lock().unwrap();
        assert!(state.closed);
        assert_eq!(state.bytes, 0);
    }
}

#[test]
fn oversized_protocol_frames_are_admitted_alone() {
    let (sender, receiver) = channel();
    let mut bytes = vec![b'x'; 32 * 1024 * 1024];
    *bytes.last_mut().unwrap() = 0;
    let large = Framing::Zero
        .read(&mut Cursor::new(bytes))
        .unwrap()
        .unwrap();
    let capacity = large.capacity();
    sender.send(Ok(large)).unwrap();
    assert_eq!(sender.budget.state.lock().unwrap().bytes, capacity);
    assert_eq!(
        receiver
            .poll(Duration::ZERO)
            .unwrap()
            .unwrap()
            .as_str()
            .unwrap()
            .len(),
        32 * 1024 * 1024 - 1
    );
    assert_eq!(sender.budget.state.lock().unwrap().bytes, 0);
}

#[test]
fn malformed_json_and_reader_errors_remain_fatal_when_polled() {
    let (sender, receiver) = channel();
    sender
        .send(Ok(Framing::Lines
            .read(&mut Cursor::new(b"{bad}\n"))
            .unwrap()
            .unwrap()))
        .unwrap();
    sender.send(Ok(frame(64))).unwrap();
    assert!(
        receiver
            .poll(Duration::ZERO)
            .unwrap_err()
            .to_string()
            .contains("key must be a string")
    );
    assert_eq!(
        receiver.poll(Duration::ZERO).unwrap_err().to_string(),
        "The server process closed its output"
    );
    assert!(sender.send(Ok(frame(64))).is_err());
    assert_eq!(sender.budget.state.lock().unwrap().bytes, 0);

    let (sender, receiver) = channel();
    assert!(receiver.poll(Duration::ZERO).unwrap().is_none());
    sender.send(Err(failure("fixture read error"))).unwrap();
    assert_eq!(
        receiver.poll(Duration::ZERO).unwrap_err().to_string(),
        "fixture read error"
    );
    assert!(sender.send(Ok(frame(64))).is_err());
}

#[test]
#[ignore = "fresh-process RAM benchmark; set PRISM_MEMORY_IMPL=baseline|candidate and PRISM_MEMORY_FIXTURE to a file of 32 JSON array lines"]
#[allow(clippy::assertions_on_constants)]
fn memory_rpc_queue() {
    assert!(
        !cfg!(debug_assertions),
        "run memory benchmarks with --release"
    );
    let implementation = std::env::var("PRISM_MEMORY_IMPL").unwrap();
    assert!(matches!(implementation.as_str(), "baseline" | "candidate"));
    let path = std::env::var_os("PRISM_MEMORY_FIXTURE").unwrap();
    let input = std::fs::File::open(path).unwrap();
    let mut input = BufReader::new(input);
    let first = Framing::Lines.read(&mut input).unwrap().unwrap();
    let capacity = first.capacity();
    let queue_count = (MAX_QUEUED_BYTES / capacity).clamp(1, MAX_MESSAGES);
    let (ready, saturated) = mpsc::sync_channel(0);
    let started = Instant::now();
    let mut count = 0;
    let mut elements = 0;
    if implementation == "baseline" {
        let (sender, receiver) = mpsc::sync_channel(MAX_MESSAGES);
        let producer = std::thread::spawn(move || {
            let mut next = Some(first);
            let mut read = 0;
            loop {
                let Some(frame) = next
                    .take()
                    .or_else(|| Framing::Lines.read(&mut input).unwrap())
                else {
                    break;
                };
                let value = frame.decode().unwrap();
                read += 1;
                if read == MAX_MESSAGES + 1 {
                    ready.send(()).unwrap();
                }
                sender.send(value).unwrap();
            }
        });
        saturated.recv_timeout(Duration::from_secs(30)).unwrap();
        while let Ok(value) = receiver.recv() {
            let array = value.as_array().unwrap();
            assert!(array.iter().all(|item| item == 0));
            elements += array.len();
            count += 1;
        }
        producer.join().unwrap();
    } else {
        let (sender, receiver) = channel();
        let producer = std::thread::spawn(move || {
            let mut next = Some(first);
            let mut read = 0;
            loop {
                let Some(frame) = next
                    .take()
                    .or_else(|| Framing::Lines.read(&mut input).unwrap())
                else {
                    break;
                };
                read += 1;
                if read == queue_count + 1 {
                    ready.send(()).unwrap();
                }
                sender.send(Ok(frame)).unwrap();
            }
        });
        saturated.recv_timeout(Duration::from_secs(30)).unwrap();
        loop {
            match receiver.poll(Duration::from_secs(30)) {
                Ok(Some(value)) => {
                    let array = value.as_array().unwrap();
                    assert!(array.iter().all(|item| item == 0));
                    elements += array.len();
                    count += 1;
                }
                Err(error) if error.to_string() == "The server process closed its output" => break,
                other => panic!("unexpected queue result: {other:?}"),
            }
        }
        producer.join().unwrap();
    }
    assert_eq!(count, 32);
    assert_eq!(elements, 32 * 524_288);
    eprintln!(
        "memory_rpc_queue impl={implementation} frames={count} elements={elements} frame_capacity={capacity} queue_slots={queue_count} elapsed_ms={:.3}",
        started.elapsed().as_secs_f64() * 1000.0
    );
}
