use crate::error::{Result, failure};
use crate::protocol::{Framing, Rpc};
use crate::settings::Settings;
use serde_json::{Value, json};
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, atomic::AtomicBool};
use std::time::Duration;

pub(super) struct Rizin {
    rpc: Rpc,
    timeout: Duration,
    analyzed: bool,
}

impl Rizin {
    pub fn close(mut self) -> Result<()> {
        self.rpc.close()
    }

    pub fn is_reusable(&self) -> bool {
        self.rpc.is_reusable()
    }

    #[cfg_attr(feature = "profiling", hotpath::measure)]
    pub fn open(settings: &Settings, path: &Path, cancelled: Arc<AtomicBool>) -> Result<Self> {
        let mut command = Command::new(&settings.rizin);
        command
            .args([
                "-q0",
                "-N",
                "-e",
                "scr.color=0",
                "-e",
                "scr.utf8=false",
                "-e",
                "io.cache=true",
            ])
            .arg(path);
        let mut rpc = Rpc::spawn(&mut command, Framing::Zero, cancelled)?;
        rpc.limit_memory(u64::from(settings.analysis_memory_mib) * 1024 * 1024);
        rpc.exchange(None, Duration::from_secs(30))?;
        Ok(Self {
            rpc,
            timeout: Duration::from_secs(settings.timeout_seconds),
            analyzed: false,
        })
    }

    #[cfg_attr(feature = "profiling", hotpath::measure)]
    pub(super) fn command(&mut self, command: &str) -> Result<Value> {
        self.rpc.exchange(Some(command), self.timeout)
    }

    #[cfg_attr(feature = "profiling", hotpath::measure)]
    pub(super) fn analyze(&mut self) -> Result<()> {
        if !self.analyzed {
            self.command("aa")?;
            self.analyzed = true;
        }
        Ok(())
    }

    pub fn tool(&mut self, name: &str, args: Value) -> Result<Value> {
        self.analyze()?;
        let value = match name {
            "list_procedures" => self.procedures()?,
            "list_segments" => self.command("iSj")?,
            "list_strings" => self.command("izj")?,
            "procedure_assembly" => {
                let address = address(&args)?;
                let function = self.command(&format!("pdfj @ {address}"))?;
                Value::String(super::rizin_tools::assembly(&function)?)
            }
            "procedure_pseudo_code" => {
                let address = address(&args)?;
                let plugins = self.command("Lc")?;
                let plugins = plugins.as_str().unwrap_or_default();
                let command = if plugins.contains("pdg") {
                    "pdg"
                } else if plugins.contains("pdc") {
                    "pdc"
                } else {
                    return Err(failure(
                        "Rizin needs rz-ghidra or jsdec for pseudocode. Use Assembly or install a decompiler plugin.",
                    ));
                };
                let text = self.command(&format!("{command} @ {address}"))?;
                if text.as_str().is_none_or(|text| text.trim().is_empty()) {
                    return Err(failure("The decompiler did not return pseudocode"));
                }
                text
            }
            "analyze_function" => {
                let address = address(&args)?;
                let blocks = self.command(&format!("afbj @ {address}"))?;
                let function = self.command(&format!("pdfj @ {address}"))?;
                let callers = self.command(&format!("axtj @ {address}"))?;
                return Ok(
                    json!({"raw_result":{"value":super::rizin_tools::dossier(&blocks,&function,&callers)?}}),
                );
            }
            "procedure_callers" => self.command(&format!("axtj @ {}", address(&args)?))?,
            "procedure_callees" => self.command(&format!("axffj @ {}", address(&args)?))?,
            _ => {
                return Err(failure(format!(
                    "The Rizin adapter does not expose {name}. Use analysis_tools for supported operations."
                )));
            }
        };
        Ok(
            json!({"normalized_result":value,"provider":"rizin","coverage":"initial analysis; unresolved indirect calls may remain"}),
        )
    }

    fn procedures(&mut self) -> Result<Value> {
        let reply = self.command("afl")?;
        let text = reply
            .as_str()
            .ok_or("Rizin did not return a function inventory")?;
        let rows = text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .take(300)
            .map(super::rizin_inventory::parse)
            .collect::<Result<Vec<_>>>()?;
        let rows: Vec<_> = rows
            .into_iter()
            .map(|row| json!({"address":row.address,"value":row.name}))
            .collect();
        Ok(
            json!({"items":rows,"limit":300,"complete":false,"note":"Use Prism list_functions for the complete paged index"}),
        )
    }
}

fn address(args: &Value) -> Result<String> {
    let text = args["procedure"]
        .as_str()
        .ok_or("A function address is required")?;
    let address = super::graph::address_value(text)
        .ok_or("Function addresses must contain only hexadecimal digits")?;
    Ok(format!("0x{address:x}"))
}
