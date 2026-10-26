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
        .expect("frame: valid fixture operation")
        .expect("frame: valid fixture operation")
}

#[test]
fn dequeue_releases_byte_budget_and_preserves_order() {
    let (sender, receiver) = channel();
    sender
        .send(Ok(frame(MAX_QUEUED_BYTES)))
        .expect("dequeue releases byte budget and preserves order: valid fixture operation");
    let (entered, started) = mpsc::channel();
    let (finished, done) = mpsc::channel();
    let producer = std::thread::spawn(move || {
        entered
            .send(())
            .expect("dequeue releases byte budget and preserves order: valid fixture operation");
        sender
            .send(Ok(frame(4096)))
            .expect("dequeue releases byte budget and preserves order: valid fixture operation");
        finished
            .send(())
            .expect("dequeue releases byte budget and preserves order: valid fixture operation");
    });
    started
        .recv_timeout(Duration::from_secs(5))
        .expect("dequeue releases byte budget and preserves order: valid fixture operation");
    assert!(done.recv_timeout(Duration::from_millis(20)).is_err());
    assert_eq!(
        receiver
            .poll(Duration::from_secs(5))
            .expect("dequeue releases byte budget and preserves order: valid fixture operation")
            .expect("dequeue releases byte budget and preserves order: valid fixture operation")
            .as_str()
            .expect("dequeue releases byte budget and preserves order: valid fixture operation")
            .len(),
        MAX_QUEUED_BYTES - 3
    );
    done.recv_timeout(Duration::from_secs(5))
        .expect("dequeue releases byte budget and preserves order: valid fixture operation");
    assert_eq!(
        receiver
            .poll(Duration::from_secs(5))
            .expect("dequeue releases byte budget and preserves order: valid fixture operation")
            .expect("dequeue releases byte budget and preserves order: valid fixture operation")
            .as_str()
            .expect("dequeue releases byte budget and preserves order: valid fixture operation")
            .len(),
        4093
    );
    producer
        .join()
        .expect("dequeue releases byte budget and preserves order: valid fixture operation");
}

#[test]
fn dropping_receiver_unblocks_byte_and_message_limits() {
    for byte_limit in [false, true] {
        let (sender, receiver) = channel();
        for _ in 0..if byte_limit { 1 } else { MAX_MESSAGES } {
            sender
                .send(Ok(frame(if byte_limit { MAX_QUEUED_BYTES } else { 64 })))
                .expect(
                    "dropping receiver unblocks byte and message limits: valid fixture operation",
                );
        }
        let budget = sender.budget.clone();
        let (entered, started) = mpsc::channel();
        let (finished, done) = mpsc::channel();
        let producer = std::thread::spawn(move || {
            entered.send(()).expect(
                "dropping receiver unblocks byte and message limits: valid fixture operation",
            );
            finished.send(sender.send(Ok(frame(64))).is_err()).expect(
                "dropping receiver unblocks byte and message limits: valid fixture operation",
            );
        });
        started
            .recv_timeout(Duration::from_secs(5))
            .expect("dropping receiver unblocks byte and message limits: valid fixture operation");
        assert!(done.recv_timeout(Duration::from_millis(20)).is_err());
        drop(receiver);
        assert!(
            done.recv_timeout(Duration::from_secs(5)).expect(
                "dropping receiver unblocks byte and message limits: valid fixture operation"
            )
        );
        producer
            .join()
            .expect("dropping receiver unblocks byte and message limits: valid fixture operation");
        let state = budget
            .state
            .lock()
            .expect("dropping receiver unblocks byte and message limits: valid fixture operation");
        assert!(state.closed);
        assert_eq!(state.bytes, 0);
    }
}

#[test]
fn oversized_protocol_frames_are_admitted_alone() {
    let (sender, receiver) = channel();
    let mut bytes = vec![b'x'; 32 * 1024 * 1024];
    *bytes
        .last_mut()
        .expect("oversized protocol frames are admitted alone: valid fixture operation") = 0;
    let large = Framing::Zero
        .read(&mut Cursor::new(bytes))
        .expect("oversized protocol frames are admitted alone: valid fixture operation")
        .expect("oversized protocol frames are admitted alone: valid fixture operation");
    let capacity = large.capacity();
    sender
        .send(Ok(large))
        .expect("oversized protocol frames are admitted alone: valid fixture operation");
    assert_eq!(
        sender
            .budget
            .state
            .lock()
            .expect("oversized protocol frames are admitted alone: valid fixture operation")
            .bytes,
        capacity
    );
    assert_eq!(
        receiver
            .poll(Duration::ZERO)
            .expect("oversized protocol frames are admitted alone: valid fixture operation")
            .expect("oversized protocol frames are admitted alone: valid fixture operation")
            .as_str()
            .expect("oversized protocol frames are admitted alone: valid fixture operation")
            .len(),
        32 * 1024 * 1024 - 1
    );
    assert_eq!(
        sender
            .budget
            .state
            .lock()
            .expect("oversized protocol frames are admitted alone: valid fixture operation")
            .bytes,
        0
    );
}

#[test]
fn malformed_json_and_reader_errors_remain_fatal_when_polled() {
    let (sender, receiver) = channel();
    sender
        .send(Ok(Framing::Lines
            .read(&mut Cursor::new(b"{bad}\n"))
            .expect("malformed json and reader errors remain fatal when polled: valid fixture operation")
            .expect("malformed json and reader errors remain fatal when polled: valid fixture operation")))
        .expect("malformed json and reader errors remain fatal when polled: valid fixture operation");
    sender.send(Ok(frame(64))).expect(
        "malformed json and reader errors remain fatal when polled: valid fixture operation",
    );
    assert!(
        receiver
            .poll(Duration::ZERO)
            .expect_err("malformed json and reader errors remain fatal when polled: invalid fixture must fail")
            .to_string()
            .contains("key must be a string")
    );
    assert_eq!(
        receiver.poll(Duration::ZERO).expect_err("malformed json and reader errors remain fatal when polled: invalid fixture must fail").to_string(),
        "The server process closed its output"
    );
    assert!(sender.send(Ok(frame(64))).is_err());
    assert_eq!(
        sender
            .budget
            .state
            .lock()
            .expect(
                "malformed json and reader errors remain fatal when polled: valid fixture operation"
            )
            .bytes,
        0
    );

    let (sender, receiver) = channel();
    assert!(
        receiver
            .poll(Duration::ZERO)
            .expect(
                "malformed json and reader errors remain fatal when polled: valid fixture operation"
            )
            .is_none()
    );
    sender.send(Err(failure("fixture read error"))).expect(
        "malformed json and reader errors remain fatal when polled: valid fixture operation",
    );
    assert_eq!(
        receiver.poll(Duration::ZERO).expect_err("malformed json and reader errors remain fatal when polled: invalid fixture must fail").to_string(),
        "fixture read error"
    );
    assert!(sender.send(Ok(frame(64))).is_err());
}

#[test]
#[ignore = "fresh-process RAM benchmark; set PRISM_MEMORY_IMPL=baseline|candidate and PRISM_MEMORY_FIXTURE to a file of 32 JSON array lines"]
#[expect(
    clippy::assertions_on_constants,
    reason = "This opt-in benchmark must reject debug builds"
)]
fn memory_rpc_queue() {
    assert!(
        !cfg!(debug_assertions),
        "run memory benchmarks with --release"
    );
    let implementation =
        std::env::var("PRISM_MEMORY_IMPL").expect("memory rpc queue: valid fixture operation");
    assert!(matches!(implementation.as_str(), "baseline" | "candidate"));
    let path = std::env::var_os("PRISM_MEMORY_FIXTURE")
        .expect("memory rpc queue: valid fixture operation");
    let input = std::fs::File::open(path).expect("memory rpc queue: valid fixture operation");
    let mut input = BufReader::new(input);
    let first = Framing::Lines
        .read(&mut input)
        .expect("memory rpc queue: valid fixture operation")
        .expect("memory rpc queue: valid fixture operation");
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
                let Some(frame) = next.take().or_else(|| {
                    Framing::Lines
                        .read(&mut input)
                        .expect("memory rpc queue: valid fixture operation")
                }) else {
                    break;
                };
                let value = frame
                    .decode()
                    .expect("memory rpc queue: valid fixture operation");
                read += 1;
                if read == MAX_MESSAGES + 1 {
                    ready
                        .send(())
                        .expect("memory rpc queue: valid fixture operation");
                }
                sender
                    .send(value)
                    .expect("memory rpc queue: valid fixture operation");
            }
        });
        saturated
            .recv_timeout(Duration::from_secs(30))
            .expect("memory rpc queue: valid fixture operation");
        while let Ok(value) = receiver.recv() {
            let array = value
                .as_array()
                .expect("memory rpc queue: valid fixture operation");
            assert!(array.iter().all(|item| item == 0));
            elements += array.len();
            count += 1;
        }
        producer
            .join()
            .expect("memory rpc queue: valid fixture operation");
    } else {
        let (sender, receiver) = channel();
        let producer = std::thread::spawn(move || {
            let mut next = Some(first);
            let mut read = 0;
            loop {
                let Some(frame) = next.take().or_else(|| {
                    Framing::Lines
                        .read(&mut input)
                        .expect("memory rpc queue: valid fixture operation")
                }) else {
                    break;
                };
                read += 1;
                if read == queue_count + 1 {
                    ready
                        .send(())
                        .expect("memory rpc queue: valid fixture operation");
                }
                sender
                    .send(Ok(frame))
                    .expect("memory rpc queue: valid fixture operation");
            }
        });
        saturated
            .recv_timeout(Duration::from_secs(30))
            .expect("memory rpc queue: valid fixture operation");
        loop {
            match receiver.poll(Duration::from_secs(30)) {
                Ok(Some(value)) => {
                    let array = value
                        .as_array()
                        .expect("memory rpc queue: valid fixture operation");
                    assert!(array.iter().all(|item| item == 0));
                    elements += array.len();
                    count += 1;
                }
                Err(error) if error.to_string() == "The server process closed its output" => break,
                other => panic!("unexpected queue result: {other:?}"),
            }
        }
        producer
            .join()
            .expect("memory rpc queue: valid fixture operation");
    }
    assert_eq!(count, 32);
    assert_eq!(elements, 32 * 524_288);
    eprintln!(
        "memory_rpc_queue impl={implementation} frames={count} elements={elements} frame_capacity={capacity} queue_slots={queue_count} elapsed_ms={:.3}",
        started.elapsed().as_secs_f64() * 1000.0
    );
}
