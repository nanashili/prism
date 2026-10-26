use super::android_output::{cache, decompile};
use crate::error::{Result, failure};
use crate::events::Sink;
use crate::model::{Procedure, Target};
use crate::settings::Settings;
use crate::storage::{FunctionImport, Store};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

mod inventory;
mod source;

#[cfg(test)]
mod tests;

#[cfg_attr(feature = "profiling", hotpath::measure)]
pub(crate) fn index(
    target: &Target,
    settings: &Settings,
    store: &mut Store,
    cancelled: &AtomicBool,
    sink: &Sink,
) -> Result<usize> {
    sink.analysis("Decompiling Android package", 0, None)?;
    let root = decompile(target, settings, cancelled)?;
    let mapping = inventory::read(&root.join("sources/mapping.json"))?;
    let classes = mapping["classes"]
        .as_array()
        .ok_or("jadx did not return a class mapping")?;
    sink.analysis("Indexing classes", 0, Some(classes.len() as u64))?;
    let mut inventory = store.begin_functions(&target.sha256)?;
    let mut updated = Instant::now();
    for (index, class) in classes.iter().enumerate() {
        if cancelled.load(Ordering::Relaxed) {
            return Err(failure("Cancelled"));
        }
        if class["inner"] == true {
            continue;
        }
        let file = class["json"]
            .as_str()
            .ok_or("A jadx class has no source file")?;
        let path = confined(&root.join("sources"), file)?;
        let node = inventory::read(&path)?;
        collect(&node, file, "classes.dex", &mut inventory, 0)?;
        if updated.elapsed() >= Duration::from_millis(100) {
            sink.analysis(
                "Indexing classes",
                (index + 1) as u64,
                Some(classes.len() as u64),
            )?;
            updated = Instant::now();
        }
    }
    if cancelled.load(Ordering::Relaxed) {
        return Err(failure("Cancelled"));
    }
    sink.analysis(
        "Indexing classes",
        classes.len() as u64,
        Some(classes.len() as u64),
    )?;
    inventory.finish()
}

fn collect(
    node: &Value,
    file: &str,
    inherited: &str,
    inventory: &mut FunctionImport<'_>,
    depth: usize,
) -> Result<()> {
    if depth > 64 {
        return Err(failure("The Android class nesting exceeds 64 levels"));
    }
    let owner = node["name"]
        .as_str()
        .ok_or("An Android class has no name")?;
    let dex = node["dex"].as_str().unwrap_or(inherited);
    let number = dex.trim_start_matches("classes").trim_end_matches(".dex");
    let number = if number.is_empty() {
        1
    } else {
        number.parse::<u32>()?
    };
    for method in node["methods"].as_array().into_iter().flatten() {
        let Some(offset) = method["offset"]
            .as_str()
            .and_then(super::graph::address_value)
        else {
            continue;
        };
        if offset == 0 {
            continue;
        }
        let offset = u32::try_from(offset)?;
        let address = format!("0x{:x}", (u64::from(number) << 32) | u64::from(offset));
        let name = method["name"]
            .as_str()
            .ok_or("An Android method has no name")?;
        let signature = method["signature"]
            .as_str()
            .ok_or("An Android method has no signature")?;
        let procedure = Procedure {
            address: address.clone(),
            name: format!("{owner}.{name}"),
            kind: "Dalvik method".into(),
        };
        inventory.append(&[procedure])?;
        inventory.source(&address, file, owner, signature)?;
    }
    for inner in node["inner-classes"].as_array().into_iter().flatten() {
        collect(inner, file, dex, inventory, depth + 1)?;
    }
    Ok(())
}

#[cfg_attr(feature = "profiling", hotpath::measure)]
pub(crate) fn code(target: &Target, store: &Store, address: &str) -> Result<String> {
    let (file, owner, signature) = store.function_source(&target.sha256, address)?;
    let root = cache(target).join("sources");
    source::code(&confined(&root, &file)?, &owner, &signature)
}

fn confined(root: &Path, relative: &str) -> Result<PathBuf> {
    let path = root.join(relative).canonicalize()?;
    if !path.starts_with(root.canonicalize()?) {
        return Err(failure("A jadx source path leaves its output folder"));
    }
    Ok(path)
}

#[cfg(test)]
fn read_json(path: &Path) -> Result<Value> {
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 16 * 1024 * 1024 {
        return Err(failure("The Android metadata exceeds 16 MiB"));
    }
    Ok(serde_json::from_slice(&bytes)?)
}
