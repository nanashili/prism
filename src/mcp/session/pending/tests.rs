use super::*;
use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::time::Duration;

#[test]
fn moves_large_call_parameters_and_delivers_the_same_response() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let token = "a".repeat(64);
    let endpoint = Endpoint::new(&listener.local_addr().unwrap().to_string(), &token).unwrap();
    let server = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut bytes = Vec::new();
        BufReader::new(&mut socket)
            .read_until(b'\n', &mut bytes)
            .unwrap();
        let wire: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(wire["token"], token);
        assert_eq!(wire["params"]["name"], "analysis_call");
        assert_eq!(
            wire["params"]["arguments"]["arguments"]["blob"]
                .as_str()
                .unwrap()
                .len(),
            2 * 1024 * 1024
        );
        socket.write_all(b"{\"result\":{\"ok\":true}}\n").unwrap();
    });
    let mut request = json!({
        "jsonrpc":"2.0", "id":42, "method":"tools/call",
        "params":{"name":"analysis_call","arguments":{"name":"echo","arguments":{"blob":"x".repeat(2 * 1024 * 1024)}}}
    });
    let mut pending = Pending::new(Vec::<u8>::new());
    assert!(
        pending
            .accept(
                &mut request,
                2 * 1024 * 1024 + 1024,
                Mode::Analysis,
                Some(&endpoint)
            )
            .unwrap()
    );
    assert!(request["id"].is_null());
    assert!(request["params"].is_null());
    let deadline = Instant::now() + Duration::from_secs(10);
    while !pending.requests.is_empty() {
        assert!(Instant::now() < deadline, "request failed to finish");
        pending.reap().unwrap();
        std::thread::sleep(Duration::from_millis(1));
    }
    server.join().unwrap();
    let output = pending.output.lock().unwrap();
    let response: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(response["id"], 42);
    assert_eq!(response["result"]["structuredContent"], json!({"ok":true}));
}

#[test]
fn rejected_and_non_tool_requests_keep_their_parameters() {
    let mut pending = Pending::new(Vec::<u8>::new());
    for (method, accepted) in [("ping", false), ("tools/call", true)] {
        let mut request = json!({"jsonrpc":"2.0","id":"request","method":method,"params":{"name":"unknown","arguments":{}}});
        let original = request.clone();
        assert_eq!(
            pending
                .accept(&mut request, 128, Mode::Combined, None)
                .unwrap(),
            accepted
        );
        assert_eq!(request, original);
    }
    let output = pending.output.lock().unwrap();
    let response: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(response["id"], "request");
    assert_eq!(response["error"]["code"], -32602);
}
