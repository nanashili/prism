use crate::error::{Result, failure};
use crate::events::Sink;
use crate::worker::RuntimeSnapshot;
use crate::worker::WorkerPort;
use prism::mcp::{self, Mode};
use serde_json::{Value, json};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub(crate) struct Bridge {
    address: SocketAddr,
    token: String,
    persist_endpoint: bool,
    stopped: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
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
        let mut random = [0u8; 32];
        getrandom::fill(&mut random).map_err(|error| failure(error.to_string()))?;
        let token: String = random.iter().map(|byte| format!("{byte:02x}")).collect();
        if persist_endpoint {
            persist_endpoint_file(address, &token)?;
        }
        let stopped = Arc::new(AtomicBool::new(false));
        let flag = stopped.clone();
        let thread_token = token.clone();
        let thread = std::thread::spawn(move || {
            super::listener::run(listener, thread_token, worker, sink, flag, mode, runtime);
        });
        Ok(Self {
            address,
            token,
            persist_endpoint,
            stopped,
            thread: Some(thread),
        })
    }

    pub fn endpoint(&self) -> (String, String) {
        (self.address.to_string(), self.token.clone())
    }

    pub fn configurations(&self, window: bool, analysis: bool, chat: u64) -> Result<Vec<Value>> {
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
        self.stopped.store(true, Ordering::Relaxed);
        if let Err(error) = TcpStream::connect_timeout(&self.address, Duration::from_secs(1)) {
            eprintln!("Waking the window bridge failed: {error}");
        }
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            eprintln!("The window bridge stopped unexpectedly");
        }
        if self.persist_endpoint
            && let Err(error) = remove_endpoint(self.address)
        {
            eprintln!("Removing the window address failed: {error}");
        }
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
    let endpoint: Value = serde_json::from_reader(std::io::BufReader::new(file))?;
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
