use super::*;
use std::io::Cursor;

#[test]
fn cancelling_a_saturated_process_queue_still_shuts_down() {
    let cancelled = Arc::new(AtomicBool::new(false));
    let mut command = Command::new("sh");
    command.args(["-c", "while :; do printf '0\\n'; done"]);
    let mut rpc = Rpc::spawn(&mut command, Framing::Lines, cancelled.clone()).unwrap();
    let frame_bytes = Framing::Lines
        .read(&mut Cursor::new(b"0\n"))
        .unwrap()
        .unwrap()
        .capacity();
    let deadline = Instant::now() + Duration::from_secs(5);
    // Sixteen queued frames plus the producer's admitted frame waiting on count.
    while rpc.incoming.as_ref().unwrap().retained_bytes() < frame_bytes * 17 {
        assert!(Instant::now() < deadline, "reader failed to saturate");
        std::thread::sleep(Duration::from_millis(1));
    }
    cancelled.store(true, Ordering::Relaxed);
    assert_eq!(
        rpc.exchange(None, Duration::from_secs(30))
            .unwrap_err()
            .to_string(),
        "Cancelled"
    );
    let (finished, done) = std::sync::mpsc::channel();
    let shutdown = std::thread::spawn(move || {
        drop(rpc);
        finished.send(()).unwrap();
    });
    done.recv_timeout(Duration::from_secs(5))
        .expect("shutdown blocked on queue backpressure");
    shutdown.join().unwrap();
}
