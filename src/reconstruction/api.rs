use super::{Configuration, database::Database};
use crate::error::Result;
use crate::reconstruction::{DatabaseOpen, GenerationStart};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

#[cfg_attr(feature = "profiling", hotpath::measure)]
pub(crate) fn call(
    store: &crate::storage::Store,
    project: &str,
    analyzer: &str,
    name: &str,
    args: &Value,
) -> Result<Value> {
    let root = root(project, args)?;
    let mut database = Database::open(&root, DatabaseOpen::from(name == "reconstruction_start"))?;
    database.verify_project(project)?;
    if name == "reconstruction_start" {
        let config = Configuration {
            language: args["language"].as_str().unwrap_or("Rust").trim(),
            instructions: args["instructions"].as_str().unwrap_or_default(),
            analyzer,
            model: args["model"].as_str().unwrap_or("default"),
            prompt_version: args["prompt_version"].as_str().unwrap_or("v1"),
            adapter_version: args["adapter_version"].as_str().unwrap_or("v1"),
            validation_version: args["validation_version"].as_str().unwrap_or("v1"),
            batch_size: args["batch_size"].as_u64().unwrap_or(16).try_into()?,
            workers: args["workers"].as_u64().unwrap_or(2).try_into()?,
        };
        let generation = database.start(
            store,
            project,
            &config,
            GenerationStart::from(args["new_generation"].as_bool().unwrap_or(false)),
        )?;
        remember(project, &database.root)?;
        return database.status(generation);
    }
    let generation = database.generation(args["generation"].as_i64())?;
    match name {
        "reconstruction_status" => database.status(generation),
        "reconstruction_report" => database.report(generation),
        "reconstruction_files" => {
            database.files(generation, args["offset"].as_u64().unwrap_or_default())
        }
        "reconstruction_units" => database.units(
            generation,
            args["state"].as_str().unwrap_or_default(),
            args["offset"].as_u64().unwrap_or_default(),
        ),
        "reconstruction_claim" => database.claim(
            generation,
            args["worker"].as_str().ok_or("Supply a worker name")?,
        ),
        "reconstruction_dependencies" => database.set_dependencies(generation, args),
        "reconstruction_modules" => {
            database.modules(generation, args["offset"].as_u64().unwrap_or_default())
        }
        "reconstruction_contracts" => database.set_contracts(generation, args),
        "reconstruction_submit" => database.submit(generation, &args["submission"]),
        "reconstruction_read_file" => database.read_file(
            generation,
            args["path"].as_str().ok_or("Supply a source path")?,
        ),
        "reconstruction_control" => {
            database.control(
                generation,
                args["action"].as_str().ok_or("Supply an action")?,
                args["lease"].as_str(),
                args["address"].as_str(),
            )?;
            database.status(generation)
        }
        _ => Err(crate::error::failure("Unknown reconstruction command")),
    }
}

pub(crate) fn root(project: &str, args: &Value) -> Result<PathBuf> {
    if let Some(root) = args["root"].as_str() {
        return Ok(root.into());
    }
    let file = std::fs::File::open(pointer(project)?)?;
    let root: PathBuf = serde_json::from_reader(std::io::BufReader::new(file))?;
    Ok(root)
}

fn pointer(project: &str) -> Result<PathBuf> {
    if project.len() != 64 || !project.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(crate::error::failure("A valid binary identity is required"));
    }
    Ok(crate::settings::support_dir()
        .join("reconstruction")
        .join(format!("{project}.json")))
}

fn remember(project: &str, root: &Path) -> Result<()> {
    crate::storage::atomic::write(&pointer(project)?, |file| {
        serde_json::to_writer(file, &json!(root))?;
        Ok(())
    })
}
