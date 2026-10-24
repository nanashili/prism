use super::*;
use std::io::Cursor;

fn decode(framing: Framing, bytes: &[u8]) -> Result<Value> {
    framing
        .read(&mut Cursor::new(bytes))?
        .ok_or_else(|| failure("missing frame"))?
        .decode()
}

#[test]
fn frames_preserve_json_and_rizin_text_fallbacks() {
    assert_eq!(
        decode(Framing::Lines, b"{\"result\":[1,true,null]}\n").unwrap(),
        serde_json::json!({"result":[1,true,null]})
    );
    assert_eq!(
        decode(Framing::Dap, b"Content-Length: 7\r\n\r\n{\"x\":1}").unwrap(),
        serde_json::json!({"x":1})
    );
    assert_eq!(
        decode(Framing::Zero, b"[1,2]\0").unwrap(),
        serde_json::json!([1, 2])
    );
    assert_eq!(
        decode(Framing::Zero, b"plain text\0").unwrap(),
        Value::String("plain text".into())
    );
    assert_eq!(
        decode(Framing::Zero, b"invalid \xff\0").unwrap(),
        Value::String("invalid �".into())
    );
    assert!(decode(Framing::Lines, b"{bad}\n").is_err());
    assert!(decode(Framing::Dap, b"Content-Length: 7\r\n\r\n{}").is_err());
}

#[test]
fn protocol_frame_limits_still_apply_before_queueing() {
    let bytes = vec![b'x'; MAX_FRAME + 1];
    assert!(Framing::Lines.read(&mut Cursor::new(bytes)).is_err());
    assert!(
        Framing::Dap
            .read(&mut Cursor::new(b"Content-Length: 8388609\r\n\r\n"))
            .is_err()
    );
    let mut bytes = vec![b'x'; 32 * 1024 * 1024];
    *bytes.last_mut().unwrap() = 0;
    let frame = Framing::Zero
        .read(&mut Cursor::new(&bytes))
        .unwrap()
        .unwrap();
    assert_eq!(
        frame.decode().unwrap().as_str().unwrap().len(),
        bytes.len() - 1
    );
    bytes.push(0);
    bytes[32 * 1024 * 1024 - 1] = b'x';
    assert!(Framing::Zero.read(&mut Cursor::new(bytes)).is_err());
}
