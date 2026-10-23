use crate::error::{Result, failure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::Write;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct BuildCommand {
    program: String,
    args: Vec<String>,
}

pub(super) fn run(
    root: &Path,
    language: &str,
    custom: Option<&Value>,
    cancelled: &AtomicBool,
) -> Result<String> {
    let commands = commands(language, custom)?;
    let mut output = String::new();
    for command in commands {
        command.validate()?;
        writeln!(output, "{} {:?}", command.program, command.args)?;
        let result = crate::protocol::process::capture(
            Command::new(&command.program)
                .args(&command.args)
                .current_dir(root),
            cancelled,
            Duration::from_secs(600),
        );
        match result {
            Ok(text) => output.push_str(&text),
            Err(error) => return Err(failure(format!("{output}\n{error}"))),
        }
    }
    Ok(output)
}

pub(super) fn identity(language: &str, custom: Option<&Value>) -> Result<String> {
    Ok(serde_json::to_string(&commands(language, custom)?)?)
}

fn commands(language: &str, custom: Option<&Value>) -> Result<Vec<BuildCommand>> {
    let commands = match custom {
        Some(value) => vec![serde_json::from_value::<BuildCommand>(value.clone())?],
        None => defaults(language)?,
    };
    for command in &commands {
        command.validate()?;
    }
    Ok(commands)
}

impl BuildCommand {
    fn validate(&self) -> Result<()> {
        if self.program.is_empty()
            || self.program.len() > 4096
            || self.program.contains('\0')
            || self.args.len() > 128
            || self
                .args
                .iter()
                .any(|arg| arg.len() > 4096 || arg.contains('\0'))
        {
            return Err(failure(
                "Supply a program and at most 128 arguments of up to 4096 bytes each",
            ));
        }
        Ok(())
    }
}

fn defaults(language: &str) -> Result<Vec<BuildCommand>> {
    let commands: Vec<(&str, Vec<&str>)> = match language.to_ascii_lowercase().as_str() {
        "rust" => vec![("cargo", vec!["check", "--jobs", "1"])],
        "c" | "c++" => vec![
            ("cmake", vec!["-S", ".", "-B", "build"]),
            ("cmake", vec!["--build", "build", "--parallel", "1"]),
        ],
        "c#" => vec![("dotnet", vec!["build", "--maxcpucount:1"])],
        "swift" => vec![("swift", vec!["build", "--jobs", "1"])],
        "go" => vec![("go", vec!["build", "-p", "1", "./..."])],
        "typescript" => vec![("tsc", vec!["--noEmit"])],
        "java" => vec![("mvn", vec!["--batch-mode", "-DskipTests", "package"])],
        "zig" => vec![("zig", vec!["build", "-j1"])],
        "python" => vec![("python3", vec!["-m", "compileall", "-q", "."])],
        _ => {
            return Err(failure(
                "This language needs an explicit build command. Export source or supply command: {program, args}",
            ));
        }
    };
    Ok(commands
        .into_iter()
        .map(|(program, args)| BuildCommand {
            program: program.into(),
            args: args.into_iter().map(str::to_owned).collect(),
        })
        .collect())
}
