use super::*;
use std::io::Cursor;

#[test]
fn cancelling_a_saturated_process_queue_still_shuts_down() {
    let cancelled = Arc::new(AtomicBool::new(false));
    let mut command = Command::new("sh");
    command.args(["-c", "while :; do printf '0\\n'; done"]);
    let mut rpc = Rpc::spawn(&mut command, Framing::Lines, cancelled.clone())
        .expect("cancelling a saturated process queue still shuts down: valid fixture operation");
    let frame_bytes = Framing::Lines
        .read(&mut Cursor::new(b"0\n"))
        .expect("cancelling a saturated process queue still shuts down: valid fixture operation")
        .expect("cancelling a saturated process queue still shuts down: valid fixture operation")
        .capacity();
    let deadline = Instant::now() + Duration::from_secs(5);
    // Sixteen queued frames plus the producer's admitted frame waiting on count.
    while rpc
        .incoming
        .as_ref()
        .expect("cancelling a saturated process queue still shuts down: valid fixture operation")
        .retained_bytes()
        < frame_bytes * 17
    {
        assert!(Instant::now() < deadline, "reader failed to saturate");
        std::thread::sleep(Duration::from_millis(1));
    }
    cancelled.store(true, Ordering::Relaxed);
    assert_eq!(
        rpc.exchange(None, Duration::from_secs(30))
            .expect_err(
                "cancelling a saturated process queue still shuts down: invalid fixture must fail"
            )
            .to_string(),
        "Cancelled"
    );
    let (finished, done) = std::sync::mpsc::channel();
    let shutdown = std::thread::spawn(move || {
        drop(rpc);
        finished.send(()).expect(
            "cancelling a saturated process queue still shuts down: valid fixture operation",
        );
    });
    done.recv_timeout(Duration::from_secs(5))
        .expect("shutdown blocked on queue backpressure");
    shutdown
        .join()
        .expect("cancelling a saturated process queue still shuts down: valid fixture operation");
}
