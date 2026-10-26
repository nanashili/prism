use super::*;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::io::Write;

fn persisted_inventory(store: &Store) -> Result<(usize, String)> {
    let mut count = 0;
    let mut digest = Sha256::new();
    loop {
        let page = store.functions("android-test", "", count)?;
        if page.is_empty() {
            break;
        }
        count += page.len();
        for procedure in page {
            let source = store.function_source("android-test", &procedure.address)?;
            digest.update(serde_json::to_vec(&(
                procedure.address,
                procedure.name,
                procedure.kind,
                source,
            ))?);
        }
    }
    Ok((count, format!("{:x}", digest.finalize())))
}

#[test]
fn streamed_inventory_preserves_methods_inner_classes_and_nullable_fields() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("class.json");
    std::fs::write(
        &path,
        serde_json::to_vec(&json!({
            "name": "example.Outer", "dex": "classes2.dex",
            "methods": [
                {"offset":"0x10", "name":"run", "signature":"run()V",
                 "declaration":"public void run()", "lines":[{"code":"ignored body 雪"}]},
                {"offset":null}, {"offset":0}, {"offset":"0x0"}, {}, null
            ],
            "inner-classes": [
                {"name":"example.Outer.Inner", "dex":null,
                 "methods":[{"offset":"20", "name":"go", "signature":"go()I"}],
                 "inner-classes":null},
                {"name":"example.Empty", "methods":null},
                {"name":"example.OtherDex", "dex":"classes3.dex",
                 "methods":[{"offset":"30", "name":"雪", "signature":"雪()V"}]}
            ],
            "annotations":[true, false, null, 3, -4, 1.25, {"name":"not an inventory node"}]
        }))?,
    )?;
    let reference = read_json(&path)?;
    let streamed = inventory::read(&path)?;
    assert!(streamed.get("annotations").is_none());
    assert!(streamed["methods"][0].get("lines").is_none());
    assert!(streamed["methods"][0].get("declaration").is_none());

    let mut checksums = Vec::new();
    for (name, node) in [("reference", reference), ("streamed", streamed)] {
        let mut store = Store::open(&directory.path().join(format!("{name}.db")))?;
        let mut import = store.begin_functions("android-test")?;
        collect(&node, "example/Outer.json", "classes.dex", &mut import, 0)?;
        assert_eq!(import.finish()?, 3);
        checksums.push(persisted_inventory(&store)?);
        assert_eq!(
            store.function_source("android-test", "0x200000020")?,
            (
                "example/Outer.json".into(),
                "example.Outer.Inner".into(),
                "go()I".into()
            )
        );
    }
    assert_eq!(checksums[0], checksums[1]);
    Ok(())
}

#[test]
fn streamed_mapping_and_duplicate_keys_keep_lookup_semantics() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("mapping.json");
    std::fs::write(&path, br#"{"classes":[{"inner":true},{"json":"old","json":"new"},null],"unused":[{"json":"discarded"}]}"#)?;
    let streamed = inventory::read(&path)?;
    assert_eq!(streamed["classes"], read_json(&path)?["classes"]);
    assert!(streamed.get("unused").is_none());
    for document in [r#"{}"#, r#"{"classes":null}"#, r#"{"classes":false}"#] {
        std::fs::write(&path, document)?;
        assert!(inventory::read(&path)?["classes"].as_array().is_none());
    }
    Ok(())
}

#[test]
fn streamed_inventory_validates_discarded_values_and_trailing_data() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("invalid.json");
    let mut invalid = vec![
        b"{\"name\":\"C\",\"lines\":[\"bad\xfftext\"]}".to_vec(),
        br#"{"name":"C","lines":["\uD800"]}"#.to_vec(),
        br#"{"name":"C","lines":[1e9999]}"#.to_vec(),
        br#"{"name":"C","lines":[1,]}"#.to_vec(),
        br#"{"name":"C"} trailing"#.to_vec(),
        br#"{"name":"C","lines":["unfinished"#.to_vec(),
    ];
    invalid.push(
        format!(
            "{{\"name\":\"C\",\"lines\":{}0{}}}",
            "[".repeat(130),
            "]".repeat(130)
        )
        .into_bytes(),
    );
    for bytes in invalid {
        std::fs::write(&path, bytes)?;
        assert!(read_json(&path).is_err());
        assert!(inventory::read(&path).is_err());
    }
    Ok(())
}

#[test]
fn streamed_inventory_keeps_exact_size_limit() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("bounded.json");
    let mut file = std::fs::File::create(&path)?;
    file.write_all(b"{}")?;
    let mut remaining = 16 * 1024 * 1024 - 2;
    let padding = [b' '; 64 * 1024];
    while remaining > 0 {
        let length = remaining.min(padding.len());
        file.write_all(&padding[..length])?;
        remaining -= length;
    }
    assert_eq!(inventory::read(&path)?, json!({}));
    file.write_all(b" ")?;
    assert!(
        inventory::read(&path)
            .expect_err("streamed inventory keeps exact size limit: invalid fixture must fail")
            .to_string()
            .contains("exceeds 16 MiB")
    );
    assert!(
        read_json(&path)
            .expect_err("streamed inventory keeps exact size limit: invalid fixture must fail")
            .to_string()
            .contains("exceeds 16 MiB")
    );
    Ok(())
}

#[test]
fn failed_inventory_import_rolls_back_existing_functions_and_sources() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("class.json");
    let mut store = Store::open(&directory.path().join("prism.db"))?;
    {
        let mut import = store.begin_functions("android-test")?;
        collect(
            &json!({"name":"Original", "methods":[{"offset":"10", "name":"valid", "signature":"valid()V"}]}),
            "old.json",
            "classes.dex",
            &mut import,
            0,
        )?;
        import.finish()?;
    }
    let original = persisted_inventory(&store)?;
    std::fs::write(&path, br#"{"name":"New","methods":[{"offset":"20","name":"first","signature":"first()V"},{"offset":"30","signature":"missingName()V"}]}"#)?;
    {
        let mut import = store.begin_functions("android-test")?;
        let streamed = inventory::read(&path)?;
        assert!(collect(&streamed, "new.json", "classes.dex", &mut import, 0).is_err());
    }
    assert_eq!(persisted_inventory(&store)?, original);
    Ok(())
}

#[test]
#[ignore = "Generate external memory fixture in a separate process before measurements"]
fn generate_android_memory_fixture() -> Result<()> {
    let path = std::env::var_os("PRISM_MEMORY_FIXTURE").ok_or("Set PRISM_MEMORY_FIXTURE")?;
    let file = std::fs::File::create(&path)?;
    let mut output = std::io::BufWriter::new(file);
    output.write_all(br#"{"name":"example.LargeClass","dex":"classes.dex","methods":["#)?;
    for method in 0..1000 {
        if method != 0 {
            output.write_all(b",")?;
        }
        write!(
            output,
            "{{\"name\":\"method{method}\",\"signature\":\"method{method}()V\",\"offset\":\"0x{:x}\",\"declaration\":\"public void method{method}()\",\"lines\":[",
            method + 16
        )?;
        for line in 0..80 {
            if line != 0 {
                output.write_all(b",")?;
            }
            serde_json::to_writer(
                &mut output,
                &json!({
                    "code":"    int currentResult = sourceValues[index] + previousValue; outputValues[index] = currentResult;",
                    "source-line":line,
                    "offset":format!("0x{:x}", method * 320 + line * 4),
                    "column":4
                }),
            )?;
        }
        output.write_all(b"]}")?;
    }
    output.write_all(b"],\"inner-classes\":[]}")?;
    output.flush()?;
    let bytes = std::fs::metadata(&path)?.len();
    assert!(bytes < 16 * 1024 * 1024);
    println!("android_fixture bytes={bytes} methods=1000 source_lines=80000");
    Ok(())
}

#[test]
#[ignore = "Run baseline/candidate in separate fresh processes with external fixture and peak RSS collection"]
fn memory_android_inventory() -> Result<()> {
    let path = std::env::var_os("PRISM_MEMORY_FIXTURE").ok_or("Set PRISM_MEMORY_FIXTURE")?;
    let implementation = std::env::var("PRISM_MEMORY_IMPL")?;
    let directory = tempfile::tempdir()?;
    let mut store = Store::open(&directory.path().join("prism.db"))?;
    let started = Instant::now();
    let mut import = store.begin_functions("android-test")?;
    let node = match implementation.as_str() {
        // Exact prior read_json + collect path, still used by the source viewer.
        "baseline" => read_json(Path::new(&path))?,
        "candidate" => inventory::read(Path::new(&path))?,
        _ => return Err(failure("PRISM_MEMORY_IMPL must be baseline or candidate")),
    };
    collect(
        &node,
        "example/LargeClass.json",
        "classes.dex",
        &mut import,
        0,
    )?;
    let count = import.finish()?;
    std::hint::black_box(&node);
    let elapsed = started.elapsed();
    let (persisted, checksum) = persisted_inventory(&store)?;
    assert_eq!(count, persisted);
    assert_eq!(count, 1000);
    println!(
        "memory_android_inventory implementation={implementation} elapsed_ms={:.3} functions={count} checksum={checksum}",
        elapsed.as_secs_f64() * 1000.0
    );
    Ok(())
}
