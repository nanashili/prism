use super::*;
use serde_json::json;

fn fixture(
    decoded: &[u8],
    codec: &str,
    raw: u64,
    digest: String,
    truncate: bool,
) -> Result<tempfile::NamedTempFile> {
    let mut payload = match codec {
        "zstd" => zstd::encode_all(decoded, 3)?,
        _ => decoded.to_vec(),
    };
    if truncate {
        payload.truncate(payload.len().saturating_sub(2));
    }
    let manifest = Manifest {
        format: 1,
        name: "test".into(),
        kind: "test".into(),
        arch: "test".into(),
        meta: String::new(),
        binary: String::new(),
        saved_at: 0,
        app: "test".into(),
        entries: vec![Entry {
            name: "test.json".into(),
            codec: codec.into(),
            offset: 0,
            size: payload.len() as u64,
            raw,
            sha256: digest,
        }],
    };
    let manifest = serde_json::to_vec(&manifest)?;
    let mut file = tempfile::NamedTempFile::new()?;
    file.write_all(MAGIC)?;
    file.write_all(&(manifest.len() as u32).to_le_bytes())?;
    file.write_all(&manifest)?;
    file.write_all(&payload)?;
    Ok(file)
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[test]
fn streamed_json_preserves_values_and_verified_extraction() -> Result<()> {
    let bytes =
        " \r\n{\"unicode\":\"🦀 café\",\"nested\":[null,true,1,{\"x\":\"escaped\\nline\"}]} \t\n"
            .as_bytes();
    for codec in ["none", "zstd"] {
        let file = fixture(bytes, codec, bytes.len() as u64, digest(bytes), false)?;
        let mut archive = Archive::open(file.path())?;
        assert_eq!(
            archive.json("test.json")?,
            serde_json::from_slice::<serde_json::Value>(bytes)?
        );
        let mut extracted = Vec::new();
        archive.extract("test.json", &mut extracted, 1024)?;
        assert_eq!(extracted, bytes);
    }
    Ok(())
}

#[test]
fn streamed_json_rejects_corruption_size_mismatch_and_trailing_data() -> Result<()> {
    for codec in ["none", "zstd"] {
        for bytes in [b"[]".as_slice(), b"[] trailing invalid JSON".as_slice()] {
            let file = fixture(bytes, codec, bytes.len() as u64, "0".repeat(64), false)?;
            let error = Archive::open(file.path())?.json("test.json").expect_err("streamed json rejects corruption size mismatch and trailing data: invalid fixture must fail");
            assert!(error.to_string().contains("integrity check"), "{error}");
        }
        for raw in [1, 3] {
            let file = fixture(b"[]", codec, raw, digest(b"[]"), false)?;
            assert!(Archive::open(file.path())?.json("test.json").is_err());
        }
        let file = fixture(b"[] {}", codec, 5, digest(b"[] {}"), false)?;
        assert!(Archive::open(file.path())?.json("test.json").is_err());
        let file = fixture(b"[]", codec, 32 * 1024 * 1024 + 1, digest(b"[]"), false)?;
        let error = Archive::open(file.path())?.json("test.json").expect_err("streamed json rejects corruption size mismatch and trailing data: invalid fixture must fail");
        assert!(error.to_string().contains("configured read limit"));
    }
    let file = fixture(b"{\"a\":1}", "zstd", 7, digest(b"{\"a\":1}"), true)?;
    assert!(Archive::open(file.path())?.json("test.json").is_err());
    Ok(())
}

#[test]
fn streamed_json_checks_data_after_the_buffer_and_accepts_empty_strings() -> Result<()> {
    let mut bytes = serde_json::to_vec(&json!({"value":"x".repeat(130_000),"empty":""}))?;
    bytes.extend_from_slice(b" \n");
    let file = fixture(&bytes, "zstd", bytes.len() as u64, digest(&bytes), false)?;
    assert_eq!(
        Archive::open(file.path())?.json("test.json")?["value"]
            .as_str()
            .expect("streamed json checks data after the buffer and accepts empty strings: valid fixture operation")
            .len(),
        130_000
    );
    bytes.extend_from_slice(b"{}");
    let file = fixture(&bytes, "zstd", bytes.len() as u64, digest(&bytes), false)?;
    assert!(Archive::open(file.path())?.json("test.json").is_err());
    Ok(())
}

#[test]
#[ignore = "fresh-process RSS workload; requires PRISM_MEMORY_FIXTURE JSON file"]
fn memory_archive_json() -> Result<()> {
    use std::hint::black_box;
    use std::time::Instant;

    let input = std::env::var_os("PRISM_MEMORY_FIXTURE").ok_or("Set PRISM_MEMORY_FIXTURE")?;
    let mode = std::env::var("PRISM_MEMORY_IMPL").unwrap_or_else(|_| "candidate".into());
    let mut payload = tempfile::tempfile()?;
    let entry = pack_part("test.json", File::open(input)?, &mut payload, 0)?;
    assert!(entry.raw <= 32 * 1024 * 1024);
    let raw_bytes = entry.raw;
    let manifest = serde_json::to_vec(&Manifest {
        format: 1,
        name: "benchmark".into(),
        kind: String::new(),
        arch: String::new(),
        meta: String::new(),
        binary: String::new(),
        saved_at: 0,
        app: "benchmark".into(),
        entries: vec![entry],
    })?;
    let mut file = tempfile::NamedTempFile::new()?;
    file.write_all(MAGIC)?;
    file.write_all(&(manifest.len() as u32).to_le_bytes())?;
    file.write_all(&manifest)?;
    payload.rewind()?;
    std::io::copy(&mut payload, &mut file)?;
    let mut archive = Archive::open(file.path())?;
    let started = Instant::now();
    let value: serde_json::Value = match mode.as_str() {
        "baseline" => {
            let mut bytes = Vec::new();
            archive.extract("test.json", &mut bytes, 32 * 1024 * 1024)?;
            serde_json::from_slice(&bytes)?
        }
        "candidate" => archive.json("test.json")?,
        _ => return Err("PRISM_MEMORY_IMPL must be baseline or candidate".into()),
    };
    let elapsed = started.elapsed();
    struct DigestWriter(Sha256);
    impl Write for DigestWriter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.update(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut digest = DigestWriter(Sha256::new());
    serde_json::to_writer(&mut digest, &value)?;
    println!(
        "archive_json mode={mode} raw_bytes={raw_bytes} semantic_sha256={:x} elapsed_ms={:.3}",
        digest.0.finalize(),
        elapsed.as_secs_f64() * 1000.0
    );
    black_box(value);
    Ok(())
}
