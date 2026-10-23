use super::*;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::time::Instant;

// Exact prior read_json, find_class and method rendering path.
fn baseline(path: &Path, owner: &str, signature: &str) -> Result<String> {
    let node = super::super::read_json(path)?;
    let class = find_class(&node, owner, 0).ok_or("The indexed Android class is missing")?;
    let method = class["methods"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|method| method["signature"] == signature)
        .ok_or("The indexed Android method is missing")?;
    let mut text = format!(
        "{} {{\n",
        method["declaration"].as_str().unwrap_or(signature)
    );
    for line in method["lines"].as_array().into_iter().flatten() {
        text.push_str("    ");
        text.push_str(line["code"].as_str().unwrap_or_default());
        text.push('\n');
        if text.len() > TEXT_LIMIT {
            return Err(failure("This method exceeds the code display limit"));
        }
    }
    text.push_str("}\n");
    Ok(text)
}

fn find_class<'a>(node: &'a Value, owner: &str, depth: usize) -> Option<&'a Value> {
    if depth > 64 {
        return None;
    }
    if node["name"] == owner {
        return Some(node);
    }
    node["inner-classes"]
        .as_array()?
        .iter()
        .find_map(|inner| find_class(inner, owner, depth + 1))
}

fn parity(path: &Path, json: impl AsRef<[u8]>, owner: &str, signature: &str) -> Result<()> {
    std::fs::write(path, json)?;
    assert_eq!(
        code(path, owner, signature).map_err(|error| error.to_string()),
        baseline(path, owner, signature).map_err(|error| error.to_string()),
    );
    Ok(())
}

#[test]
fn selected_source_keeps_field_order_fallbacks_and_class_precedence() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("class.json");
    for document in [
        r#"{"name":"C","methods":[{"signature":"f()V","declaration":"void f()","lines":[{"code":"snow 雪"},null,{},"ignored",{"code":42}]}]}"#,
        r#"{"methods":[{"lines":[{"code":"parent"}],"declaration":"void f()","signature":"f()V"}],"inner-classes":[{"name":"C","methods":[{"signature":"f()V","lines":[{"code":"child"}]}]}],"name":"C"}"#,
        r#"{"inner-classes":[{"name":"C","methods":[{"signature":"f()V"}]}],"name":"C","methods":[]}"#,
        r#"{"name":"Outer","inner-classes":[{"name":"C","methods":[]},{"name":"C","methods":[{"signature":"f()V"}]}]}"#,
        r#"{"name":"Outer","inner-classes":[null,{}, {"methods":[{"signature":"f()V","lines":[{"code":"inner"}]}],"name":"C"}]}"#,
        r#"{"name":"C","methods":[null,{},false,{"signature":"other"},{"signature":"f()V","declaration":null,"lines":null}]}"#,
        r#"{"name":"C","methods":[{"signature":"f()V","declaration":false,"lines":{}}]}"#,
        r#"{"name":"C","methods":false,"inner-classes":null}"#,
        r#"{"name":"Other","methods":[{"signature":"f()V"}]}"#,
        r#"{"name":"C","methods":[{"signature":"f()V","lines":[{"code":"first"}]},{"signature":"f()V","lines":[{"code":"second"}]}]}"#,
    ] {
        parity(&path, document, "C", "f()V")?;
    }
    parity(
        &path,
        br#"{"name":"C","methods":[{"signature":"missing"}]}"#,
        "C",
        "f()V",
    )?;
    parity(
        &path,
        r#"{"methods":[{"lines":[{"code":"雪"}],"signature":"雪()V"}],"name":"雪"}"#,
        "雪",
        "雪()V",
    )?;
    Ok(())
}

#[test]
fn selected_source_keeps_duplicate_key_last_wins_semantics() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("class.json");
    for document in [
        r#"{"name":"C","name":"Outer","methods":[{"signature":"f()V","lines":[{"code":"parent"}]}],"inner-classes":[{"name":"C","methods":[{"signature":"f()V","lines":[{"code":"child"}]}]}]}"#,
        r#"{"name":"Outer","name":"C","methods":[{"signature":"other","lines":[{"code":"selected"}],"signature":"f()V"}]}"#,
        r#"{"name":"C","methods":[{"signature":"f()V","signature":"other"},{"signature":"f()V","lines":[{"code":"last"}]}]}"#,
        r#"{"name":"C","methods":[{"signature":"other"}],"methods":[{"signature":"f()V","declaration":"old","declaration":null,"lines":[{"code":"old"}],"lines":[{"code":"before","code":"after"}]}]}"#,
        r#"{"name":"Outer","inner-classes":[{"name":"C","methods":[]}],"inner-classes":[{"name":"C","methods":[{"signature":"f()V"}]}]}"#,
        r#"{"name":"C","methods":[{"signature":"f()V","lines":[{"code":"ignored","code":null}]}]}"#,
    ] {
        parity(&path, document, "C", "f()V")?;
    }
    Ok(())
}

#[test]
fn source_limits_only_affect_the_selected_method_and_preserve_exact_boundary() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("class.json");
    let oversized = "x".repeat(TEXT_LIMIT + 1);
    let unselected = json!({"name":"C", "methods":[
        {"signature":"other", "lines":[{"code":oversized}]},
        {"signature":"f()V", "lines":[{"code":"small"}]}
    ]});
    parity(&path, serde_json::to_vec(&unselected)?, "C", "f()V")?;
    // A matching signature in a different class must not fail the chosen class.
    let outer = json!({"name":"Other", "methods":[{"signature":"f()V","lines":[{"code":oversized}]}], "inner-classes":[unselected]});
    parity(&path, serde_json::to_vec(&outer)?, "C", "f()V")?;
    for delta in [0, 1] {
        let header = "f()V {\n".len();
        let code = "a".repeat(TEXT_LIMIT - header - 5 + delta);
        let document =
            json!({"name":"C", "methods":[{"signature":"f()V", "lines":[{"code":code}]}]});
        parity(&path, serde_json::to_vec(&document)?, "C", "f()V")?;
    }
    let selected =
        json!({"name":"C", "methods":[{"signature":"f()V", "lines":[{"code":oversized}]}]});
    parity(&path, serde_json::to_vec(&selected)?, "C", "f()V")?;
    // Existing code checks the limit after each source line; an empty method
    // with a very long declaration retains its original behavior.
    let empty =
        json!({"name":"C", "methods":[{"signature":"f()V", "declaration":oversized, "lines":[]}]});
    parity(&path, serde_json::to_vec(&empty)?, "C", "f()V")?;
    let encoded = serde_json::to_string(&oversized)?;
    parity(
        &path,
        format!(
            "{{\"name\":\"C\",\"methods\":[{{\"signature\":\"f()V\",\"lines\":[{{\"code\":{encoded},\"code\":\"last\"}}]}}]}}"
        ),
        "C",
        "f()V",
    )?;
    parity(
        &path,
        format!(
            "{{\"name\":\"C\",\"methods\":[{{\"signature\":\"f()V\",\"lines\":[{{\"code\":{encoded}}}],\"lines\":[{{\"code\":\"last\"}}]}}]}}"
        ),
        "C",
        "f()V",
    )?;
    Ok(())
}

#[test]
fn selected_source_still_validates_discarded_and_trailing_input() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("class.json");
    for document in [
        b"{\"name\":\"C\",\"methods\":[{\"signature\":\"f()V\"},{\"ignored\":\"bad\xfftext\"}]}"
            .as_slice(),
        br#"{"name":"C","methods":[{"signature":"f()V"}],"unused":[1e9999]}"#,
        br#"{"name":"C","methods":[{"signature":"f()V"}],"unused":"\uD800"}"#,
        br#"{"name":"C","methods":[{"signature":"f()V"}]} trailing"#,
        br#"{"name":"C","methods":[{"signature":"f()V"},]}"#,
    ] {
        std::fs::write(&path, document)?;
        assert!(baseline(&path, "C", "f()V").is_err());
        assert!(code(&path, "C", "f()V").is_err());
    }
    let deep = format!(
        "{{\"name\":\"C\",\"methods\":[{{\"signature\":\"f()V\"}}],\"unused\":{}0{}}}",
        "[".repeat(130),
        "]".repeat(130)
    );
    std::fs::write(&path, deep)?;
    assert!(baseline(&path, "C", "f()V").is_err());
    assert!(code(&path, "C", "f()V").is_err());
    Ok(())
}

#[test]
#[ignore = "Run baseline/candidate in fresh processes with the external Android fixture and peak RSS collection"]
fn memory_android_source() -> Result<()> {
    let path = std::env::var_os("PRISM_MEMORY_FIXTURE").ok_or("Set PRISM_MEMORY_FIXTURE")?;
    let implementation = std::env::var("PRISM_MEMORY_IMPL")?;
    let started = Instant::now();
    let text = match implementation.as_str() {
        "baseline" => baseline(Path::new(&path), "example.LargeClass", "method999()V")?,
        "candidate" => code(Path::new(&path), "example.LargeClass", "method999()V")?,
        _ => return Err(failure("PRISM_MEMORY_IMPL must be baseline or candidate")),
    };
    let elapsed = started.elapsed();
    assert!(text.starts_with("public void method999() {\n"));
    assert_eq!(text.lines().count(), 82);
    println!(
        "memory_android_source implementation={implementation} elapsed_ms={:.3} bytes={} checksum={:x}",
        elapsed.as_secs_f64() * 1000.0,
        text.len(),
        Sha256::digest(text.as_bytes())
    );
    Ok(())
}
