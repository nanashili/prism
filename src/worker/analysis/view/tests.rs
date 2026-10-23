use super::*;
use sha2::{Digest, Sha256};
use std::time::Instant;

// Exact previous implementation for output equivalence and fresh-process RSS.
fn baseline(mut reply: Value) -> Result<String> {
    let value = reply
        .get_mut("normalized_result")
        .map(Value::take)
        .unwrap_or(reply);
    let mut text = match value {
        Value::String(text) => text,
        value => serde_json::to_string_pretty(&value)?,
    };
    if text.len() > files::TEXT_LIMIT {
        let mut end = files::TEXT_LIMIT;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
        text.push_str("\nDisplay limited to 1 MiB.\n");
    }
    Ok(text)
}

#[test]
fn bounded_json_preserves_small_normalized_and_string_views() -> Result<()> {
    for value in [
        Value::Null,
        json!({"normalized_result":{"text":"雪\nquoted \"text\"", "offsets":[0,1,2]}}),
        json!({"name":"function", "address":"0x1000", "enabled":true}),
        json!("unwrapped text"),
        json!({"normalized_result":"normalized text"}),
        json!([1,2,{"name":"x"}]),
    ] {
        assert_eq!(bounded_view_text(value.clone())?, baseline(value)?);
    }
    Ok(())
}

#[test]
fn bounded_json_preserves_exact_limit_and_unicode_cutoff() -> Result<()> {
    for delta in 0..8 {
        for suffix in ["", "é", "雪", "🦀", "\\\"\n"] {
            let prefix = "a".repeat(files::TEXT_LIMIT - 8 + delta);
            let value = json!([format!("{prefix}{suffix}")]);
            assert_eq!(bounded_view_text(value.clone())?, baseline(value)?);
        }
    }
    let string = Value::String("🦀".repeat(files::TEXT_LIMIT / 4 + 1));
    assert_eq!(bounded_view_text(string.clone())?, baseline(string)?);
    Ok(())
}

#[test]
fn bounded_json_stops_at_existing_display_limit_for_nested_data() -> Result<()> {
    let mut nested = json!((0..32_768).collect::<Vec<_>>());
    for _ in 0..16 {
        nested = Value::Array(vec![nested]);
    }
    let expected = baseline(nested.clone())?;
    assert_eq!(bounded_view_text(nested)?, expected);
    assert!(expected.ends_with("\nDisplay limited to 1 MiB.\n"));
    Ok(())
}

#[test]
fn prefix_writer_bounds_storage_and_reports_partial_writes() -> Result<()> {
    let mut output = JsonPrefix {
        bytes: Vec::new(),
        overflowed: false,
    };
    let limit = files::TEXT_LIMIT + 4;
    output.write_all(&vec![b'a'; limit - 1])?;
    assert_eq!(output.write(b"bc")?, 1);
    assert!(output.write(b"c").is_err());
    assert_eq!(output.bytes.len(), limit);
    assert!(output.overflowed);
    assert_eq!(output.write(b"")?, 0);
    Ok(())
}

#[test]
fn truncated_provider_strings_release_their_large_backing_allocation() -> Result<()> {
    let mut text = String::with_capacity(8 * files::TEXT_LIMIT);
    text.push_str(&"a".repeat(files::TEXT_LIMIT + 1));
    let bounded = bounded_view_text(Value::String(text))?;
    assert_eq!(
        bounded.len(),
        files::TEXT_LIMIT + "\nDisplay limited to 1 MiB.\n".len()
    );
    assert!(bounded.capacity() <= files::TEXT_LIMIT * 2);
    Ok(())
}

#[test]
#[ignore = "Generate external memory fixture separately from fresh-process measurements"]
fn generate_analysis_view_memory_fixture() -> Result<()> {
    let path = std::env::var_os("PRISM_MEMORY_FIXTURE").ok_or("Set PRISM_MEMORY_FIXTURE")?;
    let mut output = std::io::BufWriter::new(std::fs::File::create(&path)?);
    output.write_all(&[b'['; 33])?;
    for index in 0..1_000_000 {
        if index != 0 {
            output.write_all(b",")?;
        }
        output.write_all(b"0")?;
    }
    output.write_all(&[b']'; 33])?;
    output.flush()?;
    println!(
        "analysis_view_fixture bytes={} values=1000000 nesting=33",
        std::fs::metadata(path)?.len()
    );
    Ok(())
}

#[test]
#[ignore = "Run baseline/candidate in separate fresh processes with external fixture and peak RSS collection"]
fn memory_bounded_analysis_view() -> Result<()> {
    let path = std::env::var_os("PRISM_MEMORY_FIXTURE").ok_or("Set PRISM_MEMORY_FIXTURE")?;
    let implementation = std::env::var("PRISM_MEMORY_IMPL")?;
    let value: Value =
        serde_json::from_reader(std::io::BufReader::new(std::fs::File::open(path)?))?;
    let started = Instant::now();
    let text = match implementation.as_str() {
        "baseline" => baseline(value)?,
        "candidate" => bounded_view_text(value)?,
        _ => {
            return Err(crate::error::failure(
                "PRISM_MEMORY_IMPL must be baseline or candidate",
            ));
        }
    };
    let elapsed = started.elapsed();
    assert!(text.ends_with("\nDisplay limited to 1 MiB.\n"));
    assert!(text.len() <= files::TEXT_LIMIT + "\nDisplay limited to 1 MiB.\n".len());
    println!(
        "memory_bounded_analysis_view implementation={implementation} elapsed_ms={:.3} bytes={} checksum={:x}",
        elapsed.as_secs_f64() * 1000.0,
        text.len(),
        Sha256::digest(text.as_bytes())
    );
    Ok(())
}
