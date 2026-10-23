use crate::error::{Result, failure};
use serde_json::{Value, json};

pub(crate) fn custom_configuration(command: &str) -> Result<Option<Value>> {
    let command = command.trim();
    if command.is_empty() {
        return Ok(None);
    }
    if command.len() > 8192 || command.contains('\0') {
        return Err(failure(
            "The tool server command is too long or contains a null byte",
        ));
    }

    let arguments =
        shlex::split(command).ok_or("The tool server command contains unclosed quotes")?;
    let (program, arguments) = arguments
        .split_first()
        .ok_or("Enter a tool server command")?;
    if program.is_empty() || arguments.len() > 256 {
        return Err(failure(
            "The tool server needs a program and at most 256 arguments",
        ));
    }
    let program = crate::executable_paths::find(program);

    Ok(Some(json!({
        "name":"custom", "command":program, "args":arguments, "env":[],
    })))
}

pub(crate) fn with_custom(mut servers: Vec<Value>, command: &str) -> Result<Vec<Value>> {
    if let Some(server) = custom_configuration(command)? {
        servers.push(server);
    }
    Ok(servers)
}
