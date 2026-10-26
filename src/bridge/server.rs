use crate::error::{Result, failure};
use crate::events::Sink;
use crate::worker::RuntimeSnapshot;
use crate::worker::WorkerPort;
use prism::mcp::{self, Mode};
use serde_json::{Value, json};
use std::net::{SocketAddr, TcpListener};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub(crate) struct Bridge {
    address: SocketAddr,
    token: String,
    stopped: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<Result<()>>>,
}

impl Bridge {
    pub fn start(worker: WorkerPort, sink: Sink, runtime: RuntimeSnapshot) -> Result<Self> {
        Self::start_with_mode(worker, sink, runtime, mcp::Mode::Combined, true)
    }

    pub fn start_headless(
        worker: WorkerPort,
        sink: Sink,
        runtime: RuntimeSnapshot,
    ) -> Result<Self> {
        Self::start_with_mode(worker, sink, runtime, mcp::Mode::Headless, false)
    }

    fn start_with_mode(
        worker: WorkerPort,
        sink: Sink,
        runtime: RuntimeSnapshot,
        mode: Mode,
        persist_endpoint: bool,
    ) -> Result<Self> {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
        let address = listener.local_addr()?;
        listener.set_nonblocking(true)?;
        let mut random = [0u8; 32];
        getrandom::fill(&mut random).map_err(|error| failure(error.to_string()))?;
        let token: String = random.iter().map(|byte| format!("{byte:02x}")).collect();
        if persist_endpoint {
            persist_endpoint_file(address, &token)?;
        }
        let stopped = Arc::new(AtomicBool::new(false));
        let flag = stopped.clone();
        let thread_token = token.clone();
        let launched = std::thread::Builder::new()
            .name("prism-window-bridge".into())
            .spawn(move || {
                super::listener::run(listener, thread_token, worker, sink, flag, mode, runtime);
                if persist_endpoint {
                    remove_endpoint(address)?;
                }
                Ok(())
            });
        let thread = match launched {
            Ok(thread) => thread,
            Err(error) => {
                let cleanup = if persist_endpoint {
                    remove_endpoint(address)
                } else {
                    Ok(())
                };
                return crate::error::finish(
                    Err(error.into()),
                    cleanup,
                    "Removing the window address",
                );
            }
        };
        Ok(Self {
            address,
            token,
            stopped,
            thread: Some(thread),
        })
    }

    pub fn close(mut self) -> Result<()> {
        self.stopped.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            thread
                .join()
                .map_err(|_| failure("The window bridge stopped unexpectedly"))??;
        }
        Ok(())
    }

    pub fn endpoint(&self) -> (String, String) {
        (self.address.to_string(), self.token.clone())
    }

    pub fn configurations(
        &self,
        window: WindowTools,
        analysis: AnalysisTools,
        chat: u64,
    ) -> Result<Vec<Value>> {
        let window = window == WindowTools::Enabled;
        let analysis = analysis == AnalysisTools::Enabled;

        let base = mcp_configuration(std::env::current_exe()?);
        let mut servers = Vec::new();
        for (enabled, mode) in [(window, Mode::Window), (analysis, Mode::Analysis)] {
            if enabled {
                let mut configuration = base.clone();
                configuration["name"] = json!(mode.name());
                configuration["env"]
                    .as_array_mut()
                    .ok_or("Invalid MCP environment")?
                    .push(json!({"name":"PRISM_CHAT_ID","value":chat.to_string()}));
                if let Some(argument) = mode.argument() {
                    configuration["args"]
                        .as_array_mut()
                        .ok_or("Invalid MCP configuration")?
                        .push(json!(argument));
                }
                servers.push(configuration);
            }
        }
        Ok(servers)
    }
}

fn mcp_configuration(executable: std::path::PathBuf) -> Value {
    mcp::configuration(&executable, Mode::Combined)
}

impl Drop for Bridge {
    fn drop(&mut self) {
        // The listener polls this flag; its owning thread removes the endpoint
        // after serving requests. Joining belongs to the explicit close path.
        self.stopped.store(true, Ordering::Relaxed);
    }
}

fn persist_endpoint_file(address: SocketAddr, token: &str) -> Result<()> {
    crate::storage::atomic::write(
        &crate::settings::support_dir().join("window.json"),
        |file| {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
            }
            serde_json::to_writer(
                file,
                &json!({"address":address.to_string(),"token":token,"pid":std::process::id()}),
            )?;
            Ok(())
        },
    )
}

fn remove_endpoint(address: SocketAddr) -> Result<()> {
    let path = crate::settings::support_dir().join("window.json");
    let file = match std::fs::File::open(&path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    let endpoint: Value = serde_json::from_reader(file)?;
    if endpoint["address"].as_str() == Some(&address.to_string()) {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::mcp_configuration;
    use serde_json::json;

    #[test]
    fn bridge_secret_is_not_passed_on_the_helper_command_line() {
        let configuration = mcp_configuration("/tmp/prism".into());

        assert_eq!(configuration["args"], json!(["mcp"]));
        assert_eq!(configuration["env"], json!([]));
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WindowTools {
    Enabled,
    Disabled,
}

impl From<bool> for WindowTools {
    fn from(value: bool) -> Self {
        if value { Self::Enabled } else { Self::Disabled }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AnalysisTools {
    Enabled,
    Disabled,
}

impl From<bool> for AnalysisTools {
    fn from(value: bool) -> Self {
        if value { Self::Enabled } else { Self::Disabled }
    }
}
