fn main() {
    if let Err(error) = prism::mcp::run_cli(std::env::args_os().skip(1)) {
        eprintln!("Prism MCP: {error}");
        std::process::exit(1);
    }
}
