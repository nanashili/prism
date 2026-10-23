use super::{Mode, endpoint::Endpoint, session::Session};
use crate::error::{Result, failure};
use std::ffi::OsString;

pub fn run_cli(mut arguments: impl Iterator<Item = OsString>) -> Result<()> {
    let mut mode = Mode::Combined;
    let mut endpoint = None;

    while let Some(argument) = arguments.next() {
        match argument.to_str() {
            Some("--window") => mode = Mode::Window,
            Some("--analysis") => mode = Mode::Analysis,
            Some("--headless") => mode = Mode::Headless,
            Some("--help" | "-h") => {
                println!(
                    "Prism MCP\nUsage: prism-mcp [--window | --analysis]\n\nRun with Prism open. Headless mode is managed by the Prism executable."
                );
                return Ok(());
            }
            Some(address) if !address.starts_with('-') && endpoint.is_none() => {
                let token = arguments.next().ok_or("Missing bridge token")?;
                endpoint = Some(Endpoint::new(
                    address,
                    token.to_str().ok_or("Invalid bridge token")?,
                )?);
            }
            _ => {
                return Err(failure(
                    "Usage: prism-mcp [--window | --analysis | --headless]",
                ));
            }
        }
    }

    if mode == Mode::Headless && endpoint.is_none() {
        return Err(failure(
            "Headless MCP requires a Prism-managed bridge endpoint",
        ));
    }

    Session::new(mode, endpoint).run(&mut std::io::stdin().lock(), std::io::stdout())
}
