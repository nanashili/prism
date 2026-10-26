//! Standard-input entry point for the Prism MCP helper.

fn main() {
    if let Err(error) = run() {
        eprintln!("Prism MCP: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), prism::Error> {
    let _profiling = prism::profiling::start_mcp();
    prism::mcp::run_cli(std::env::args_os().skip(1))
}
