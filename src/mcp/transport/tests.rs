use super::*;
use std::net::{TcpListener, TcpStream};

type Reader = fn(&mut TcpStream, &AtomicBool, Instant) -> Result<Vec<u8>>;

fn connection() -> (TcpStream, TcpStream) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let receiver = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (sender, _) = listener.accept().unwrap();
    (receiver, sender)
}

fn receive(payload: &[u8], chunk_size: usize, read: Reader) -> (Result<Vec<u8>>, Duration) {
    let (mut receiver, mut sender) = connection();
    std::thread::scope(|scope| {
        let writer = scope.spawn(move || {
            for chunk in payload.chunks(chunk_size) {
                sender.write_all(chunk)?;
            }
            std::io::Result::Ok(())
        });
        let started = Instant::now();
        let result = read(
            &mut receiver,
            &AtomicBool::new(false),
            started + Duration::from_secs(30),
        );
        let elapsed = started.elapsed();
        drop(receiver);
        let sent = writer.join().unwrap();
        if result.is_ok() {
            sent.unwrap();
        }
        (result, elapsed)
    })
}

#[test]
fn receives_fragmented_frames_and_newline_at_chunk_boundary() {
    for length in [1, 4096, 4097, 32 * 1024 + 1] {
        let mut payload = vec![b'x'; length];
        payload[length - 1] = b'\n';
        for chunk_size in [1, 4096, 8192] {
            assert_eq!(
                receive(&payload, chunk_size, read_cancellable).0.unwrap(),
                payload
            );
        }
    }
}

#[test]
fn enforces_reply_limit_including_newline() {
    let mut payload = vec![b'x'; MAX_MESSAGE];
    payload[MAX_MESSAGE - 1] = b'\n';
    assert_eq!(
        receive(&payload, 4096, read_cancellable).0.unwrap(),
        payload
    );
    payload[MAX_MESSAGE - 1] = b'x';
    payload.push(b'\n');
    assert_eq!(
        receive(&payload, 4096, read_cancellable)
            .0
            .unwrap_err()
            .to_string(),
        "MCP reply exceeds 8 MiB"
    );
}

#[test]
fn rejects_eof_before_newline() {
    for payload in [b"".as_slice(), b"unterminated"] {
        assert_eq!(
            receive(payload, 4096, read_cancellable)
                .0
                .unwrap_err()
                .to_string(),
            "The window bridge closed"
        );
    }
}

#[test]
fn checks_cancellation_and_deadline_before_reading() {
    let (mut receiver, _sender) = connection();
    assert_eq!(
        read_cancellable(
            &mut receiver,
            &AtomicBool::new(true),
            Instant::now() + Duration::from_secs(1),
        )
        .unwrap_err()
        .to_string(),
        "Tool request cancelled"
    );
    assert_eq!(
        read_cancellable(&mut receiver, &AtomicBool::new(false), Instant::now())
            .unwrap_err()
            .to_string(),
        "The tool request timed out"
    );
}

// The original receiver is kept only as a benchmark reference. Socket reads,
// allocations, timeouts and limits match production; only delimiter search differs.
fn baseline(socket: &mut TcpStream, cancelled: &AtomicBool, deadline: Instant) -> Result<Vec<u8>> {
    socket.set_read_timeout(Some(Duration::from_millis(100)))?;
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        if cancelled.load(Ordering::Relaxed) {
            return Err(failure("Tool request cancelled"));
        }
        if Instant::now() >= deadline {
            return Err(failure("The tool request timed out"));
        }
        match socket.read(&mut buffer) {
            Ok(0) => return Err(failure("The window bridge closed")),
            Ok(length) => {
                bytes.extend_from_slice(&buffer[..length]);
                if bytes.len() > MAX_MESSAGE {
                    return Err(failure("MCP reply exceeds 8 MiB"));
                }
                if bytes.contains(&b'\n') {
                    return Ok(bytes);
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::WouldBlock
                        | std::io::ErrorKind::Interrupted
                ) => {}
            Err(error) => return Err(error.into()),
        }
    }
}

#[test]
#[ignore = "release benchmark: cargo test --release --locked --lib benchmark_mcp_receive -- --ignored --nocapture --test-threads=1"]
fn benchmark_mcp_receive() {
    if cfg!(debug_assertions) {
        panic!("run benchmarks with --release");
    }
    for length in [16 * 1024, 1024 * 1024, MAX_MESSAGE] {
        let mut payload = vec![b'x'; length];
        payload[length - 1] = b'\n';
        for read in [baseline as Reader, read_cancellable] {
            assert_eq!(receive(&payload, 4096, read).0.unwrap(), payload);
        }
        let mut before = Vec::new();
        let mut after = Vec::new();
        for round in 0..11 {
            let readers = if round % 2 == 0 {
                [
                    (baseline as Reader, &mut before),
                    (read_cancellable, &mut after),
                ]
            } else {
                [
                    (read_cancellable as Reader, &mut after),
                    (baseline, &mut before),
                ]
            };
            for (read, samples) in readers {
                let (result, elapsed) = receive(&payload, 4096, read);
                assert_eq!(result.unwrap(), payload);
                samples.push(elapsed);
            }
        }
        before.sort();
        after.sort();
        eprintln!(
            "mcp_receive bytes={length} samples=11 baseline_median_us={:.3} current_median_us={:.3} speedup={:.2} baseline_range_us={:.3}..{:.3} current_range_us={:.3}..{:.3}",
            before[5].as_secs_f64() * 1e6,
            after[5].as_secs_f64() * 1e6,
            before[5].as_secs_f64() / after[5].as_secs_f64(),
            before[0].as_secs_f64() * 1e6,
            before[10].as_secs_f64() * 1e6,
            after[0].as_secs_f64() * 1e6,
            after[10].as_secs_f64() * 1e6,
        );
    }
}
